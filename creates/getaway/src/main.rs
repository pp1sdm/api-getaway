use getaway::{build_app, AppState, GatewayConfig, RouteTable};
use reqwest::Client;
use tokio::net::TcpListener;
use axum::serve;

#[tokio::main]
async  fn main() {
    // 拿到getaway配置
    let getaway_config = GatewayConfig::load();

    // 通过配置拿到生成路由表
    let router_table = RouteTable::add_route_table(&getaway_config);

    // 创建客户端
    let client = Client::builder()
        .tcp_nodelay(true)
        .pool_max_idle_per_host(32)
        .build()
        .expect("网络客户端构建失败");

    // 组装
    let state = AppState::new(client, router_table, getaway_config);
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let app = build_app(state);

    // 监听
    let listener = TcpListener::bind(&addr).await.unwrap();

    // 网关正在监听
    println!("网关监听{addr}");

    // 启动服务
    serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await.unwrap()
}
