pub mod app;
pub mod config;
pub mod proxy;
pub mod router;
pub mod state;

// 常用模块导出
pub use app::build_app;
pub use config::getaway_config::GatewayConfig;
pub use router::{Route, RouteTable};
pub use state::AppState;