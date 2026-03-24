// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;

use serde::{Deserialize, Serialize};

// --- ECN ---

/// ECN mode for outgoing packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcnMode {
    Off,
    Ect0,
    Ect1,
}

impl EcnMode {
    pub fn name(&self) -> Option<&'static str> {
        match self {
            EcnMode::Off => None,
            EcnMode::Ect0 => Some("ECT0"),
            EcnMode::Ect1 => Some("ECT1"),
        }
    }
}

/// Parse ECN mode from a string ("ect0" or "ect1").
pub fn parse_ecn_mode(s: &str) -> Result<EcnMode, String> {
    match s.to_lowercase().as_str() {
        "ect0" => Ok(EcnMode::Ect0),
        "ect1" => Ok(EcnMode::Ect1),
        _ => Err(format!("invalid ECN mode: '{s}'. Must be: ect0, ect1")),
    }
}

/// ECN codepoint observed on a received packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcnCodepoint {
    NotEct,  // 00
    Ect1,    // 01
    Ect0,    // 10
    Ce,      // 11
}

impl EcnCodepoint {
    /// Extract ECN codepoint from a TOS byte.
    pub fn from_tos(tos: u8) -> Self {
        match tos & 0x03 {
            0b00 => EcnCodepoint::NotEct,
            0b01 => EcnCodepoint::Ect1,
            0b10 => EcnCodepoint::Ect0,
            0b11 => EcnCodepoint::Ce,
            _ => unreachable!(),
        }
    }
}

/// Tracks ECN codepoint counts on received packets.
#[derive(Debug, Clone, Default)]
pub struct EcnCounters {
    pub not_ect: u64,
    pub ect0: u64,
    pub ect1: u64,
    pub ce: u64,
    pub unknown: u64,
}

impl EcnCounters {
    pub fn record(&mut self, codepoint: EcnCodepoint) {
        match codepoint {
            EcnCodepoint::NotEct => self.not_ect += 1,
            EcnCodepoint::Ect0 => self.ect0 += 1,
            EcnCodepoint::Ect1 => self.ect1 += 1,
            EcnCodepoint::Ce => self.ce += 1,
        }
    }

    pub fn record_unknown(&mut self) {
        self.unknown += 1;
    }

    /// Total packets with known ECN state.
    pub fn total_observed(&self) -> u64 {
        self.not_ect + self.ect0 + self.ect1 + self.ce
    }

    /// CE ratio as percentage, excluding unknowns. Returns None if no observations.
    pub fn ce_ratio(&self) -> Option<f64> {
        let total = self.total_observed();
        if total == 0 {
            return None;
        }
        Some(self.ce as f64 / total as f64 * 100.0)
    }
}

// --- DSCP ---

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

/// Build a TOS byte from DSCP and ECN mode.
/// Replaces the old `dscp_to_tos()` which only handled DSCP.
pub fn build_tos(dscp: Option<u8>, ecn: EcnMode) -> u8 {
    let dscp_bits = dscp.unwrap_or(0) << 2;
    let ecn_bits: u8 = match ecn {
        EcnMode::Off => 0,
        EcnMode::Ect0 => 0b10,
        EcnMode::Ect1 => 0b01,
    };
    dscp_bits | ecn_bits
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

// --- Socket helpers ---

/// Apply TOS byte (DSCP+ECN combined) to a raw file descriptor.
/// Fails fast with a descriptive error.
pub fn apply_tos_to_fd(fd: std::os::unix::io::RawFd, tos: u8) -> Result<(), String> {
    let tos_int = tos as libc::c_int;
    let ret = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_TOS,
            &tos_int as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("failed to set IP_TOS to {tos} (0x{tos:02x}): {err}"));
    }
    Ok(())
}

/// Apply DSCP+ECN to a socket2 socket. Asserts IPv4.
pub fn apply_tos_to_socket(
    sock: &socket2::Socket,
    dscp: Option<u8>,
    ecn: EcnMode,
    addr: &SocketAddr,
) -> Result<(), String> {
    if addr.is_ipv6() {
        return Err(format!(
            "TOS marking is IPv4-only. IPv6 endpoint {} requires IPV6_TCLASS (not yet supported)",
            addr
        ));
    }
    let tos = build_tos(dscp, ecn);
    if tos == 0 {
        return Ok(()); // Default TOS, no need to set
    }
    apply_tos_to_fd(sock.as_raw_fd(), tos)
}

/// Enable IP_RECVTOS on a raw file descriptor so recvmsg returns TOS in cmsg.
/// Fail-fast: returns error if the option cannot be enabled.
pub fn enable_recv_tos(fd: std::os::unix::io::RawFd) -> Result<(), String> {
    let val: libc::c_int = 1;
    let ret = unsafe {
        libc::setsockopt(
            fd,
            libc::IPPROTO_IP,
            libc::IP_RECVTOS,
            &val as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    if ret != 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("failed to enable IP_RECVTOS: {err}"));
    }
    Ok(())
}

/// Receive a UDP datagram via recvmsg, extracting the TOS byte from cmsg.
/// Returns (bytes_read, source_addr, Option<tos_byte>).
/// This is a non-blocking call — must only be called when the fd is readable.
pub fn recvmsg_with_tos(
    fd: std::os::unix::io::RawFd,
    buf: &mut [u8],
) -> std::io::Result<(usize, Option<u8>)> {
    let mut iov = libc::iovec {
        iov_base: buf.as_mut_ptr() as *mut libc::c_void,
        iov_len: buf.len(),
    };

    // Control message buffer — enough for one IP_TOS cmsg
    let mut cmsg_buf = [0u8; 64];

    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_buf.len() as _;

    let n = unsafe { libc::recvmsg(fd, &mut msg, libc::MSG_DONTWAIT) };
    if n < 0 {
        return Err(std::io::Error::last_os_error());
    }

    // Parse cmsg for IP_TOS
    let mut tos: Option<u8> = None;
    unsafe {
        let mut cmsg = libc::CMSG_FIRSTHDR(&msg);
        while !cmsg.is_null() {
            if (*cmsg).cmsg_level == libc::IPPROTO_IP && (*cmsg).cmsg_type == libc::IP_TOS {
                let data = libc::CMSG_DATA(cmsg);
                // IP_TOS cmsg data is a c_int
                let tos_int = std::ptr::read_unaligned(data as *const libc::c_int);
                tos = Some(tos_int as u8);
            }
            cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
        }
    }

    Ok((n as usize, tos))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- DSCP tests ---

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
    fn name_lookup() {
        assert_eq!(dscp_name(0), "BE");
        assert_eq!(dscp_name(46), "EF");
        assert_eq!(dscp_name(34), "AF41");
        assert_eq!(dscp_name(7), "7");
    }

    #[test]
    fn parse_stream_dscp_valid() {
        let (id, dscp) = parse_stream_dscp("0=AF41").unwrap();
        assert_eq!(id, 0);
        assert_eq!(dscp, 34);
    }

    #[test]
    fn parse_stream_dscp_invalid() {
        assert!(parse_stream_dscp("no_equals").is_err());
        assert!(parse_stream_dscp("abc=EF").is_err());
        assert!(parse_stream_dscp("0=INVALID").is_err());
    }

    #[test]
    fn ipv6_rejected() {
        let addr: SocketAddr = "[::1]:5000".parse().unwrap();
        let sock = socket2::Socket::new(
            socket2::Domain::IPV6,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .unwrap();
        let result = apply_tos_to_socket(&sock, Some(46), EcnMode::Off, &addr);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("IPv4-only"));
    }

    // --- ECN tests ---

    #[test]
    fn parse_ecn_valid() {
        assert_eq!(parse_ecn_mode("ect0").unwrap(), EcnMode::Ect0);
        assert_eq!(parse_ecn_mode("ect1").unwrap(), EcnMode::Ect1);
        assert_eq!(parse_ecn_mode("ECT0").unwrap(), EcnMode::Ect0);
    }

    #[test]
    fn parse_ecn_invalid() {
        assert!(parse_ecn_mode("invalid").is_err());
        assert!(parse_ecn_mode("ce").is_err());
        assert!(parse_ecn_mode("").is_err());
    }

    #[test]
    fn build_tos_dscp_only() {
        assert_eq!(build_tos(Some(46), EcnMode::Off), 184); // 46 << 2 = 184
        assert_eq!(build_tos(Some(0), EcnMode::Off), 0);
        assert_eq!(build_tos(None, EcnMode::Off), 0);
    }

    #[test]
    fn build_tos_ecn_only() {
        assert_eq!(build_tos(None, EcnMode::Ect0), 0b10);
        assert_eq!(build_tos(None, EcnMode::Ect1), 0b01);
    }

    #[test]
    fn build_tos_combined() {
        // DSCP=46 (EF) + ECT(0) = 184 | 2 = 186 = 0xBA
        assert_eq!(build_tos(Some(46), EcnMode::Ect0), 0xBA);
        // DSCP=46 (EF) + ECT(1) = 184 | 1 = 185 = 0xB9
        assert_eq!(build_tos(Some(46), EcnMode::Ect1), 0xB9);
    }

    #[test]
    fn ecn_codepoint_from_tos() {
        assert_eq!(EcnCodepoint::from_tos(0x00), EcnCodepoint::NotEct);
        assert_eq!(EcnCodepoint::from_tos(0x01), EcnCodepoint::Ect1);
        assert_eq!(EcnCodepoint::from_tos(0x02), EcnCodepoint::Ect0);
        assert_eq!(EcnCodepoint::from_tos(0x03), EcnCodepoint::Ce);
        // With DSCP bits set
        assert_eq!(EcnCodepoint::from_tos(0xBA), EcnCodepoint::Ect0); // EF + ECT0
        assert_eq!(EcnCodepoint::from_tos(0xBB), EcnCodepoint::Ce);   // EF + CE
    }

    #[test]
    fn ecn_counters() {
        let mut c = EcnCounters::default();
        c.record(EcnCodepoint::Ect0);
        c.record(EcnCodepoint::Ect0);
        c.record(EcnCodepoint::Ce);
        c.record_unknown();

        assert_eq!(c.ect0, 2);
        assert_eq!(c.ce, 1);
        assert_eq!(c.unknown, 1);
        assert_eq!(c.total_observed(), 3);
        let ratio = c.ce_ratio().unwrap();
        assert!((ratio - 33.333).abs() < 0.1);
    }

    #[test]
    fn ecn_counters_empty() {
        let c = EcnCounters::default();
        assert!(c.ce_ratio().is_none());
    }

    #[test]
    fn ecn_mode_name() {
        assert_eq!(EcnMode::Off.name(), None);
        assert_eq!(EcnMode::Ect0.name(), Some("ECT0"));
        assert_eq!(EcnMode::Ect1.name(), Some("ECT1"));
    }
}
