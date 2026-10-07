use std::net::SocketAddr;

use axum::http::{header, HeaderMap, HeaderName, HeaderValue};

// 需要处理的逐跳请求头
const HOP_BY_HOP: [HeaderName; 8] = [
    header::CONNECTION,
    header::PROXY_AUTHENTICATE,
    header::PROXY_AUTHORIZATION,
    header::TE,
    header::TRAILER,
    header::TRANSFER_ENCODING,
    header::UPGRADE,
    HeaderName::from_static("keep-alive"),
];

// 自定义分析请求头
const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
const X_FORWARDED_PROTO: HeaderName = HeaderName::from_static("x-forwarded-proto");
const X_FORWARDED_HOST: HeaderName = HeaderName::from_static("x-forwarded-host");
const X_REAL_IP: HeaderName = HeaderName::from_static("x-real-ip");

/// 剥离逐跳首部。请求方向、响应方向都要调，规则完全一样。
pub fn strip_hop_by_hop(headers: &mut HeaderMap) {
    // ① 先收集 Connection 里"点名"的首部 —— 它们同样逐跳。
    //    `Connection: keep-alive, X-Trace` 意味着 X-Trace 也不许往下传。
    //    必须先收集再删：迭代器借着 headers，边遍历边 remove 编不过。
    let named: Vec<HeaderName> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|v| HeaderName::from_bytes(v.trim().as_bytes()).ok())
        .collect();

    // ② 固定名单 + 被点名的，一次删完
    for name in HOP_BY_HOP.iter().chain(named.iter()) {
        headers.remove(name);
    }
}

/// 写入 X-Forwarded-*。
///
/// 策略：**丢弃客户端自带的 X-Forwarded-For，用 peer IP 重建。**
/// 因为我们前面没有可信代理，客户端写什么都不可信 ——
/// 无条件追加等于让任何人伪造来源 IP。
/// 将来前面挂了 LB，再按可信 CIDR 决定是"追加"还是"重建"。
pub fn set_forwarded(
    headers: &mut HeaderMap,
    peer: SocketAddr,
    scheme: &str,
    orig_host: Option<&str>,
) {
    let ip = peer.ip().to_string();

    // 先清干净，再写自己的 —— 不要"有就追加"
    headers.remove(&X_FORWARDED_FOR);
    headers.remove(&X_REAL_IP);
    headers.remove(&X_FORWARDED_HOST);

    // IP 的 Display 只输出 ASCII 字母数字和 . : %，构造不可能失败；
    // 但依然不 unwrap —— 请求路径上不留 panic 点。
    if let Ok(v) = HeaderValue::from_str(&ip) {
        headers.insert(X_FORWARDED_FOR, v.clone());
        headers.insert(X_REAL_IP, v);
    }

    // scheme 由调用方传入，不在这里硬编码 "http"。
    // 将来网关上了 TLS，只改调用点，这个函数不动。
    if let Ok(v) = HeaderValue::from_str(scheme) {
        headers.insert(X_FORWARDED_PROTO, v);
    }

    if let Some(host) = orig_host {
        if let Ok(v) = HeaderValue::from_str(host) {
            headers.insert(X_FORWARDED_HOST, v);
        }
    }
}