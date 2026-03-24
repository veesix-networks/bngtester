// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use bngtester::protocol::clock::{mono_now_ns, ClockMode};
use bngtester::protocol::session::{HeartbeatTracker, HEARTBEAT_INTERVAL};
use bngtester::protocol::{
    self, ClockSyncMsg, ErrorMsg, Message, PortAssignment, ReadyMsg, ResultsMsg, SessionStatus,
    StreamResult, StreamStatus,
};
use bngtester::report::json::{write_combined_json, write_json};
use bngtester::report::junit::{write_combined_junit, write_junit};
use bngtester::report::text::{write_combined_text, write_text};
use bngtester::report::{
    ClientReport, CombinedReport, HistogramReport, StreamConfigReport, StreamReport,
    StreamResults, TestConfig, TestReport, Thresholds,
};
use bngtester::stream::config::StreamOverrides;

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

    /// Combined mode: collect results from multiple clients into one report
    #[arg(long)]
    combined: bool,

    /// Maximum number of client sessions (combined mode only)
    #[arg(long, default_value = "1")]
    max_clients: usize,

    /// Timeout in seconds waiting for all clients (combined mode only)
    #[arg(long, default_value = "300")]
    timeout: u64,
}

/// Server configuration extracted from CLI args.
#[allow(dead_code)]
struct ServerConfig {
    listen: SocketAddr,
    output: String,
    file: Option<String>,
    raw_file: Option<String>,
    thresholds: Thresholds,
    histogram_buckets: Option<String>,
    combined: bool,
    max_clients: usize,
    timeout_secs: u64,
}

/// Registry of completed sessions for combined reporting.
#[derive(Clone)]
struct SessionRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

struct RegistryInner {
    completed: Vec<CompletedSession>,
    used_ids: HashMap<String, usize>,
}

struct CompletedSession {
    client_id: String,
    peer: SocketAddr,
    report: TestReport,
}

impl SessionRegistry {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryInner {
                completed: Vec::new(),
                used_ids: HashMap::new(),
            })),
        }
    }

    async fn register(&self, session: CompletedSession) {
        let mut inner = self.inner.lock().await;
        inner.completed.push(session);
    }

    async fn assign_unique_id(&self, raw_id: &str) -> String {
        let mut inner = self.inner.lock().await;
        let count = inner.used_ids.entry(raw_id.to_string()).or_insert(0);
        let id = if *count == 0 {
            raw_id.to_string()
        } else {
            format!("{raw_id}-{count}")
        };
        *count += 1;
        id
    }

    async fn count(&self) -> usize {
        self.inner.lock().await.completed.len()
    }

    async fn take_all(self) -> Vec<CompletedSession> {
        let mut inner = self.inner.lock().await;
        std::mem::take(&mut inner.completed)
    }
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

    let config = Arc::new(ServerConfig {
        listen: cli.listen,
        output: cli.output,
        file: cli.file,
        raw_file: cli.raw_file,
        thresholds,
        histogram_buckets: cli.histogram_buckets,
        combined: cli.combined,
        max_clients: cli.max_clients,
        timeout_secs: cli.timeout,
    });

    eprintln!("bngtester-server: listening on {}", config.listen);
    let listener = TcpListener::bind(config.listen).await.unwrap_or_else(|e| {
        eprintln!("bngtester-server: failed to bind {}: {e}", config.listen);
        std::process::exit(1);
    });

    if config.combined {
        run_combined_mode(&listener, &config).await;
    } else {
        run_per_session_mode(&listener, &config).await;
    }
}

/// Per-session mode: each session writes its own report, stdout serialized via writer lock.
async fn run_per_session_mode(listener: &TcpListener, config: &Arc<ServerConfig>) {
    let writer_lock = Arc::new(Mutex::new(()));
    let mut join_set = JoinSet::new();

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("bngtester-server: accept error: {e}");
                continue;
            }
        };
        eprintln!("bngtester-server: client connected from {peer}");

        let cfg = config.clone();
        let wl = writer_lock.clone();
        let registry = SessionRegistry::new();

        join_set.spawn(async move {
            let result = handle_session(stream, peer, cfg.clone(), registry).await;
            match result {
                Ok(completed) => {
                    eprintln!("bngtester-server: session with {peer} complete");
                    write_per_session_report(&cfg, &wl, &completed).await;
                }
                Err(e) => {
                    eprintln!("bngtester-server: session with {peer} failed: {e}");
                }
            }
        });

        // Reap finished tasks without blocking
        while let Some(res) = join_set.try_join_next() {
            if let Err(e) = res {
                eprintln!("bngtester-server: task panic: {e}");
            }
        }
    }
}

/// Write a single session's report, using the writer lock for stdout serialization.
async fn write_per_session_report(
    config: &ServerConfig,
    writer_lock: &Mutex<()>,
    session: &CompletedSession,
) {
    let _guard = writer_lock.lock().await;

    let output_result: Result<(), Box<dyn std::error::Error + Send + Sync>> = (|| {
        let mut output: Box<dyn std::io::Write> = if let Some(ref base_path) = config.file {
            let path = per_session_file_path(base_path, &session.client_id);
            Box::new(std::fs::File::create(&path).map_err(|e| {
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?)
        } else {
            Box::new(std::io::stdout())
        };

        match config.output.as_str() {
            "json" => write_json(&mut output, &session.report)?,
            "junit" => write_junit(&mut output, &session.report, &config.thresholds)?,
            "text" => write_text(&mut output, &session.report)?,
            other => {
                eprintln!("bngtester-server: unknown output format: {other}");
            }
        }
        Ok(())
    })();

    if let Err(e) = output_result {
        eprintln!(
            "bngtester-server: failed to write report for {}: {e}",
            session.client_id
        );
    }
}

/// Generate per-session file path: {base}-{client_id}.{ext}
fn per_session_file_path(base_path: &str, client_id: &str) -> PathBuf {
    let path = PathBuf::from(base_path);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "report".to_string());
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "txt".to_string());
    let safe_id = client_id.replace(':', "_");
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    parent.join(format!("{stem}-{safe_id}.{ext}"))
}

/// Combined mode: accept up to max_clients sessions, then write a single combined report.
async fn run_combined_mode(listener: &TcpListener, config: &Arc<ServerConfig>) {
    let registry = SessionRegistry::new();
    let mut join_set = JoinSet::new();
    let timeout = Duration::from_secs(config.timeout_secs);

    eprintln!(
        "bngtester-server: combined mode, waiting for {} clients (timeout {}s)",
        config.max_clients, config.timeout_secs
    );

    let accept_deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = accept_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            eprintln!("bngtester-server: timeout reached waiting for clients");
            break;
        }

        // Check if we have enough completed sessions
        if registry.count().await >= config.max_clients {
            break;
        }

        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer)) => {
                        eprintln!("bngtester-server: client connected from {peer}");
                        let cfg = config.clone();
                        let reg = registry.clone();
                        join_set.spawn(async move {
                            match handle_session(stream, peer, cfg, reg).await {
                                Ok(completed) => {
                                    eprintln!("bngtester-server: session with {peer} complete");
                                    Some(completed)
                                }
                                Err(e) => {
                                    eprintln!("bngtester-server: session with {peer} failed: {e}");
                                    None
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("bngtester-server: accept error: {e}");
                    }
                }
            }
            Some(result) = join_set.join_next() => {
                match result {
                    Ok(Some(completed)) => {
                        registry.register(completed).await;
                        let count = registry.count().await;
                        eprintln!(
                            "bngtester-server: {}/{} sessions complete",
                            count, config.max_clients
                        );
                        if count >= config.max_clients {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("bngtester-server: task panic: {e}");
                    }
                }
            }
            _ = tokio::time::sleep(remaining) => {
                eprintln!("bngtester-server: timeout reached waiting for clients");
                break;
            }
        }
    }

    // Wait for any remaining in-progress sessions (brief grace)
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Some(completed)) => {
                registry.register(completed).await;
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("bngtester-server: task panic: {e}");
            }
        }
    }

    let sessions = registry.take_all().await;
    let total = sessions.len();

    if total == 0 {
        eprintln!("bngtester-server: no sessions completed, nothing to report");
        return;
    }

    let combined = CombinedReport {
        combined: true,
        total_clients: total,
        clients: sessions
            .into_iter()
            .map(|s| ClientReport {
                client_id: s.client_id,
                peer: s.peer.to_string(),
                report: s.report,
            })
            .collect(),
    };

    let output_result: Result<(), Box<dyn std::error::Error>> = (|| {
        let mut output: Box<dyn std::io::Write> = if let Some(ref path) = config.file {
            Box::new(std::fs::File::create(path)?)
        } else {
            Box::new(std::io::stdout())
        };

        match config.output.as_str() {
            "json" => write_combined_json(&mut output, &combined)?,
            "junit" => write_combined_junit(&mut output, &combined, &config.thresholds)?,
            "text" => write_combined_text(&mut output, &combined)?,
            other => {
                eprintln!("bngtester-server: unknown output format: {other}");
            }
        }
        Ok(())
    })();

    if let Err(e) = output_result {
        eprintln!("bngtester-server: failed to write combined report: {e}");
    }
}

async fn handle_session(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    config: Arc<ServerConfig>,
    registry: SessionRegistry,
) -> Result<CompletedSession, Box<dyn std::error::Error + Send + Sync>> {
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

    // Determine client_id: use provided value or default to peer address
    let raw_id = hello
        .client_id
        .clone()
        .unwrap_or_else(|| format!("{}", peer));
    let client_id = registry.assign_unique_id(&raw_id).await;

    eprintln!(
        "bngtester-server: [{}] test config: mode={:?} duration={}s streams={}",
        client_id, hello.mode, hello.duration_secs, hello.streams_per_direction
    );

    // --- Parse ECN config from hello ---
    let ecn_requested = hello.ecn.is_some();
    let ecn_mode_name = hello.ecn.clone();

    // --- Allocate UDP receiver port ---
    let udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
    let udp_port = udp_socket.local_addr()?.port();

    if ecn_requested {
        use std::os::unix::io::AsRawFd;
        let fd = udp_socket.as_raw_fd();
        bngtester::dscp::enable_recv_tos(fd)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::other(e))
            })?;
        eprintln!("bngtester-server: [{}] IP_RECVTOS enabled for ECN tracking", client_id);
    }

    // --- Send Ready with port assignments ---
    let tcp_ports: Vec<PortAssignment> = Vec::new();
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

    eprintln!(
        "bngtester-server: [{}] test started, receiving packets on UDP port {udp_port}",
        client_id
    );

    // --- Run receiver + heartbeat ---
    let recv_cancel = cancel.clone();
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
            packets_reordered: jitter.count(),
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

    // Heartbeat loop
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
                        eprintln!("bngtester-server: [{}] received stop", client_id);
                        cancel.cancel();
                        session_status = SessionStatus::Complete;
                        break;
                    }
                    Ok(None) => {
                        eprintln!("bngtester-server: [{}] client disconnected", client_id);
                        cancel.cancel();
                        session_status = SessionStatus::Interrupted;
                        break;
                    }
                    Ok(Some(_)) => {}
                    Err(e) => {
                        eprintln!("bngtester-server: [{}] control error: {e}", client_id);
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
                    eprintln!("bngtester-server: [{}] heartbeat timeout", client_id);
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
            server: config.listen.to_string(),
        },
        streams: {
            let mut stream_overrides = StreamOverrides::default();
            for sc in &hello.stream_config {
                if let Some(size) = sc.size {
                    stream_overrides.sizes.push((sc.stream_id, size));
                }
                if let Some(rate) = sc.rate_pps {
                    stream_overrides.rates.push((sc.stream_id, rate));
                }
                if let Some(pat) = sc.pattern {
                    stream_overrides.patterns.push((sc.stream_id, pat));
                }
                if let Some(dscp) = sc.dscp {
                    stream_overrides.dscps.push((sc.stream_id, dscp));
                }
            }
            let s0_resolved = stream_overrides.resolve(
                0,
                hello.packet_size,
                hello.rate_pps,
                hello.pattern,
                hello.dscp,
            );
            let stream_config_report = if stream_overrides.has_overrides(0) {
                Some(StreamConfigReport {
                    size: s0_resolved.size,
                    rate_pps: s0_resolved.rate_pps,
                    pattern: format!("{:?}", s0_resolved.pattern).to_lowercase(),
                })
            } else {
                None
            };

            vec![StreamReport {
                id: stream_result.stream_id,
                stream_type: "udp_latency".to_string(),
                direction: "upstream".to_string(),
                status: stream_result.status,
                dscp: s0_resolved.dscp,
                dscp_name: s0_resolved.dscp.map(bngtester::dscp::dscp_name),
                ecn_mode: ecn_mode_name.clone(),
                config: stream_config_report,
                results: StreamResults {
                    packets_sent: None,
                    packets_received: Some(stream_result.packets_received),
                    packets_lost: Some(stream_result.packets_lost),
                    loss_percent: Some(
                        if stream_result.packets_received + stream_result.packets_lost > 0 {
                            stream_result.packets_lost as f64
                                / (stream_result.packets_received + stream_result.packets_lost)
                                    as f64
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
                            Some(
                                stream_result.ecn_ce.unwrap_or(0) as f64 / total as f64 * 100.0,
                            )
                        } else {
                            None
                        }
                    },
                },
            }]
        },
        bufferbloat: None,
        time_series: ts,
        histogram: Some(hist_report),
    };

    Ok(CompletedSession {
        client_id,
        peer,
        report,
    })
}
