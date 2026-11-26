//! Bộ lọc chống tấn công SSRF (Server-Side Request Forgery) cho module Uptime Monitor.
//! Chặn các địa chỉ loopback, private LAN, link-local, cloud metadata (`169.254.169.254`) và IPv6 nội bộ.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use pum_core::{AppError, AppResult, validation::validate_http_url};
use tokio::net::lookup_host;
use url::Url;

/// Kiểm tra một địa chỉ IP có thuộc dải nội bộ / nguy hiểm bị cấm SSRF hay không.
#[must_use]
pub const fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

const fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || ip.is_unspecified()
        // Carrier-Grade NAT 100.64.0.0/10
        || (octets[0] == 100 && (octets[1] & 0b1100_0000) == 64)
        // Benchmarking 198.18.0.0/15
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        // Reserved 240.0.0.0/4
        || octets[0] >= 240
}

const fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_forbidden_ipv4(mapped);
    }
    let segs = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // Unique Local Addresses fc00::/7
        || (segs[0] & 0xfe00) == 0xfc00
        // Link-local fe80::/10
        || (segs[0] & 0xffc0) == 0xfe80
}

/// Kiểm tra URL đích trước khi gửi HTTP request từ server.
pub async fn validate_outbound_url(raw: &str, allow_private: bool) -> AppResult<Url> {
    let url = validate_http_url(raw)?;
    if allow_private {
        return Ok(url);
    }

    let host = url
        .host_str()
        .ok_or_else(|| AppError::Validation("URL thiếu host".to_owned()))?;
    let host_lower = host.to_ascii_lowercase();

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
    {
        return Err(AppError::SecurityViolation(
            "SSRF Guard: không cho phép theo dõi địa chỉ mạng nội bộ/localhost".to_owned(),
        ));
    }

    // Nếu host là IP trực tiếp, kiểm tra ngay
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        if is_forbidden_ip(ip) {
            return Err(AppError::SecurityViolation(format!(
                "SSRF Guard: địa chỉ IP `{ip}` thuộc dải nội bộ không được phép"
            )));
        }
        return Ok(url);
    }

    // Phân giải DNS và kiểm tra toàn bộ IP trả về
    let port = url.port_or_known_default().unwrap_or(80);
    let addrs = lookup_host((host, port)).await.map_err(|_| {
        AppError::Validation(format!("không thể phân giải tên miền `{host}` qua DNS"))
    })?;

    let mut found_any = false;
    for addr in addrs {
        found_any = true;
        if is_forbidden_ip(addr.ip()) {
            return Err(AppError::SecurityViolation(format!(
                "SSRF Guard: tên miền `{host}` trỏ về IP nội bộ bị chặn ({})",
                addr.ip()
            )));
        }
    }

    if !found_any {
        return Err(AppError::Validation(format!(
            "tên miền `{host}` không trả về địa chỉ IP hợp lệ"
        )));
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_private_and_metadata_ips() {
        let blocked = [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.0.1",
            "192.168.1.10",
            "169.254.169.254",
            "100.64.1.2",
            "0.0.0.0",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
        ];
        for raw in blocked {
            let ip: IpAddr = raw.parse().expect("valid IP literal");
            assert!(is_forbidden_ip(ip), "expected {raw} to be blocked");
        }
    }

    #[test]
    fn allows_public_internet_ips() {
        for raw in ["1.1.1.1", "8.8.8.8", "140.82.121.4", "2606:4700:4700::1111"] {
            let ip: IpAddr = raw.parse().expect("valid IP literal");
            assert!(!is_forbidden_ip(ip), "expected {raw} to be allowed");
        }
    }
}
