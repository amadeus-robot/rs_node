use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};

use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;
use stunclient::StunClient;

use crate::CONFIG;

#[derive(Deserialize)]
struct IpResponse {
    ip: String,
}

pub struct STUN;

impl STUN {
    pub fn get_my_public_ipv4(iface: Option<Ipv4Addr>) -> Option<Ipv4Addr> {
        let local_ip = iface.unwrap_or(CONFIG.ama.udp_ipv4_tuple);

        let socket = UdpSocket::bind((local_ip, 0)).ok()?;
        socket.set_read_timeout(Some(Duration::from_secs(6))).ok()?;

        let stun_server: SocketAddr = "stun.l.google.com:19302".to_socket_addrs().ok()?.next()?; // take the first resolved address

        let client = StunClient::new(stun_server); // ✅ pass SocketAddr

        let socket_addr: std::net::SocketAddr = client.query_external_address(&socket).ok()?;

        match socket_addr.ip() {
            std::net::IpAddr::V4(ipv4) => Some(ipv4),
            std::net::IpAddr::V6(_) => None,
        }
    }

    pub fn get_my_public_ipv4_http(iface: Option<Ipv4Addr>) -> Option<Ipv4Addr> {
        let local_ip = iface.unwrap_or(CONFIG.ama.udp_ipv4_tuple);

        // Create a blocking HTTP client bound to the local IP
        let client = Client::builder()
            .timeout(Duration::from_secs(6))
            .local_address(IpAddr::V4(local_ip))
            .build()
            .ok()?;

        // Send GET request
        let resp_text = client
            .get("http://api.myip.la/en?json")
            .send()
            .ok()?
            .text()
            .ok()?;

        // Parse JSON
        let ip_resp: IpResponse = serde_json::from_str(&resp_text).ok()?;

        // Convert string IP to Ipv4Addr
        ip_resp.ip.parse::<Ipv4Addr>().ok()
    }

    /// Get current IPv4 (env -> STUN -> HTTP)
    pub fn get_current_ip4(iface: Option<Ipv4Addr>) -> Option<Ipv4Addr> {
        if let Ok(ip) = env::var("PUBLIC_UDP_IPV4") {
            return ip.parse().ok();
        }

        println!("Trying to get IPv4 via STUN...");
        if let Some(ip) = Self::get_my_public_ipv4(iface) {
            return Some(ip);
        }

        println!("Trying to get IPv4 via HTTP...");
        if let Some(ip) = Self::get_my_public_ipv4_http(iface) {
            return Some(ip);
        }

        println!("Failed to find your node's public IPv4. Set PUBLIC_UDP_IPV4 manually.");
        None
    }
}

#[cfg(test)]
mod stun_tests {
    use super::STUN;
    use std::{env, net::Ipv4Addr};

    #[test]
    fn test_get_current_ip4_env() {
        // Set a temporary env variable
        unsafe { env::set_var("PUBLIC_UDP_IPV4", "1.2.3.4") };

        let ip = STUN::get_current_ip4(None).expect("Should parse env IP");

        println!("PUBLIC_UDP_IPV4 {}", ip);
        assert_eq!(ip, Ipv4Addr::new(1, 2, 3, 4));

        // Clean up
        unsafe { env::remove_var("PUBLIC_UDP_IPV4") };
    }

    #[test]
    fn test_get_my_public_ipv4_stun() {
        // This test will attempt a real STUN request. Might fail if network is unavailable.
        let ip = STUN::get_my_public_ipv4(None);
        if let Some(ip) = ip {
            println!("STUN public IP: {}", ip);
            // Ensure it's a valid IPv4 address (basic sanity check)
            assert!(ip.octets().len() == 4);
        } else {
            println!("STUN test skipped: no public IP detected");
        }
    }

    #[test]
    fn test_get_my_public_ipv4_http() {
        // This test will attempt a real HTTP request. Might fail if network is unavailable.
        let ip = STUN::get_my_public_ipv4_http(None);
        if let Some(ip) = ip {
            println!("HTTP public IP: {}", ip);
            // Ensure it's a valid IPv4 address (basic sanity check)
            assert!(ip.octets().len() == 4);
        } else {
            println!("HTTP test skipped: no public IP detected");
        }
    }

    #[test]
    fn test_get_current_ip4_fallback() {
        // Remove env variable to force fallback
        unsafe { env::remove_var("PUBLIC_UDP_IPV4") };

        let ip = STUN::get_current_ip4(None);
        if let Some(ip) = ip {
            println!("Current public IP: {}", ip);
            assert!(ip.octets().len() == 4);
        } else {
            println!("No public IP found, as expected if network unavailable");
        }
    }
}
