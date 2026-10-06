mod router;
mod config;

use axum::{
    routing::get,
    Router,
    body::Body,
    http::Request
};
use router::*;
use config::getaway_config::*;

#[tokio::main]
async  fn main() {
    // 拿到getaway配置
    let getaway_config = Config::load();

    // 通过配置拿到生成路由表
    let router_table = RouteTable::from_config(&getaway_config);

    async fn proxy(request: Request<Body>) {
        // 拿到路径
        let path = request.uri().path();

        // 匹配路由

    }

    // 捕获前端请求
    let app = Router::new().fallback(proxy);

    // 绑定地址
    let addr = format!("{}:{}", getaway_config.server.host, getaway_config.server.port);

    // 绑定Tokio的监听
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();

    // 服务器服务
    axum::serve(listener, app)
        .await
        .unwrap();
}
