//! Desktop server-mode 的局域网访问地址发现。
//!
//! wildcard 监听只读取操作系统报告的真实 IPv4 网卡地址；具体 IPv4 监听直接使用实际绑定地址，
//! 不推导未绑定的接口，也不参与服务生命周期。

use std::net::{Ipv4Addr, SocketAddr};

#[cfg(windows)]
use std::collections::HashSet;

#[cfg(windows)]
use std::net::IpAddr;

/// 返回可以向局域网其它设备展示的地址；非 Windows 平台保持空列表。
#[cfg(not(windows))]
pub(crate) fn discover_lan_access_urls(_bound_addr: SocketAddr) -> Vec<String> {
    Vec::new()
}

/// 返回可以向局域网其它设备展示的地址；非 Windows 平台保持空列表。
#[cfg(windows)]
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

#[cfg(not(windows))]
fn discover_lan_ipv4_addresses() -> Vec<Ipv4Addr> {
    Vec::new()
}

#[cfg(windows)]
fn discover_lan_ipv4_addresses() -> Vec<Ipv4Addr> {
    use std::mem::size_of;

    use windows_sys::Win32::{
        NetworkManagement::{
            IpHelper::{
                GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
                GAA_FLAG_SKIP_MULTICAST,
            },
            Ndis::IfOperStatusUp,
        },
        Networking::WinSock::{AF_INET, SOCKADDR_IN},
    };

    const ERROR_BUFFER_OVERFLOW: u32 = 111;
    const ERROR_SUCCESS: u32 = 0;
    const INITIAL_BUFFER_SIZE: u32 = 15 * 1024;

    let mut buffer_size = INITIAL_BUFFER_SIZE;
    let mut buffer = adapter_buffer(buffer_size);
    loop {
        let mut required_size = buffer_size;
        // SAFETY: `buffer` is a contiguous, correctly aligned allocation. The API writes only
        // within the byte size supplied through `required_size` and uses linked pointers into it.
        let status = unsafe {
            GetAdaptersAddresses(
                AF_INET as u32,
                GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_DNS_SERVER | GAA_FLAG_SKIP_MULTICAST,
                std::ptr::null(),
                buffer.as_mut_ptr(),
                &mut required_size,
            )
        };
        if status == ERROR_BUFFER_OVERFLOW {
            buffer_size = required_size;
            buffer = adapter_buffer(buffer_size);
            continue;
        }
        if status != ERROR_SUCCESS {
            return Vec::new();
        }

        let mut seen_addresses = HashSet::new();
        let mut addresses = Vec::new();
        let mut adapter = buffer.as_mut_ptr();
        while !adapter.is_null() {
            // SAFETY: `adapter` is returned by GetAdaptersAddresses and remains valid while
            // `buffer` is alive. The linked list and unicast nodes are owned by that buffer.
            let adapter_ref = unsafe { &*adapter };
            if adapter_ref.OperStatus == IfOperStatusUp {
                let mut unicast = adapter_ref.FirstUnicastAddress;
                while !unicast.is_null() {
                    // SAFETY: Every node is an API-owned linked-list node in `buffer`; the
                    // socket address length and family are checked before reading SOCKADDR_IN.
                    let unicast_ref = unsafe { &*unicast };
                    let socket_address = &unicast_ref.Address;
                    if !socket_address.lpSockaddr.is_null()
                        && socket_address.iSockaddrLength >= size_of::<SOCKADDR_IN>() as i32
                    {
                        // SAFETY: AF_INET and the length check establish that this pointer
                        // refers to a SOCKADDR_IN provided by the Windows API.
                        let sockaddr =
                            unsafe { &*(socket_address.lpSockaddr.cast::<SOCKADDR_IN>()) };
                        if sockaddr.sin_family == AF_INET {
                            // SAFETY: S_un is the documented IN_ADDR union; S_addr is exposed
                            // in the native byte representation used by the Windows bindings.
                            let raw = unsafe { sockaddr.sin_addr.S_un.S_addr };
                            let address = Ipv4Addr::from(raw.to_ne_bytes());
                            append_publishable_address(
                                address,
                                &mut seen_addresses,
                                &mut addresses,
                            );
                        }
                    }
                    unicast = unicast_ref.Next;
                }
            }
            adapter = adapter_ref.Next;
        }
        return addresses;
    }
}

#[cfg(windows)]
fn append_publishable_address(
    address: Ipv4Addr,
    seen_addresses: &mut HashSet<Ipv4Addr>,
    addresses: &mut Vec<Ipv4Addr>,
) {
    if is_publishable_ipv4(address) && seen_addresses.insert(address) {
        addresses.push(address);
    }
}

#[cfg(windows)]
fn adapter_buffer(
    size: u32,
) -> Vec<windows_sys::Win32::NetworkManagement::IpHelper::IP_ADAPTER_ADDRESSES_LH> {
    use std::mem::size_of;
    use windows_sys::Win32::NetworkManagement::IpHelper::IP_ADAPTER_ADDRESSES_LH;

    let count = (size as usize).div_ceil(size_of::<IP_ADAPTER_ADDRESSES_LH>());
    vec![IP_ADAPTER_ADDRESSES_LH::default(); count.max(1)]
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

    #[cfg(windows)]
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
