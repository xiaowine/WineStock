//! Desktop server-mode 的局域网访问地址发现。
//!
//! wildcard 监听只读取操作系统报告的真实 IPv4 网卡地址；具体 IPv4 监听直接使用实际绑定地址，
//! 不推导未绑定的接口，也不参与服务生命周期。接口枚举由 `if-addrs` 统一覆盖 Windows、macOS
//! 和 Linux，平台防火墙仍由各自 provider 负责。

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// 返回可以向局域网其它设备展示的地址。
pub(crate) fn discover_lan_access_urls(bound_addr: SocketAddr) -> Vec<String> {
    let port = bound_addr.port();
    match bound_addr.ip() {
        IpAddr::V4(address) if !address.is_unspecified() => {
            return if is_publishable_ipv4(address) {
                vec![format!("http://{address}:{port}")]
            } else {
                Vec::new()
            };
        }
        // Desktop 首版只发布 IPv4；具体 IPv6 监听地址不能凭 IPv4 网卡信息伪造 URL。
        IpAddr::V6(address) if !address.is_unspecified() => return Vec::new(),
        _ => {}
    }

    discover_lan_ipv4_addresses()
        .into_iter()
        .map(|address| format!("http://{address}:{port}"))
        .collect()
}

fn discover_lan_ipv4_addresses() -> Vec<Ipv4Addr> {
    let mut seen_addresses = HashSet::new();
    let mut addresses = Vec::new();
    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return addresses;
    };
    for interface in interfaces {
        if let if_addrs::IfAddr::V4(address) = interface.addr {
            append_publishable_address(address.ip, &mut seen_addresses, &mut addresses);
        }
    }
    addresses
}

fn append_publishable_address(
    address: Ipv4Addr,
    seen_addresses: &mut HashSet<Ipv4Addr>,
    addresses: &mut Vec<Ipv4Addr>,
) {
    if is_publishable_ipv4(address) && seen_addresses.insert(address) {
        addresses.push(address);
    }
}

fn is_publishable_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    (octets[0] == 10)
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
}

#[cfg(test)]
mod tests {
    use super::is_publishable_ipv4;
    use std::net::Ipv4Addr;

    #[test]
    fn only_rfc1918_addresses_are_publishable() {
        assert!(is_publishable_ipv4(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(is_publishable_ipv4(Ipv4Addr::new(172, 16, 2, 3)));
        assert!(is_publishable_ipv4(Ipv4Addr::new(172, 31, 2, 3)));
        assert!(is_publishable_ipv4(Ipv4Addr::new(192, 168, 2, 3)));
        assert!(!is_publishable_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(!is_publishable_ipv4(Ipv4Addr::new(169, 254, 2, 3)));
        assert!(!is_publishable_ipv4(Ipv4Addr::new(172, 32, 2, 3)));
        assert!(!is_publishable_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
        assert!(!is_publishable_ipv4(Ipv4Addr::new(224, 0, 0, 1)));
        assert!(!is_publishable_ipv4(Ipv4Addr::new(255, 255, 255, 255)));
    }

    #[test]
    fn deduplicates_publishable_addresses_in_first_seen_order() {
        use super::{append_publishable_address, discover_lan_access_urls};
        use std::{collections::HashSet, net::SocketAddr};

        let mut seen = HashSet::new();
        let mut addresses = Vec::new();
        append_publishable_address(Ipv4Addr::new(192, 168, 1, 20), &mut seen, &mut addresses);
        append_publishable_address(Ipv4Addr::new(10, 0, 0, 8), &mut seen, &mut addresses);
        append_publishable_address(Ipv4Addr::new(192, 168, 1, 20), &mut seen, &mut addresses);

        assert_eq!(
            addresses,
            vec![Ipv4Addr::new(192, 168, 1, 20), Ipv4Addr::new(10, 0, 0, 8)]
        );
        assert_eq!(
            discover_lan_access_urls("192.168.1.20:17890".parse::<SocketAddr>().unwrap()),
            vec!["http://192.168.1.20:17890"]
        );
    }
}
