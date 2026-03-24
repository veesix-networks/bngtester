// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use bngtester::protocol::clock::{mono_now_ns, ClockMode};
use bngtester::protocol::session::{HeartbeatTracker, HEARTBEAT_INTERVAL};
use bngtester::protocol::{
    self, ClockSyncMsg, ErrorMsg, Message, PortAssignment, ReadyMsg, ResultsMsg, SessionStatus,
    StreamResult, StreamStatus,
};
use bngtester::report::json::write_json;
use bngtester::report::junit::write_junit;
use bngtester::report::text::write_text;
use bngtester::report::{
    HistogramReport, StreamReport, StreamResults, TestConfig, TestReport, Thresholds,
};

#[derive(Parser)]
#[command(name = "bngtester-server", about = "BNG test traffic receiver and measurement server")]
struct Cli {
    /// Listen address for control channel
    #[arg(short, long, default_value = "0.0.0.0:5000")]
    listen: SocketAddr,

    /// Output format: json, junit, text
    #[arg(short, long, default_value = "text")]
    output: String,

    /// Write report to file (default: stdout)
    #[arg(short, long)]
    file: Option<String>,

    /// Write per-packet JSONL data to file
    #[arg(long)]
    raw_file: Option<String>,

    /// JUnit pass/fail threshold (repeatable). Format: key=value
    #[arg(long = "threshold", value_name = "KEY=VAL")]
    thresholds: Vec<String>,

    /// Latency histogram bucket specification
    #[arg(long)]
    histogram_buckets: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut thresholds = Thresholds::default();
    for t in &cli.thresholds {
        if let Err(e) = thresholds.parse_threshold(t) {
            eprintln!("bngtester-server: {e}");
            std::process::exit(1);
        }
    }

    eprintln!("bngtester-server: listening on {}", cli.listen);
    let listener = TcpListener::bind(cli.listen).await.unwrap_or_else(|e| {
        eprintln!("bngtester-server: failed to bind {}: {e}", cli.listen);
        std::process::exit(1);
    });

    // Accept one client session at a time
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("bngtester-server: accept error: {e}");
                continue;
            }
        };
        eprintln!("bngtester-server: client connected from {peer}");

        let result = handle_session(stream, peer, &cli, &thresholds).await;
        match result {
            Ok(()) => eprintln!("bngtester-server: session with {peer} complete"),
            Err(e) => eprintln!("bngtester-server: session with {peer} failed: {e}"),
        }
    }
}

async fn handle_session(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    cli: &Cli,
    thresholds: &Thresholds,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut reader, mut writer) = stream.into_split();
    let cancel = CancellationToken::new();

    // --- Hello ---
    let hello = match protocol::read_message(&mut reader).await? {
        Some(Message::Hello(h)) => h,
        Some(other) => {
            let msg = Message::Error(ErrorMsg {
                reason: format!("expected hello, got {:?}", std::mem::discriminant(&other)),
            });
            protocol::write_message(&mut writer, &msg).await?;
            return Err("protocol error: expected hello".into());
        }
        None => return Err("client disconnected before hello".into()),
    };

    eprintln!(
        "bngtester-server: test config: mode={:?} duration={}s streams={}",
        hello.mode, hello.duration_secs, hello.streams_per_direction
    );

    // --- Parse ECN config from hello ---
    let ecn_requested = hello.ecn.is_some();
    let ecn_mode_name = hello.ecn.clone();

    // --- Allocate UDP receiver port ---
    // Pre-bind UDP socket so we know the port before sending Ready
    let udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
    let udp_port = udp_socket.local_addr()?.port();

    // Enable IP_RECVTOS if ECN was requested
    if ecn_requested {
        use std::os::unix::io::AsRawFd;
        let fd = udp_socket.as_raw_fd();
        bngtester::dscp::enable_recv_tos(fd)
            .map_err(|e| -> Box<dyn std::error::Error> {
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
            })?;
        eprintln!("bngtester-server: IP_RECVTOS enabled for ECN tracking");
    }

    // --- Send Ready with port assignments ---
    // For now, TCP throughput streams connect directly to ephemeral ports.
    // The server will accept them after start.
    let tcp_ports: Vec<PortAssignment> = Vec::new(); // TCP ports allocated on-demand

    let ready = Message::Ready(ReadyMsg {
        udp_port,
        tcp_ports,
    });
    protocol::write_message(&mut writer, &ready).await?;

    // --- Clock sync (if cross-host) ---
    let clock_mode = if hello.cross_host {
        let mut _samples: Vec<bngtester::protocol::clock::ClockSample> = Vec::new();
        for _ in 0..bngtester::protocol::clock::sync_rounds() {
            match protocol::read_message(&mut reader).await? {
                Some(Message::ClockSync(cs)) => {
                    let server_recv = mono_now_ns();
                    let server_send = mono_now_ns();
                    let reply = Message::ClockSync(ClockSyncMsg {
                        client_send_ns: cs.client_send_ns,
                        server_recv_ns: Some(server_recv),
                        server_send_ns: Some(server_send),
                    });
                    protocol::write_message(&mut writer, &reply).await?;
                    // We don't compute offset server-side; client does it
                }
                _ => break,
            }
        }
        ClockMode::SyncEstimated { offset_ns: 0 }
    } else {
        ClockMode::SameHost
    };

    // --- Wait for Start ---
    match protocol::read_message(&mut reader).await? {
        Some(Message::Start(_)) => {}
        Some(other) => {
            return Err(format!(
                "expected start, got {:?}",
                std::mem::discriminant(&other)
            )
            .into());
        }
        None => return Err("client disconnected before start".into()),
    }

    eprintln!("bngtester-server: test started, receiving packets on UDP port {udp_port}");

    // --- Run receiver + heartbeat ---
    let recv_cancel = cancel.clone();

    // Collect metrics from UDP receiver using pre-bound socket
    let metrics = Arc::new(Mutex::new(None));
    let metrics_clone = metrics.clone();

    let recv_handle = tokio::spawn(async move {
        let mut latency = bngtester::metrics::latency::LatencyCollector::new();
        let mut histogram = bngtester::metrics::latency::LatencyHistogram::default_buckets();
        let mut loss = bngtester::metrics::loss::LossTracker::new();
        let mut jitter = bngtester::metrics::jitter::JitterTracker::new();
        let mut throughput = bngtester::metrics::throughput::ThroughputTracker::new();
        let mut timeseries = bngtester::metrics::timeseries::TimeSeriesCollector::new();
        let mut ecn_counters = bngtester::dscp::EcnCounters::default();

        let mut buf = vec![0u8; 65536];

        if ecn_requested {
            // ECN-aware receive path using recvmsg
            use std::os::unix::io::AsRawFd;
            use tokio::io::Interest;
            let fd = udp_socket.as_raw_fd();

            loop {
                tokio::select! {
                    _ = recv_cancel.cancelled() => break,
                    result = udp_socket.readable() => {
                        if result.is_err() {
                            break;
                        }
                        match udp_socket.try_io(Interest::READABLE, || {
                            bngtester::dscp::recvmsg_with_tos(fd, &mut buf)
                        }) {
                            Ok((n, tos_byte)) => {
                                if n < bngtester::traffic::packet::HEADER_SIZE {
                                    continue;
                                }
                                let header = match bngtester::traffic::packet::PacketHeader::read_from(&buf[..n]) {
                                    Some(h) => h,
                                    None => continue,
                                };

                                // Track ECN codepoint
                                match tos_byte {
                                    Some(tos) => {
                                        let cp = bngtester::dscp::EcnCodepoint::from_tos(tos);
                                        ecn_counters.record(cp);
                                    }
                                    None => {
                                        ecn_counters.record_unknown();
                                    }
                                }

                                let (recv_sec, recv_nsec) = bngtester::traffic::packet::clock_now();
                                let recv_ns = recv_sec as u128 * 1_000_000_000 + recv_nsec as u128;
                                let send_ns = header.timestamp_ns();

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
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                            Err(_) if recv_cancel.is_cancelled() => break,
                            Err(e) => {
                                eprintln!("bngtester-server: recv error: {e}");
                                break;
                            }
                        }
                    }
                }
            }
        } else {
            // Standard receive path (no ECN tracking)
            loop {
                tokio::select! {
                    _ = recv_cancel.cancelled() => break,
                    result = udp_socket.recv_from(&mut buf) => {
                        match result {
                            Ok((n, _src)) => {
                                if n < bngtester::traffic::packet::HEADER_SIZE {
                                    continue;
                                }
                                let header = match bngtester::traffic::packet::PacketHeader::read_from(&buf[..n]) {
                                    Some(h) => h,
                                    None => continue,
                                };

                                let (recv_sec, recv_nsec) = bngtester::traffic::packet::clock_now();
                                let recv_ns = recv_sec as u128 * 1_000_000_000 + recv_nsec as u128;
                                let send_ns = header.timestamp_ns();

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
                            Err(_) if recv_cancel.is_cancelled() => break,
                            Err(e) => {
                                eprintln!("bngtester-server: recv error: {e}");
                                break;
                            }
                        }
                    }
                }
            }
        }

        let latency_stats = latency.stats().map(|s| bngtester::protocol::LatencyStats {
            min: s.min / 1_000.0,
            avg: s.avg / 1_000.0,
            max: s.max / 1_000.0,
            p50: s.p50 / 1_000.0,
            p95: s.p95 / 1_000.0,
            p99: s.p99 / 1_000.0,
            p999: s.p999 / 1_000.0,
        });

        let (ecn_not_ect, ecn_ect0, ecn_ect1, ecn_ce) = if ecn_requested {
            (Some(ecn_counters.not_ect), Some(ecn_counters.ect0), Some(ecn_counters.ect1), Some(ecn_counters.ce))
        } else {
            (None, None, None, None)
        };

        let stream_result = StreamResult {
            stream_id: 0,
            status: StreamStatus::Complete,
            packets_received: loss.received(),
            packets_lost: loss.estimated_lost(),
            packets_reordered: jitter.count(), // placeholder
            latency_ns: latency_stats,
            jitter_ns: Some(jitter.jitter_ns()),
            throughput_bps: throughput.bits_per_sec(),
            throughput_pps: throughput.packets_per_sec(),
            tcp_info: None,
            ecn_not_ect,
            ecn_ect0,
            ecn_ect1,
            ecn_ce,
        };

        let hist_report = HistogramReport {
            bucket_us: histogram.boundaries_us(),
            counts: histogram.counts.clone(),
        };

        let ts = timeseries.finalize();

        *metrics_clone.lock().await = Some((stream_result, hist_report, ts));
    });

    // Heartbeat loop — also watch for stop
    let mut heartbeat = HeartbeatTracker::new();
    let session_status;

    loop {
        tokio::select! {
            msg = protocol::read_message(&mut reader) => {
                match msg {
                    Ok(Some(Message::Heartbeat)) => {
                        heartbeat.received();
                    }
                    Ok(Some(Message::Stop)) => {
                        eprintln!("bngtester-server: received stop");
                        cancel.cancel();
                        session_status = SessionStatus::Complete;
                        break;
                    }
                    Ok(None) => {
                        eprintln!("bngtester-server: client disconnected");
                        cancel.cancel();
                        session_status = SessionStatus::Interrupted;
                        break;
                    }
                    Ok(Some(_)) => {} // ignore unexpected messages
                    Err(e) => {
                        eprintln!("bngtester-server: control error: {e}");
                        cancel.cancel();
                        session_status = SessionStatus::Interrupted;
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(HEARTBEAT_INTERVAL) => {
                let hb = Message::Heartbeat;
                if protocol::write_message(&mut writer, &hb).await.is_err() {
                    cancel.cancel();
                    session_status = SessionStatus::Interrupted;
                    break;
                }
                heartbeat.sent();

                if heartbeat.is_timed_out() {
                    eprintln!("bngtester-server: heartbeat timeout");
                    cancel.cancel();
                    session_status = SessionStatus::Interrupted;
                    break;
                }
            }
        }
    }

    // Wait for receiver to finish
    let _ = recv_handle.await;

    // Build results
    let guard = metrics.lock().await;
    let (stream_result, hist_report, ts) = match guard.as_ref() {
        Some(v) => v.clone(),
        None => {
            return Err("no metrics collected".into());
        }
    };
    drop(guard);

    // Send results to client
    let results_msg = Message::Results(ResultsMsg {
        status: session_status,
        streams: vec![stream_result.clone()],
    });
    let _ = protocol::write_message(&mut writer, &results_msg).await;

    // Build local report
    let report = TestReport {
        status: session_status,
        clock_mode: clock_mode.name().to_string(),
        test: TestConfig {
            mode: hello.mode,
            duration_secs: hello.duration_secs,
            client: peer.to_string(),
            server: cli.listen.to_string(),
        },
        streams: {
        // Resolve DSCP from hello config for stream 0
        let stream_dscp_overrides: Vec<(u8, u8)> = hello.stream_dscp.iter()
            .map(|sc| (sc.stream_id, sc.dscp))
            .collect();
        let s0_dscp = bngtester::dscp::resolve_stream_dscp(0, hello.dscp, &stream_dscp_overrides);

        vec![StreamReport {
            id: stream_result.stream_id,
            stream_type: "udp_latency".to_string(),
            direction: "upstream".to_string(),
            status: stream_result.status,
            dscp: s0_dscp,
            dscp_name: s0_dscp.map(bngtester::dscp::dscp_name),
            ecn_mode: ecn_mode_name.clone(),
            results: StreamResults {
                packets_sent: None,
                packets_received: Some(stream_result.packets_received),
                packets_lost: Some(stream_result.packets_lost),
                loss_percent: Some(
                    if stream_result.packets_received + stream_result.packets_lost > 0 {
                        stream_result.packets_lost as f64
                            / (stream_result.packets_received + stream_result.packets_lost) as f64
                            * 100.0
                    } else {
                        0.0
                    },
                ),
                packets_reordered: None,
                reorder_percent: None,
                latency_us: stream_result.latency_ns,
                jitter_us: stream_result.jitter_ns.map(|j| j / 1_000.0),
                throughput_bps: Some(stream_result.throughput_bps),
                throughput_pps: Some(stream_result.throughput_pps),
                goodput_bps: None,
                tcp_info: None,
                ecn_ect_sent: None,
                ecn_not_ect_received: stream_result.ecn_not_ect,
                ecn_ect0_received: stream_result.ecn_ect0,
                ecn_ect1_received: stream_result.ecn_ect1,
                ecn_ce_received: stream_result.ecn_ce,
                ecn_ce_ratio: {
                    let total = stream_result.ecn_not_ect.unwrap_or(0)
                        + stream_result.ecn_ect0.unwrap_or(0)
                        + stream_result.ecn_ect1.unwrap_or(0)
                        + stream_result.ecn_ce.unwrap_or(0);
                    if total > 0 {
                        Some(stream_result.ecn_ce.unwrap_or(0) as f64 / total as f64 * 100.0)
                    } else {
                        None
                    }
                },
            },
        }]},
        bufferbloat: None,
        time_series: ts,
        histogram: Some(hist_report),
    };

    // Output report
    let output: Box<dyn std::io::Write> = if let Some(ref path) = cli.file {
        Box::new(std::fs::File::create(path)?)
    } else {
        Box::new(std::io::stdout())
    };
    let mut output = output;

    match cli.output.as_str() {
        "json" => write_json(&mut output, &report)?,
        "junit" => write_junit(&mut output, &report, thresholds)?,
        "text" => write_text(&mut output, &report)?,
        other => {
            eprintln!("bngtester-server: unknown output format: {other}");
        }
    }

    Ok(())
}
