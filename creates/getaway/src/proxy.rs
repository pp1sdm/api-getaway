use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method as HttpMethod, StatusCode},
    response::{IntoResponse, Response},
};

use super::router::{Method, Route};
use super::state::AppState;

// ============ 代理入口 ============

// 代理函数
pub async fn proxy(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request, // 必须最后：它会吃掉整个请求
) -> Response {
    // 方法转换：http::Method -> router::Method
    let Some(method) = to_router_method(request.method()) else {
        return (StatusCode::NOT_IMPLEMENTED, "unsupported method").into_response();
    };

    // 查路由。参数顺序是 (method, path)
    let path = request.uri().path();
    let Some(matched) = state.routes.match_route(method, path) else {
        // 查不到 = 网关的 404，不甩锅给上游
        return (StatusCode::NOT_FOUND, "no route matched").into_response();
    };

    // 转发
    match forward(&state, &matched.route, peer, request).await {
        Ok(resp) => resp,
        Err(e) => proxy_error(e),
    }
}

// ============ 方法转换 ============

// 注意：这里不能用 match！http::Method 不是可结构化匹配的类型，
// 写 match 会报 "constant of non-structural type in a pattern"，只能 if 链。
fn to_router_method(m: &HttpMethod) -> Option<Method> {
    if m == HttpMethod::GET {
        Some(Method::GET)
    } else if m == HttpMethod::POST {
        Some(Method::POST)
    } else if m == HttpMethod::PUT {
        Some(Method::PUT)
    } else if m == HttpMethod::DELETE {
        Some(Method::DELETE)
    } else if m == HttpMethod::PATCH {
        Some(Method::PATCH)
    } else {
        None
    }
}

// ============ 转发 ============

// 转发
async fn forward(
    state: &AppState,
    route: &Route,
    peer: SocketAddr,
    request: Request,
) -> Result<Response, reqwest::Error> {
    let method = request.method().clone();

    // ① 保留 path + query。只取 path() 会把 ?page=2 静默丢掉
    let path_and_query = request
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/")
        .to_string();

    // ② 拼上游 URL。route.upstream = "http://127.0.0.1:3001"
    let url = format!("{}{}", route.upstream.trim_end_matches('/'), path_and_query);

    // ③ 下面两个判断必须在"把 headers 搬走"之前做完
    let orig_host = request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    // GET 硬挂空流会让上游收到 chunked 编码，部分上游会报错
    let has_body = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .map(|v| v != "0")
        .unwrap_or(false)
        || request.headers().contains_key(header::TRANSFER_ENCODING);

    // 直接把 HeaderMap 从请求里搬出来（mem::take），省掉一次每请求的整表克隆
    let mut headers = std::mem::take(request.headers_mut());
    strip_hop_by_hop(&mut headers);
    set_forwarded(&mut headers, peer, orig_host.as_deref());
    headers.remove(header::HOST); // 交给 reqwest 按 URL 生成（≈ nginx $proxy_host）

    // ④ 构造请求
    let mut builder = state.client.request(method, url).headers(headers);

    if has_body {
        // ⑤ 流式，绝不 collect —— collect 一个上传大文件就爆内存
        // 不能用 Body::wrap(axum::Body)：wrap 要求 B: Sync，
        // 而 axum 的 Body 是 UnsyncBoxBody，不是 Sync。
        // wrap_stream 的约束是 S: TryStream + Send（没有 Sync），正好合适。
        // 注意：wrap_stream 需要 reqwest 开 stream feature。
        builder = builder.body(reqwest::Body::wrap_stream(
            request.into_body().into_data_stream(),
        ));
    }

    // ⑥ 发请求（state.client 终于用上了）
    let upstream_resp = builder.send().await?;

    // ⑦ 回传状态和头
    let status = upstream_resp.status();
    // 这里是 clone 而不是 mem::take，因为紧接着 bytes_stream() 会把 response 吃掉。
    // reqwest::Response 也有 headers_mut()，想省这一次克隆可以改成 take。
    let mut resp_headers = upstream_resp.headers().clone();
    strip_hop_by_hop(&mut resp_headers);

    // ⑧ body 也流式回传
    let body = Body::from_stream(upstream_resp.bytes_stream());

    // 整块 HeaderMap 直接搬进响应，不逐条 insert：
    // 这样多个 Set-Cookie 天然全部保留，也不会漏掉任何重复的 header。
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;

    Ok(response)
}

// ============ 逐跳头 ============

// RFC 9110 §7.6.1。不剥的话，把 Transfer-Encoding 透传给上游，
// 而上游收到的是 hyper 已解码的明文流，解析会直接错乱。
const HOP_BY_HOP: [&str; 8] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

fn strip_hop_by_hop(headers: &mut HeaderMap) {
    // 第一步：Connection 头里"点名"的字段也是逐跳头，必须先收集
    let mut named: Vec<HeaderName> = Vec::new();
    for value in headers.get_all(header::CONNECTION).iter() {
        if let Ok(s) = value.to_str() {
            for name in s.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                if let Ok(h) = HeaderName::from_bytes(name.as_bytes()) {
                    named.push(h);
                }
            }
        }
    }

    // 第二步：固定名单 + 被点名的
    for name in HOP_BY_HOP {
        headers.remove(name);
    }
    for name in named {
        headers.remove(&name);
    }
}

// ============ X-Forwarded-* ============

const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
const X_FORWARDED_PROTO: HeaderName = HeaderName::from_static("x-forwarded-proto");
const X_FORWARDED_HOST: HeaderName = HeaderName::from_static("x-forwarded-host");
const X_REAL_IP: HeaderName = HeaderName::from_static("x-real-ip");

fn set_forwarded(headers: &mut HeaderMap, peer: SocketAddr, orig_host: Option<&str>) {
    let ip = peer.ip().to_string();

    // 追加，不是覆盖 —— 保留整条代理链路
    let value = match headers.get(&X_FORWARDED_FOR).and_then(|v| v.to_str().ok()) {
        Some(prev) if !prev.is_empty() => format!("{prev}, {ip}"),
        _ => ip.clone(),
    };

    if let Ok(v) = HeaderValue::from_str(&value) {
        headers.insert(X_FORWARDED_FOR, v);
    }
    if let Ok(v) = HeaderValue::from_str(&ip) {
        headers.insert(X_REAL_IP, v);
    }
    // from_static 不可失败：它返回 HeaderValue 而不是 Result（非法值直接 panic），
    // 别跟上一行的 from_str 一样套 if let Ok
    headers.insert(X_FORWARDED_PROTO, HeaderValue::from_static("http"));
    if let Some(h) = orig_host {
        if let Ok(v) = HeaderValue::from_str(h) {
            headers.insert(X_FORWARDED_HOST, v);
        }
    }
}

// ============ 错误映射 ============

fn proxy_error(e: reqwest::Error) -> Response {
    // 生产环境这里必须 tracing::error!(?e)，否则线上 502 无从排查
    if e.is_timeout() {
        (StatusCode::GATEWAY_TIMEOUT, "upstream timeout").into_response()
    } else if e.is_connect() {
        (StatusCode::BAD_GATEWAY, "upstream connect failed").into_response()
    } else {
        (StatusCode::BAD_GATEWAY, "upstream error").into_response()
    }
}
