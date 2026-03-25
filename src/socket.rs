// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::{IpAddr, SocketAddr};
use std::os::unix::io::AsRawFd;

/// Bind a socket2 socket to a specific network interface via SO_BINDTODEVICE.
pub fn bind_to_device(sock: &socket2::Socket, iface: &str) -> Result<(), String> {
    sock.bind_device(Some(iface.as_bytes())).map_err(|e| {
        format!(
            "SO_BINDTODEVICE failed for interface '{}': {}",
            iface, e
        )
    })
}

/// Bind a socket2 socket to a specific source IP (port 0 for ephemeral).
pub fn bind_source_ip(sock: &socket2::Socket, ip: IpAddr) -> Result<(), String> {
    let addr = SocketAddr::new(ip, 0);
    sock.bind(&socket2::SockAddr::from(addr)).map_err(|e| {
        format!("bind to source IP {} failed: {}", ip, e)
    })
}

/// Apply all pre-connect socket options in the correct order:
/// SO_BINDTODEVICE -> bind(source_ip) -> set_tos
pub fn setup_socket(
    sock: &socket2::Socket,
    bind_iface: Option<&str>,
    source_ip: Option<IpAddr>,
    tos: Option<u8>,
) -> Result<(), String> {
    if let Some(iface) = bind_iface {
        bind_to_device(sock, iface)?;
    }

    if let Some(ip) = source_ip {
        bind_source_ip(sock, ip)?;
    }

    if let Some(tos_val) = tos {
        crate::dscp::apply_tos_to_fd(sock.as_raw_fd(), tos_val)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_to_device_invalid_interface() {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        let result = bind_to_device(&sock, "nonexistent_if_xyz");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("SO_BINDTODEVICE"));
        assert!(err.contains("nonexistent_if_xyz"));
    }

    #[test]
    fn bind_source_ip_invalid() {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        let ip: IpAddr = "192.0.2.99".parse().unwrap();
        let result = bind_source_ip(&sock, ip);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("192.0.2.99"));
    }

    #[test]
    fn setup_socket_no_options() {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        let result = setup_socket(&sock, None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn setup_socket_invalid_iface_fails_fast() {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        let result = setup_socket(&sock, Some("bad_iface_xyz"), None, Some(184));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SO_BINDTODEVICE"));
    }
}
