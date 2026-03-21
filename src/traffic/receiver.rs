// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::metrics::jitter::JitterTracker;
use crate::metrics::latency::{LatencyCollector, LatencyHistogram};
use crate::metrics::loss::LossTracker;
use crate::metrics::throughput::ThroughputTracker;
use crate::metrics::timeseries::TimeSeriesCollector;
use crate::protocol::clock::ClockMode;
use crate::traffic::packet::{clock_now, PacketHeader, HEADER_SIZE};

/// Result from a UDP receiver session.
pub struct UdpReceiverResult {
    pub latency: LatencyCollector,
    pub histogram: LatencyHistogram,
    pub loss: LossTracker,
    pub jitter: JitterTracker,
    pub throughput: ThroughputTracker,
    pub timeseries: TimeSeriesCollector,
}

/// Run a UDP receiver. Listens for incoming packets, measures metrics.
/// Returns when FLAG_LAST is received or cancellation is triggered.
pub async fn run_udp_receiver(
    bind_addr: SocketAddr,
    clock_mode: ClockMode,
    cancel: CancellationToken,
) -> std::io::Result<(UdpReceiverResult, u16)> {
    let socket = UdpSocket::bind(bind_addr).await?;
    let local_port = socket.local_addr()?.port();

    let mut latency = LatencyCollector::new();
    let mut histogram = LatencyHistogram::default_buckets();
    let mut loss = LossTracker::new();
    let mut jitter = JitterTracker::new();
    let mut throughput = ThroughputTracker::new();
    let mut timeseries = TimeSeriesCollector::new();

    let mut buf = vec![0u8; 65536];

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            result = socket.recv_from(&mut buf) => {
                let (n, _src) = result?;
                if n < HEADER_SIZE {
                    continue;
                }
                let header = match PacketHeader::read_from(&buf[..n]) {
                    Some(h) => h,
                    None => continue,
                };

                let (recv_sec, recv_nsec) = clock_now();
                let recv_ns = recv_sec as u128 * 1_000_000_000 + recv_nsec as u128;
                let send_ns = header.timestamp_ns();

                // One-way latency
                let raw_latency = recv_ns as i128 - send_ns as i128;
                let corrected = clock_mode.correct_latency(raw_latency);
                let latency_ns = corrected.max(0) as f64;

                latency.record(latency_ns);
                histogram.record(latency_ns);
                jitter.record(latency_ns);
                loss.record(header.seq);
                throughput.record(n as u64, recv_ns);
                timeseries.record(recv_ns, n as u64, Some(latency_ns));

                if header.is_last() {
                    break;
                }
            }
        }
    }

    Ok((
        UdpReceiverResult {
            latency,
            histogram,
            loss,
            jitter,
            throughput,
            timeseries,
        },
        local_port,
    ))
}
