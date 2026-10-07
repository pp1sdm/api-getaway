use serde::Deserialize;
use crate::router::Method;

#[derive(Debug, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub upstream: UpstreamConfig,
    pub routes: Vec<RouteConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct RouteConfig {
    pub path: String,
    pub method: Method,
}

impl GatewayConfig {
    pub fn load() -> Self {
        let content = std::fs::read_to_string("config/getaway_config.toml")
            .expect("读取配置文件失败");

        toml::from_str(&content)
            .expect("解析 TOML 配置失败")
    }
}