use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async  fn main() {
    // 创建服务器路由表
    let app: Router = Router::new()
        .route("/", get(|| async { "这里是根目录路径" }))
        .route("/api/user", get(|| async { "这里是用户得api" }))
        .route("health", get(|| async { "这里是健康路由" }))
        .fallback("这里是保底路由");

    // 绑定Tokio的监听
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001")
        .await
        .unwrap();

    // 流程打印
    println!("这里是后端项目");

    // 服务器服务
    axum::serve(listener, app)
        .await
        .unwrap();
}
