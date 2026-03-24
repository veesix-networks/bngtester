// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::{interval, Instant};
use tokio_util::sync::CancellationToken;

use crate::protocol::TrafficPattern;
use crate::traffic::packet::{build_packet, clock_now, PacketHeader, FLAG_LAST};

/// IMIX sizes and ratios (Simple IMIX: 7:4:1).
const IMIX_SIZES: [(usize, u32); 3] = [(64, 7), (594, 4), (1518, 1)];

/// Sweep sizes: 64 to 1518 in steps.
const SWEEP_SIZES: [usize; 8] = [64, 128, 256, 512, 768, 1024, 1280, 1518];

/// Resolve the next packet size based on pattern and sequence number.
pub fn next_packet_size(pattern: TrafficPattern, fixed_size: usize, seq: u32) -> usize {
    match pattern {
        TrafficPattern::Fixed => fixed_size,
        TrafficPattern::Imix => {
            let cycle_len: u32 = IMIX_SIZES.iter().map(|(_, r)| r).sum();
            let pos = seq % cycle_len;
            let mut acc = 0u32;
            for &(size, ratio) in &IMIX_SIZES {
                acc += ratio;
                if pos < acc {
                    return size;
                }
            }
            IMIX_SIZES[0].0
        }
        TrafficPattern::Sweep => {
            let idx = (seq as usize) % SWEEP_SIZES.len();
            SWEEP_SIZES[idx]
        }
    }
}

/// Configuration for a UDP stream generator.
pub struct UdpGeneratorConfig {
    pub target: SocketAddr,
    pub stream_id: u8,
    pub rate_pps: u32,
    pub duration: Duration,
    pub packet_size: usize,
    pub pattern: TrafficPattern,
    pub tos: Option<u8>,
}

/// Result from a completed UDP generator run.
pub struct UdpGeneratorResult {
    pub stream_id: u8,
    pub packets_sent: u64,
    pub bytes_sent: u64,
}

/// Run a UDP stream generator. Sends timestamped packets at the configured rate.
pub async fn run_udp_generator(
    config: UdpGeneratorConfig,
    cancel: CancellationToken,
) -> std::io::Result<UdpGeneratorResult> {
    let socket = if let Some(tos) = config.tos {
        // Create via socket2 to set TOS before any packets are sent
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )?;
        use std::os::unix::io::AsRawFd;
        crate::dscp::apply_tos_to_fd(sock.as_raw_fd(), tos)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
        sock.bind(&socket2::SockAddr::from("0.0.0.0:0".parse::<SocketAddr>().unwrap()))?;
        sock.connect(&socket2::SockAddr::from(config.target))?;
        sock.set_nonblocking(true)?;
        UdpSocket::from_std(std::net::UdpSocket::from(sock))?
    } else {
        let s = UdpSocket::bind("0.0.0.0:0").await?;
        s.connect(config.target).await?;
        s
    };

    let mut seq: u32 = 0;
    let mut packets_sent: u64 = 0;
    let mut bytes_sent: u64 = 0;

    let start = Instant::now();

    if config.rate_pps == 0 {
        // Unlimited rate — send as fast as possible
        loop {
            if cancel.is_cancelled() || start.elapsed() >= config.duration {
                break;
            }
            let size = next_packet_size(config.pattern, config.packet_size, seq);
            let (ts_sec, ts_nsec) = clock_now();
            let header = PacketHeader {
                stream_id: config.stream_id,
                flags: 0,
                seq,
                ts_sec,
                ts_nsec,
                payload_len: 0,
            };
            let pkt = build_packet(&header, size);
            match socket.send(&pkt).await {
                Ok(n) => {
                    packets_sent += 1;
                    bytes_sent += n as u64;
                }
                Err(_) if cancel.is_cancelled() => break,
                Err(e) => return Err(e),
            }
            seq = seq.wrapping_add(1);
        }
    } else {
        // Rate-limited
        let pkt_interval = Duration::from_secs_f64(1.0 / config.rate_pps as f64);
        let mut ticker = interval(pkt_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = ticker.tick() => {
                    if start.elapsed() >= config.duration {
                        break;
                    }
                    let size = next_packet_size(config.pattern, config.packet_size, seq);
                    let (ts_sec, ts_nsec) = clock_now();
                    let header = PacketHeader {
                        stream_id: config.stream_id,
                        flags: 0,
                        seq,
                        ts_sec,
                        ts_nsec,
                        payload_len: 0,
                    };
                    let pkt = build_packet(&header, size);
                    match socket.send(&pkt).await {
                        Ok(n) => {
                            packets_sent += 1;
                            bytes_sent += n as u64;
                        }
                        Err(_) if cancel.is_cancelled() => break,
                        Err(e) => return Err(e),
                    }
                    seq = seq.wrapping_add(1);
                }
            }
        }
    }

    // Send final packet with FLAG_LAST
    let (ts_sec, ts_nsec) = clock_now();
    let header = PacketHeader {
        stream_id: config.stream_id,
        flags: FLAG_LAST,
        seq,
        ts_sec,
        ts_nsec,
        payload_len: 0,
    };
    let pkt = build_packet(&header, 32);
    let _ = socket.send(&pkt).await;

    Ok(UdpGeneratorResult {
        stream_id: config.stream_id,
        packets_sent,
        bytes_sent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_pattern_size() {
        assert_eq!(next_packet_size(TrafficPattern::Fixed, 512, 0), 512);
        assert_eq!(next_packet_size(TrafficPattern::Fixed, 512, 100), 512);
    }

    #[test]
    fn imix_pattern_ratios() {
        // 7:4:1 = 12 packets per cycle
        // 0-6: 64 bytes, 7-10: 594 bytes, 11: 1518 bytes
        assert_eq!(next_packet_size(TrafficPattern::Imix, 0, 0), 64);
        assert_eq!(next_packet_size(TrafficPattern::Imix, 0, 6), 64);
        assert_eq!(next_packet_size(TrafficPattern::Imix, 0, 7), 594);
        assert_eq!(next_packet_size(TrafficPattern::Imix, 0, 10), 594);
        assert_eq!(next_packet_size(TrafficPattern::Imix, 0, 11), 1518);
        // Cycle repeats
        assert_eq!(next_packet_size(TrafficPattern::Imix, 0, 12), 64);
    }

    #[test]
    fn sweep_pattern_cycles() {
        assert_eq!(next_packet_size(TrafficPattern::Sweep, 0, 0), 64);
        assert_eq!(next_packet_size(TrafficPattern::Sweep, 0, 7), 1518);
        assert_eq!(next_packet_size(TrafficPattern::Sweep, 0, 8), 64); // wraps
    }
}
