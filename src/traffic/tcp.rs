// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{interval, Instant};
use tokio_util::sync::CancellationToken;

use crate::protocol::TcpStats;

/// TCP_INFO fields we care about. Matches Linux struct tcp_info layout.
#[repr(C)]
#[allow(non_camel_case_types)]
struct tcp_info_subset {
    tcpi_state: u8,
    tcpi_ca_state: u8,
    tcpi_retransmits: u8,
    tcpi_probes: u8,
    tcpi_backoff: u8,
    tcpi_options: u8,
    // bitfield: tcpi_snd_wscale : 4, tcpi_rcv_wscale : 4
    tcpi_wscale: u8,
    tcpi_delivery_rate_app_limited: u8,
    tcpi_rto: u32,
    tcpi_ato: u32,
    tcpi_snd_mss: u32,
    tcpi_rcv_mss: u32,
    tcpi_unacked: u32,
    tcpi_sacked: u32,
    tcpi_lost: u32,
    tcpi_retrans: u32,
    tcpi_fackets: u32,
    // Times
    tcpi_last_data_sent: u32,
    tcpi_last_ack_sent: u32,
    tcpi_last_data_recv: u32,
    tcpi_last_ack_recv: u32,
    // Metrics
    tcpi_pmtu: u32,
    tcpi_rcv_ssthresh: u32,
    tcpi_rtt: u32,        // smoothed RTT in usec
    tcpi_rttvar: u32,     // RTT variance in usec
    tcpi_snd_ssthresh: u32,
    tcpi_snd_cwnd: u32,   // congestion window
    tcpi_advmss: u32,
    tcpi_reordering: u32,
    tcpi_rcv_rtt: u32,
    tcpi_rcv_space: u32,
    tcpi_total_retrans: u32,
}

/// Read TCP_INFO from a connected socket.
fn get_tcp_info(fd: std::os::unix::io::RawFd) -> Option<(u32, u32, u32, u32)> {
    let mut info: tcp_info_subset = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<tcp_info_subset>() as libc::socklen_t;

    let ret = unsafe {
        libc::getsockopt(
            fd,
            libc::IPPROTO_TCP,
            libc::TCP_INFO,
            &mut info as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };

    if ret != 0 {
        return None;
    }

    // If kernel returned less data than we asked for, the fields we got are still valid
    Some((
        info.tcpi_rtt,
        info.tcpi_rttvar,
        info.tcpi_total_retrans,
        info.tcpi_snd_cwnd,
    ))
}

/// Configuration for a TCP throughput generator.
pub struct TcpGeneratorConfig {
    pub target: SocketAddr,
    pub stream_id: u8,
    pub duration: Duration,
    pub connect_timeout: Duration,
    pub tos: Option<u8>,
    pub bind_iface: Option<String>,
    pub source_ip: Option<std::net::IpAddr>,
}

/// Result from a completed TCP generator run.
pub struct TcpGeneratorResult {
    pub stream_id: u8,
    pub bytes_sent: u64,
    pub tcp_stats: Option<TcpStats>,
}

/// Run a TCP throughput generator. Sends data as fast as possible and polls TCP_INFO.
pub async fn run_tcp_generator(
    config: TcpGeneratorConfig,
    cancel: CancellationToken,
) -> std::io::Result<TcpGeneratorResult> {
    let needs_socket2 = config.tos.is_some()
        || config.source_ip.is_some()
        || config.bind_iface.is_some();

    let stream = if needs_socket2 {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;
        crate::socket::setup_socket(
            &sock,
            config.bind_iface.as_deref(),
            config.source_ip,
            config.tos,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        sock.set_nonblocking(true)?;
        let addr = socket2::SockAddr::from(config.target);
        match sock.connect(&addr) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EINPROGRESS) => {}
            Err(e) => return Err(e.into()),
        }
        let std_stream: std::net::TcpStream = sock.into();
        let tokio_stream = TcpStream::from_std(std_stream)?;
        tokio::time::timeout(config.connect_timeout, tokio_stream.writable())
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "TCP connect timeout"))??;
        tokio_stream
    } else {
        tokio::time::timeout(config.connect_timeout, TcpStream::connect(config.target))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "TCP connect timeout"))??
    };

    let fd = stream.as_raw_fd();
    let buf = vec![0u8; 65536]; // 64KB write buffer

    let mut bytes_sent: u64 = 0;
    let mut rtt_min = u32::MAX;
    let mut rtt_max = 0u32;
    let mut rtt_sum = 0u64;
    let mut rtt_count = 0u64;
    let mut cwnd_max = 0u32;
    let mut total_retrans = 0u32;

    let start = Instant::now();

    // Poll TCP_INFO every 100ms
    let mut info_ticker = interval(Duration::from_millis(100));
    info_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let (_reader, mut writer) = stream.into_split();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = info_ticker.tick() => {
                if start.elapsed() >= config.duration {
                    break;
                }
                // Poll TCP_INFO
                if let Some((rtt, _rttvar, retrans, cwnd)) = get_tcp_info(fd) {
                    if rtt > 0 {
                        rtt_min = rtt_min.min(rtt);
                        rtt_max = rtt_max.max(rtt);
                        rtt_sum += rtt as u64;
                        rtt_count += 1;
                    }
                    cwnd_max = cwnd_max.max(cwnd);
                    total_retrans = retrans;
                }
            }
            result = writer.write(&buf) => {
                match result {
                    Ok(n) => bytes_sent += n as u64,
                    Err(_) if cancel.is_cancelled() => break,
                    Err(e) => return Err(e),
                }
            }
        }
    }

    let tcp_stats = if rtt_count > 0 {
        Some(TcpStats {
            rtt_us: rtt_sum as f64 / rtt_count as f64,
            rtt_var_us: 0.0, // Simplified — could track variance separately
            retransmissions: total_retrans,
            cwnd_max,
        })
    } else {
        None
    };

    Ok(TcpGeneratorResult {
        stream_id: config.stream_id,
        bytes_sent,
        tcp_stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_info_on_invalid_fd_returns_none() {
        // fd -1 should fail
        assert!(get_tcp_info(-1).is_none());
    }
}
