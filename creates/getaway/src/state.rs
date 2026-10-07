use std::sync::Arc;
use reqwest::Client;

use super::config::getaway_config::GatewayConfig;
use super::router::RouteTable;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,

    pub routes: Arc<RouteTable>,

    pub config: Arc<GatewayConfig>,
}

// 状态实现
impl AppState {
    pub fn new(client: Client, router: RouteTable, config: GatewayConfig) -> Self {
        Self {
            client,
            routes: Arc::new(router),
            config: Arc::new(config),
        }
    }
}