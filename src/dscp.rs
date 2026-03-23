// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;

/// Parse a DSCP codepoint from a string. Accepts standard PHB names and numeric 0-63.
pub fn parse_dscp(s: &str) -> Result<u8, String> {
    let upper = s.to_uppercase();
    let val = match upper.as_str() {
        "BE" | "CS0" => 0,
        "CS1" => 8,
        "CS2" => 16,
        "CS3" => 24,
        "CS4" => 32,
        "CS5" => 40,
        "CS6" => 48,
        "CS7" => 56,
        "AF11" => 10,
        "AF12" => 12,
        "AF13" => 14,
        "AF21" => 18,
        "AF22" => 20,
        "AF23" => 22,
        "AF31" => 26,
        "AF32" => 28,
        "AF33" => 30,
        "AF41" => 34,
        "AF42" => 36,
        "AF43" => 38,
        "EF" => 46,
        _ => {
            // Try numeric
            let v: u8 = s
                .parse()
                .map_err(|_| format!("unknown DSCP codepoint: '{s}'"))?;
            if v > 63 {
                return Err(format!("DSCP value {v} out of range (must be 0-63)"));
            }
            v
        }
    };
    Ok(val)
}

/// Convert a DSCP value (0-63) to a TOS byte (DSCP << 2).
/// ECN bits are set to 0. When ECN support is added (#33),
/// this must be updated to preserve ECN bits via read-modify-write.
pub fn dscp_to_tos(dscp: u8) -> u8 {
    dscp << 2
}

/// Get the human-readable name for a DSCP value, or the numeric string.
pub fn dscp_name(dscp: u8) -> String {
    match dscp {
        0 => "BE".to_string(),
        8 => "CS1".to_string(),
        16 => "CS2".to_string(),
        24 => "CS3".to_string(),
        32 => "CS4".to_string(),
        40 => "CS5".to_string(),
        48 => "CS6".to_string(),
        56 => "CS7".to_string(),
        10 => "AF11".to_string(),
        12 => "AF12".to_string(),
        14 => "AF13".to_string(),
        18 => "AF21".to_string(),
        20 => "AF22".to_string(),
        22 => "AF23".to_string(),
        26 => "AF31".to_string(),
        28 => "AF32".to_string(),
        30 => "AF33".to_string(),
        34 => "AF41".to_string(),
        36 => "AF42".to_string(),
        38 => "AF43".to_string(),
        46 => "EF".to_string(),
        v => format!("{v}"),
    }
}

/// Parse a "stream_id=dscp" string (e.g., "0=AF41").
pub fn parse_stream_dscp(s: &str) -> Result<(u8, u8), String> {
    let (id_str, dscp_str) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid stream-dscp format: '{s}' (expected ID=DSCP)"))?;
    let id: u8 = id_str
        .parse()
        .map_err(|_| format!("invalid stream ID: '{id_str}'"))?;
    let dscp = parse_dscp(dscp_str)?;
    Ok((id, dscp))
}

/// Apply IP_TOS to a raw file descriptor. Fails fast with a descriptive error.
/// Only supports IPv4 sockets. Returns an error for IPv6.
pub fn apply_tos_to_fd(fd: std::os::unix::io::RawFd, dscp: u8) -> Result<(), String> {
    let tos = dscp_to_tos(dscp) as libc::c_int;
    let ret = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_TOS,
            &tos as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!(
            "failed to set IP_TOS to {} (DSCP {} / {}): {}",
            tos,
            dscp,
            dscp_name(dscp),
            err
        ));
    }
    Ok(())
}

/// Apply DSCP to a socket2 socket. Asserts IPv4.
pub fn apply_dscp_to_socket(sock: &socket2::Socket, dscp: u8, addr: &SocketAddr) -> Result<(), String> {
    if addr.is_ipv6() {
        return Err(format!(
            "DSCP marking is IPv4-only. IPv6 endpoint {} requires IPV6_TCLASS (not yet supported)",
            addr
        ));
    }
    apply_tos_to_fd(sock.as_raw_fd(), dscp)
}

/// Resolve the effective DSCP for a given stream. Returns the per-stream override
/// if present, otherwise the global default, otherwise None.
pub fn resolve_stream_dscp(
    stream_id: u8,
    global_dscp: Option<u8>,
    stream_overrides: &[(u8, u8)],
) -> Option<u8> {
    stream_overrides
        .iter()
        .find(|(id, _)| *id == stream_id)
        .map(|(_, dscp)| *dscp)
        .or(global_dscp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_standard_names() {
        assert_eq!(parse_dscp("BE").unwrap(), 0);
        assert_eq!(parse_dscp("CS0").unwrap(), 0);
        assert_eq!(parse_dscp("CS1").unwrap(), 8);
        assert_eq!(parse_dscp("CS7").unwrap(), 56);
        assert_eq!(parse_dscp("AF11").unwrap(), 10);
        assert_eq!(parse_dscp("AF41").unwrap(), 34);
        assert_eq!(parse_dscp("AF43").unwrap(), 38);
        assert_eq!(parse_dscp("EF").unwrap(), 46);
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(parse_dscp("ef").unwrap(), 46);
        assert_eq!(parse_dscp("af41").unwrap(), 34);
        assert_eq!(parse_dscp("cs6").unwrap(), 48);
    }

    #[test]
    fn parse_numeric() {
        assert_eq!(parse_dscp("0").unwrap(), 0);
        assert_eq!(parse_dscp("46").unwrap(), 46);
        assert_eq!(parse_dscp("63").unwrap(), 63);
    }

    #[test]
    fn parse_numeric_out_of_range() {
        assert!(parse_dscp("64").is_err());
        assert!(parse_dscp("255").is_err());
    }

    #[test]
    fn parse_invalid_name() {
        assert!(parse_dscp("INVALID").is_err());
        assert!(parse_dscp("AF44").is_err());
        assert!(parse_dscp("").is_err());
    }

    #[test]
    fn tos_conversion() {
        assert_eq!(dscp_to_tos(0), 0);
        assert_eq!(dscp_to_tos(46), 184); // EF: 46 << 2 = 184
        assert_eq!(dscp_to_tos(34), 136); // AF41: 34 << 2 = 136
        assert_eq!(dscp_to_tos(48), 192); // CS6: 48 << 2 = 192
    }

    #[test]
    fn name_lookup() {
        assert_eq!(dscp_name(0), "BE");
        assert_eq!(dscp_name(46), "EF");
        assert_eq!(dscp_name(34), "AF41");
        assert_eq!(dscp_name(7), "7"); // non-standard
    }

    #[test]
    fn parse_stream_dscp_valid() {
        let (id, dscp) = parse_stream_dscp("0=AF41").unwrap();
        assert_eq!(id, 0);
        assert_eq!(dscp, 34);

        let (id, dscp) = parse_stream_dscp("1=EF").unwrap();
        assert_eq!(id, 1);
        assert_eq!(dscp, 46);

        let (id, dscp) = parse_stream_dscp("2=0").unwrap();
        assert_eq!(id, 2);
        assert_eq!(dscp, 0);
    }

    #[test]
    fn parse_stream_dscp_invalid() {
        assert!(parse_stream_dscp("no_equals").is_err());
        assert!(parse_stream_dscp("abc=EF").is_err());
        assert!(parse_stream_dscp("0=INVALID").is_err());
    }

    #[test]
    fn resolve_dscp_override_wins() {
        let overrides = vec![(0, 34), (1, 46)];
        assert_eq!(resolve_stream_dscp(0, Some(0), &overrides), Some(34));
        assert_eq!(resolve_stream_dscp(1, Some(0), &overrides), Some(46));
    }

    #[test]
    fn resolve_dscp_global_fallback() {
        let overrides = vec![(0, 34)];
        assert_eq!(resolve_stream_dscp(1, Some(46), &overrides), Some(46));
    }

    #[test]
    fn resolve_dscp_none() {
        assert_eq!(resolve_stream_dscp(0, None, &[]), None);
    }

    #[test]
    fn ipv6_rejected() {
        let addr: SocketAddr = "[::1]:5000".parse().unwrap();
        let sock = socket2::Socket::new(
            socket2::Domain::IPV6,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        ).unwrap();
        let result = apply_dscp_to_socket(&sock, 46, &addr);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("IPv4-only"));
    }
}
