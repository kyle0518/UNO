use std::net::{IpAddr, Ipv4Addr};

use qrcode::render::unicode;
use qrcode::types::QrError;
use qrcode::QrCode;

/// 判斷介面名稱是否屬於需要排除的虛擬 / 容器網路介面。
pub fn is_virtual_interface(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("docker") || lower.starts_with("br-") || lower.starts_with("veth")
}

/// 從網路介面清單過濾出可供區網手機連線的 IPv4 位址，並保持順序去重。
pub fn filter_lan_ipv4(interfaces: &[(String, IpAddr)]) -> Vec<Ipv4Addr> {
    let mut result = Vec::new();
    for (name, ip) in interfaces {
        if is_virtual_interface(name) {
            continue;
        }
        let IpAddr::V4(v4) = ip else {
            continue;
        };
        if v4.is_loopback()
            || v4.is_link_local()
            || v4.is_unspecified()
            || v4.is_multicast()
            || v4.is_broadcast()
        {
            continue;
        }
        if !result.contains(v4) {
            result.push(*v4);
        }
    }
    result
}

fn format_ip_url(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(v4) => format!("http://{v4}:{port}"),
        IpAddr::V6(v6) => format!("http://[{v6}]:{port}"),
    }
}

/// 依監聽位址與系統網路介面列出區網連線 URL。
pub fn discover_lan_urls(host: &str, port: u16) -> Vec<String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_loopback() {
            return Vec::new();
        }
        if !ip.is_unspecified() {
            return vec![format_ip_url(ip, port)];
        }
    } else if host.eq_ignore_ascii_case("localhost") {
        return Vec::new();
    }

    match local_ip_address::list_afinet_netifas() {
        Ok(interfaces) => filter_lan_ipv4(&interfaces)
            .into_iter()
            .map(|ip| format!("http://{ip}:{port}"))
            .collect(),
        Err(err) => {
            tracing::warn!(error = %err, "無法列舉區網網路介面");
            Vec::new()
        }
    }
}

/// 將 URL 渲染成終端機可直接掃描的 Unicode QR Code 字串。
pub fn render_qr_unicode(url: &str) -> Result<String, QrError> {
    let code = QrCode::new(url.as_bytes())?;
    let rendered = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .quiet_zone(true)
        .build();
    Ok(rendered)
}

/// 組合啟動時的終端機資訊與 QR Code 橫幅。
pub fn format_startup_banner(port: u16, lan_urls: &[String]) -> String {
    let mut lines = vec![
        "UNO server running".to_string(),
        format!("  本機:   http://localhost:{port}"),
    ];

    if lan_urls.is_empty() {
        lines.push("  區網:   （未偵測到區網 IPv4 位址）".to_string());
    } else {
        for (idx, url) in lan_urls.iter().enumerate() {
            if idx == 0 {
                lines.push(format!("  區網:   {url}   <- 手機掃描下方 QR Code"));
            } else {
                lines.push(format!("  區網:   {url}"));
            }
        }
        if let Some(primary) = lan_urls.first() {
            match render_qr_unicode(primary) {
                Ok(qr) => {
                    for qr_line in qr.lines() {
                        lines.push(format!("  {qr_line}"));
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, url = %primary, "無法產生啟動 QR Code");
                }
            }
        }
    }

    lines.join("\n")
}

/// 在終端機印出伺服器連線資訊與 QR Code。
pub fn print_startup_banner(port: u16, lan_urls: &[String]) {
    println!("{}", format_startup_banner(port, lan_urls));
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use super::*;

    #[test]
    fn filters_out_virtual_and_non_lan_interfaces() {
        let interfaces = vec![
            ("lo".to_string(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            (
                "docker0".to_string(),
                IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1)),
            ),
            (
                "br-9c81a2".to_string(),
                IpAddr::V4(Ipv4Addr::new(172, 18, 0, 1)),
            ),
            (
                "veth1234abcd".to_string(),
                IpAddr::V4(Ipv4Addr::new(172, 19, 0, 2)),
            ),
            (
                "eth0".to_string(),
                IpAddr::V4(Ipv4Addr::new(169, 254, 10, 20)),
            ),
            (
                "wlan0".to_string(),
                IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)),
            ),
            (
                "wlan0".to_string(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 23)),
            ),
            (
                "eth1".to_string(),
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 23)),
            ),
            ("eth2".to_string(), IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))),
        ];

        let filtered = filter_lan_ipv4(&interfaces);
        assert_eq!(
            filtered,
            vec![Ipv4Addr::new(192, 168, 1, 23), Ipv4Addr::new(10, 0, 0, 5)]
        );
    }

    #[test]
    fn loopback_host_returns_no_lan_urls() {
        assert!(discover_lan_urls("127.0.0.1", 8787).is_empty());
        assert!(discover_lan_urls("localhost", 8787).is_empty());
    }

    #[test]
    fn explicit_host_ip_returns_single_lan_url() {
        assert_eq!(
            discover_lan_urls("192.168.1.23", 8787),
            vec!["http://192.168.1.23:8787"]
        );
    }

    #[test]
    fn renders_unicode_qr_and_startup_banner() {
        let qr = render_qr_unicode("http://192.168.1.23:8787").expect("render qr");
        assert!(qr.lines().count() > 5);
        assert!(qr.contains('\u{2588}') || qr.contains('\u{2580}') || qr.contains('\u{2584}'));

        let banner = format_startup_banner(8787, &["http://192.168.1.23:8787".to_string()]);
        assert!(banner.contains("http://localhost:8787"));
        assert!(banner.contains("http://192.168.1.23:8787"));
        assert!(banner.contains("手機掃描下方 QR Code"));
    }
}
