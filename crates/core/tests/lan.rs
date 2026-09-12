use quiz_buzzer_core::lan::{is_usable_ipv4, lan_base_urls};
use std::net::Ipv4Addr;

#[test]
fn rejects_loopback_and_link_local() {
    assert!(!is_usable_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
    assert!(!is_usable_ipv4(Ipv4Addr::new(169, 254, 1, 1)));
    assert!(is_usable_ipv4(Ipv4Addr::new(192, 168, 1, 20)));
    assert!(is_usable_ipv4(Ipv4Addr::new(10, 0, 0, 2)));
    assert!(is_usable_ipv4(Ipv4Addr::new(172, 16, 5, 1)));
}

#[test]
fn urls_include_port_and_slash() {
    let urls = lan_base_urls(7423, &[Ipv4Addr::new(192, 168, 1, 20)]);
    assert_eq!(urls, vec!["http://192.168.1.20:7423/"]);
}
