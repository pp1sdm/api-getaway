use axum::{routing::get, Router};

use super::proxy::proxy;
use super::state::AppState;

// 安全函数
async fn healthz() -> &'static str {
    "ok"
}

// 组装网关
pub fn build_app(state: AppState) -> Router {
    Router::new()
    .route("/healthz", get(healthz))

    // 其余走代理
    .fallback(proxy)

    // 挂载状态
    .with_state(state)
}