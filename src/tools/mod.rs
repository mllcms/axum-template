pub mod compare;
mod macros;
#[cfg(feature = "multipart")]
pub mod multipart;
pub mod resp;
pub mod unit;
pub mod validator;

use std::net::{Ipv4Addr, SocketAddr};

use color_string::{pcs, Colored, Font::*};

pub fn prompt_address(addr: &SocketAddr, protocol: &str) {
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
