use std::collections::HashMap;
use serde::Deserialize;
use crate::config::getaway_config;
use super::config::getaway_config::GatewayConfig;

// http请求方式枚举
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

// 路由
#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub method: Method,
    pub upstream: String,
}

// 匹配结果
#[derive(Debug)]
pub struct MatchResult {
    pub route: Route,
    pub params: HashMap<String, String>,
}

// 参数节点
struct ParamNode {
    name: String,
    node: Box<TrieNode>,
}

// 通配符节点
struct WildcardNode {
    name: String,
    routes: HashMap<Method, Route>,
}

// 节点
struct TrieNode {
    children: HashMap<String, TrieNode>,
    param: Option<ParamNode>,
    wildcard: Option<WildcardNode>,
    routes: HashMap<Method, Route>,
}

// 路由表
pub struct RouteTable {
    pub root: TrieNode,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            param: None,
            wildcard: None,
            routes: HashMap::new(),
        }
    }
}

impl RouteTable {
    fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    // 拆分路径
    fn split_path(path: &str) -> Vec<&str> {

        path
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect()
    }

    fn add_route(
        &mut self,
        method: Method,
        path: &str,
        upstream: &str,
    ) {
        let segments = Self::split_path(path);

        let route = Route {
            path: path.to_string(),
            method,
            upstream: upstream.to_string(),
        };

        let mut current = &mut self.root;

        for (index, segment) in segments.iter().enumerate() {
            // 参数路由补充
            if let Some(name) = segment.strip_prefix(':') {
                if name.is_empty() {
                    panic!("路由地址有问题");
                }

                let param = current
                    .param
                    .get_or_insert_with(|| ParamNode {
                        name: name.to_string(),
                        node: Box::new(TrieNode::new()),
                    });

                if param.name != name {
                    panic!(
                        "参数名字冲突: :{} vs :{}",
                        param.name,
                        name
                    );
                }

                current = param.node.as_mut();
            }

            // 通配符路由补充
            else if let Some(name) = segment.strip_prefix('*') {
                if name.is_empty() {
                    panic!("通配符名字不能是空的");
                }
                // Wildcard 必须是最后一个 segment
                if index != segments.len() - 1 {
                    panic!("通配符部分必须是最后一个");
                }

                if current.wildcard.is_some() {
                    panic!(
                        "wildcard route already exists: {}",
                        path
                    );
                }

                current.wildcard = Some(
                    WildcardNode {
                        name: name.to_string(),
                        routes: HashMap::new(),
                    }
                );

                current
                    .wildcard
                    .as_mut()
                    .unwrap()
                    .routes
                    .insert(method, route);


                return;
            }

            // 精确路由补充
            else {
                current = current
                    .children
                    .entry(segment.to_string())
                    .or_insert_with(TrieNode::new);
            }
        }

        if current.routes.contains_key(&method) {
            panic!(
                "当前路径的当前方法已经注册: {:?} {}",
                method,
                path
            );
        }

        // 将路由节点，给到最终匹配的路径
        current.routes.insert(method, route);
    }

    // 根据配置构建路由表
    pub fn add_route_table(config: &GatewayConfig) -> Self {
        // 初始化
        let mut route_table = RouteTable::new();

        // 独立配置
        let upstream = config.upstream.address.clone();

        // 构建
        for getaway_config::RouteConfig {path, method} in config.routes {
            route_table.add_route(method, path.as_str(), upstream.as_str())
        }

        route_table
    }

    // 匹配节点
    fn match_node(
        node: &TrieNode,
        segments: &[&str],
        index: usize,
        method: Method,
        params: &mut HashMap<String, String>,
    ) -> Option<MatchResult> {
        // 路径已经全部匹配完
        if index == segments.len() {
            if let Some(route) = node.routes.get(&method) {
                return Some(MatchResult {
                    route: route.clone(),
                    params: params.clone(),
                });
            }

            return None;
        }

        let segment = segments[index];

        // 1. 精确匹配
        if let Some(child) = node.children.get(segment) {
            if let Some(result) = Self::match_node(
                child,
                segments,
                index + 1,
                method,
                params,
            ) {
                return Some(result);
            }
        }

        // 2. 参数匹配
        if let Some(param) = &node.param {
            params.insert(
                param.name.clone(),
                segment.to_string(),
            );

            if let Some(result) = Self::match_node(
                &param.node,
                segments,
                index + 1,
                method,
                params,
            ) {
                return Some(result);
            }

            // 当前参数路线没匹配成功，回退
            params.remove(&param.name);
        }

        // 3. 通配符匹配
        if let Some(wildcard) = &node.wildcard {
            if let Some(route) = wildcard.routes.get(&method) {
                let value = segments[index..].join("/");

                params.insert(
                    wildcard.name.clone(),
                    value,
                );

                return Some(MatchResult {
                    route: route.clone(),
                    params: params.clone(),
                });
            }
        }

        None
    }

    // 匹配路由
    pub fn match_route(
        &self,
        method: Method,
        path: &str,
    ) -> Option<MatchResult> {
        let segments = Self::split_path(path);
        let mut params = HashMap::new();

        Self::match_node(
            &self.root,
            &segments,
            0,
            method,
            &mut params,
        )
    }
}