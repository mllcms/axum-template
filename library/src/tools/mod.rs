pub mod unit;

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Cursor},
    net::{Ipv4Addr, SocketAddr},
};

use axum::{extract::Request, Router};
use color_string::{pcs, Colored, Font::*};
use serde::Deserialize;

use crate::{res, resp};

pub fn parse_query<'de, T: Deserialize<'de>>(req: &'de Request) -> resp::Result<T> {
    let uri = req.uri().query().unwrap_or_default();
    serde_urlencoded::from_str(uri).map_err(|err| res!(422, "{err}"))
}

pub fn print_address(addr: &SocketAddr, protocol: &str) {
    let mut ips = vec![addr.ip()];
    if addr.ip() == Ipv4Addr::new(0, 0, 0, 0) {
        if let Ok(vec) = if_addrs::get_if_addrs() {
            let (mut local, mut network): (Vec<_>, Vec<_>) = vec.into_iter().map(|m| m.ip()).partition(|p| p.is_ipv4());
            local.sort();
            network.sort();
            local.extend(network);
            ips = local;
        }
    }

    let port = addr.port().bold();
    for ip in ips.iter().filter(|f| f.is_ipv4()) {
        if ip.is_loopback() {
            pcs!(Green => "➜  "; RBold => "Local:   "; RCyan => format!("{protocol}://{ip}:{port}"));
        } else {
            pcs!(Green => "➜  "; RBold => "Network: "; RCyan => format!("{protocol}://{ip}:{port}"));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RouteInfo {
    pub path: String,
    pub method: String,
}

pub fn print_router_info(router: &Router) {
    let cursor = Cursor::new(format!("{:#?}", router));
    let reader = BufReader::new(cursor);
    let mut map: HashMap<i32, RouteInfo> = HashMap::new();

    let mut info: Option<&mut RouteInfo> = None;
    let mut lines = reader.lines().flatten().take_while(|t| !t.contains("fallback_router"));
    while let Some(line) = lines.next() {
        match &mut info {
            Some(_) if line.contains("RouteId(") => info = None,
            Some(RouteInfo { path, method }) => {
                if line.contains("Route(") {
                    *method = "ROUTE".to_string();
                } else if line.contains("allow_header:") {
                    let n = line.find(':').unwrap();
                    if line[n..].contains("Skip") {
                        method.push_str("ALL");
                    } else if let Some(line) = lines.next() {
                        method.push_str(line.trim_matches([' ', 'b', '"', ',']))
                    }
                } else if let Some(n) = line.find('"') {
                    *path = line[n + 1..line.len() - 2]
                        .trim_end_matches("__private__axum_nest_tail_param")
                        .to_string();
                }
            }
            None => {
                if let Ok(id) = line.trim_matches([' ', ',']).parse() {
                    info = Some(map.entry(id).or_default())
                }
            }
        }
    }

    if map.is_empty() {
        return;
    }

    let mut max_path = 0;
    let mut max_method = 0;
    let mut vec: Vec<RouteInfo> = map
        .into_values()
        .filter_map(|info| {
            // nest_service 输出 path/* 过滤 path 和 path/
            if info.method == "ROUTE" && !info.path.ends_with('*') {
                return None;
            }
            max_path = max_path.max(info.path.len());
            max_method = max_method.max(info.method.len());
            Some(info)
        })
        .collect();

    max_method = max_method.max(6);
    vec.sort_by(|a, b| a.path.cmp(&b.path));

    let path_ = "─".repeat(max_path + 2);
    let method_ = "─".repeat(max_method + 2);
    println!("┌{path_}┬{method_}┐");
    println!("│ {:^max_path$} │ {:^max_method$} │", "Path", "Method");
    for RouteInfo { path, method } in vec {
        println!("├{path_}┼{method_}┤");
        println!("│ {path:max_path$} │ {:max_method$} │", method.trim_end())
    }
    println!("└{path_}┴{method_}┘");
}
