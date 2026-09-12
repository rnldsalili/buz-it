use std::net::Ipv4Addr;

pub fn is_usable_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified() && !ip.is_multicast()
}

pub fn list_ipv4() -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    for iface in if_addrs::get_if_addrs().unwrap_or_default() {
        if let if_addrs::IfAddr::V4(v4) = iface.addr {
            if is_usable_ipv4(v4.ip) {
                out.push(v4.ip);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

pub fn lan_base_urls(port: u16, ips: &[Ipv4Addr]) -> Vec<String> {
    ips.iter()
        .map(|ip| format!("http://{ip}:{port}/"))
        .collect()
}
