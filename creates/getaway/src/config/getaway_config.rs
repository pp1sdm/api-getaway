use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: Server,
    pub upstream: Upstream,
    pub routes: Vec<Route>,
    pub fallback: Fallback,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Upstream {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub path: String,
    pub method: String,
}

#[derive(Debug, Deserialize)]
pub struct Fallback {
    pub enabled: bool,
}

impl Config {
    pub fn load() -> Self {
        let content = std::fs::read_to_string("config/getaway_config.toml")
            .expect("读取配置文件失败");

        toml::from_str(&content)
            .expect("解析 TOML 配置失败")
    }
}