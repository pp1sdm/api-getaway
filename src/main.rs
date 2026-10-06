use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async  fn main() {
    // 创建服务器路由表
    let app: Router = Router::new().route("/", get(|| async { "Hello, world!" }));

    // 绑定Tokio的监听
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
    .await
    .unwrap();

    // 流程打印
    println!("即将链接服务器");

    // 服务器服务
    axum::serve(listener, app)
    .await
    .unwrap();
}
