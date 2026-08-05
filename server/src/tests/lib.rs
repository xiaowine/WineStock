//! server shell 生命周期辅助测试。

use super::*;

#[test]
fn access_url_never_uses_unspecified_ipv4_address() {
    let url = access_url(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 17890)));

    assert_eq!(url, "http://127.0.0.1:17890");
}

#[test]
fn explicit_access_url_is_reported_directly() {
    let url = access_url(SocketAddr::from((Ipv4Addr::new(10, 0, 0, 8), 17890)));

    assert_eq!(url, "http://10.0.0.8:17890");
}

#[test]
fn bind_address_is_reported_without_network_interface_expansion() {
    assert_eq!(
        display_bind_addr(SocketAddr::from(([0, 0, 0, 0], 17890))),
        "0.0.0.0:17890"
    );
    assert_eq!(
        display_bind_addr(SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], 17890))),
        "[::]:17890"
    );
}
