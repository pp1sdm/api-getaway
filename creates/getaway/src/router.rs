use std::collections::HashMap;
use axum::http::method::Method;
use super::config::getaway_config::Config;

// 路由表
pub struct RouteTable {
    pub root: TrieNode,
    pub fallback: Option<Route>,
}

pub struct TrieNode {
    pub children: HashMap<String, TrieNode>,
    pub route: Option<Route>,
}

pub struct Route {
    pub path: String,
    pub upstream: String,
}

// 路由表实现
impl RouteTable {
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            fallback: None,
        }
    }

    pub fn insert(&mut self, route: Route) {
        let mut current = &mut self.root;

        for segment in route.path.trim_matches('/').split('/') {
            current = current
                .children
                .entry(segment.to_string())
                .or_insert_with(TrieNode::new);
        }

        current.route = Some(route);
    }

    pub fn set_fallback(&mut self, route: Route) {
        self.fallback = Some(route);
    }

    pub fn from_config(config: &Config) -> Self {
        let mut table = RouteTable::new();

        for route in &config.routes {
            let route = Route {
                path: route.path.clone(),
                upstream: config.upstream.address.clone(),
            };

            table.insert(route);
        }

        if config.fallback.enabled {
            let route = Route {
                path: "*".to_string(),
                upstream: config.upstream.address.clone(),
            };

            table.set_fallback(route);
        }

        table
    }
}

// 树节点实现
impl TrieNode {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            route: None,
        }
    }
}
