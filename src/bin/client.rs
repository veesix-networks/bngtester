// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::time::Duration;

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};
use tokio::net::TcpStream;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use bngtester::config::{load_client_config, ClientConfig};
use bngtester::protocol::clock::{mono_now_ns, ClockMode, ClockSample};
use bngtester::protocol::{
    self, ClockSyncMsg, HelloMsg, Message, Protocol, SessionStatus, StartMsg,
    StreamConfigOverride, TestMode, TrafficPattern,
};
use bngtester::report::json::write_json;
use bngtester::report::junit::write_junit;
use bngtester::report::text::write_text;
use bngtester::report::{
    StreamConfigReport, StreamReport, StreamResults, TestConfig, TestReport, Thresholds,
};
use bngtester::stream::config::StreamOverrides;
use bngtester::traffic::generator::{run_udp_generator, UdpGeneratorConfig};

#[derive(Parser)]
#[command(name = "bngtester-client", about = "BNG test traffic generator")]
struct Cli {
    /// Server address (host:port)
    server: Option<String>,

    /// YAML config file path
    #[arg(long)]
    config: Option<String>,

    /// Test mode: throughput, latency, rrul, bidirectional
    #[arg(short, long, default_value = "latency")]
    mode: String,

    /// Protocol for throughput: tcp, udp
    #[arg(short, long, default_value = "tcp")]
    protocol: String,

    /// Packet size in bytes
    #[arg(short, long, default_value = "512")]
    size: usize,

    /// Latency probe rate, packets/sec
    #[arg(short, long, default_value = "100")]
    rate: u32,

    /// Test duration in seconds
    #[arg(short, long, default_value = "30")]
    duration: u32,

    /// Traffic pattern: fixed, imix, sweep
    #[arg(short = 'P', long, default_value = "fixed")]
    pattern: String,

    /// RRUL baseline phase duration in seconds
    #[arg(long, default_value = "5")]
    rrul_baseline: u32,

    /// Delay between TCP stream starts in RRUL (ms)
    #[arg(long, default_value = "100")]
    rrul_ramp_up: u32,

    /// Number of throughput streams per direction
    #[arg(long, default_value = "2")]
    streams: u32,

    /// Use clock offset estimation (for cross-host testing)
    #[arg(long)]
    cross_host: bool,

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

    /// DSCP codepoint for all data streams (e.g., EF, AF41, CS6, 46)
    #[arg(long)]
    dscp: Option<String>,

    /// Per-stream DSCP override (repeatable). Format: ID=DSCP (e.g., 0=AF41)
    #[arg(long = "stream-dscp", value_name = "ID=DSCP")]
    stream_dscp: Vec<String>,

    /// Per-stream packet size override (repeatable). Format: ID=BYTES (e.g., 0=64)
    #[arg(long = "stream-size", value_name = "ID=BYTES")]
    stream_size: Vec<String>,

    /// Per-stream rate override (repeatable). Format: ID=PPS (e.g., 0=10000, 0=0 for unlimited)
    #[arg(long = "stream-rate", value_name = "ID=PPS")]
    stream_rate: Vec<String>,

    /// Per-stream traffic pattern override (repeatable). Format: ID=PATTERN (e.g., 0=imix)
    #[arg(long = "stream-pattern", value_name = "ID=PATTERN")]
    stream_pattern: Vec<String>,

    /// ECN mode for outgoing packets (ect0 or ect1)
    #[arg(long)]
    ecn: Option<String>,

    /// Client identifier for multi-subscriber coordination
    #[arg(long)]
    client_id: Option<String>,

    /// Bind data sockets to a specific interface via SO_BINDTODEVICE
    #[arg(long)]
    bind_iface: Option<String>,

    /// Bind data sockets to a specific source IP
    #[arg(long)]
    source_ip: Option<std::net::IpAddr>,

    /// Bind control channel TCP to a specific source IP
    #[arg(long)]
    control_bind_ip: Option<std::net::IpAddr>,
}

/// Resolved client configuration after merging CLI and config file.
#[allow(dead_code)]
struct ResolvedClient {
    server: SocketAddr,
    mode: String,
    protocol: String,
    size: usize,
    rate: u32,
    duration: u32,
    pattern: String,
    rrul_baseline: u32,
    rrul_ramp_up: u32,
    streams: u32,
    cross_host: bool,
    output: String,
    file: Option<String>,
    raw_file: Option<String>,
    dscp: Option<String>,
    ecn: Option<String>,
    client_id: Option<String>,
    bind_iface: Option<String>,
    source_ip: Option<std::net::IpAddr>,
    control_bind_ip: Option<std::net::IpAddr>,
    thresholds: Thresholds,
    stream_overrides: StreamOverrides,
}

fn was_cli_provided(matches: &ArgMatches, field: &str) -> bool {
    matches.value_source(field) == Some(clap::parser::ValueSource::CommandLine)
}

fn merge_value<T>(cli_val: T, config_val: Option<T>, matches: &ArgMatches, field: &str) -> T {
    if was_cli_provided(matches, field) {
        cli_val
    } else {
        config_val.unwrap_or(cli_val)
    }
}

fn merge_option(
    cli_val: Option<String>,
    config_val: Option<String>,
    matches: &ArgMatches,
    field: &str,
) -> Option<String> {
    if was_cli_provided(matches, field) {
        cli_val
    } else {
        config_val.or(cli_val)
    }
}

fn resolve_client(cli: Cli, matches: &ArgMatches, cfg: Option<ClientConfig>) -> ResolvedClient {
    let cfg = cfg.unwrap_or_default();

    // Resolve server: CLI positional > config > error
    let server_str = if was_cli_provided(matches, "server") {
        cli.server.clone()
    } else {
        cfg.server.clone().or(cli.server.clone())
    };
    let server_str = match server_str {
        Some(s) => s,
        None => {
            eprintln!("bngtester-client: server address required (positional arg or config file 'server' field)");
            std::process::exit(1);
        }
    };
    let server: SocketAddr = match server_str.parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("bngtester-client: invalid server address '{server_str}': {e}");
            std::process::exit(1);
        }
    };

    let mode = merge_value(cli.mode, cfg.mode, matches, "mode");
    let protocol = merge_value(cli.protocol, cfg.protocol, matches, "protocol");
    let size = merge_value(
        cli.size,
        cfg.size.map(|s| s as usize),
        matches,
        "size",
    );
    let rate = merge_value(cli.rate, cfg.rate, matches, "rate");
    let duration = merge_value(cli.duration, cfg.duration, matches, "duration");
    let pattern = merge_value(cli.pattern, cfg.pattern, matches, "pattern");
    let rrul_baseline = merge_value(cli.rrul_baseline, cfg.rrul_baseline, matches, "rrul-baseline");
    let rrul_ramp_up = merge_value(cli.rrul_ramp_up, cfg.rrul_ramp_up, matches, "rrul-ramp-up");
    let streams = merge_value(cli.streams, cfg.streams, matches, "streams");

    let cross_host = if was_cli_provided(matches, "cross-host") {
        cli.cross_host
    } else {
        cfg.cross_host.unwrap_or(cli.cross_host)
    };

    let output = merge_value(cli.output, cfg.output, matches, "output");
    let file = merge_option(cli.file, cfg.file, matches, "file");
    let raw_file = merge_option(cli.raw_file, cfg.raw_file, matches, "raw-file");
    let dscp = merge_option(cli.dscp, cfg.dscp, matches, "dscp");
    let ecn = merge_option(cli.ecn, cfg.ecn, matches, "ecn");
    let client_id = merge_option(cli.client_id, cfg.client_id, matches, "client-id");
    let bind_iface = merge_option(cli.bind_iface, cfg.bind_iface, matches, "bind-iface");
    let source_ip = if was_cli_provided(matches, "source-ip") {
        cli.source_ip
    } else {
        cfg.source_ip
            .as_deref()
            .and_then(|s| s.parse().ok())
            .or(cli.source_ip)
    };
    let control_bind_ip = if was_cli_provided(matches, "control-bind-ip") {
        cli.control_bind_ip
    } else {
        cfg.control_bind_ip
            .as_deref()
            .and_then(|s| s.parse().ok())
            .or(cli.control_bind_ip)
    };

    // Merge thresholds: config first, then CLI overrides by key
    let mut thresholds = Thresholds::default();
    if let Some(cfg_thresholds) = &cfg.thresholds {
        for (k, v) in cfg_thresholds {
            let s = format!("{k}={v}");
            if let Err(e) = thresholds.parse_threshold(&s) {
                eprintln!("bngtester-client: config thresholds: {e}");
                std::process::exit(1);
            }
        }
    }
    for t in &cli.thresholds {
        if let Err(e) = thresholds.parse_threshold(t) {
            eprintln!("bngtester-client: {e}");
            std::process::exit(1);
        }
    }

    // Build stream overrides: config first, then CLI overrides per stream ID
    let mut stream_overrides = StreamOverrides::default();

    // Apply config stream_overrides first
    if let Some(cfg_overrides) = &cfg.stream_overrides {
        for entry in cfg_overrides {
            if let Some(size_val) = entry.size {
                stream_overrides.sizes.push((entry.id, size_val));
            }
            if let Some(rate_val) = entry.rate {
                stream_overrides.rates.push((entry.id, rate_val));
            }
            if let Some(ref pat_str) = entry.pattern {
                match bngtester::stream::config::parse_stream_pattern(&format!(
                    "{}={pat_str}",
                    entry.id
                )) {
                    Ok((id, pat)) => stream_overrides.patterns.push((id, pat)),
                    Err(e) => {
                        eprintln!("bngtester-client: config stream_overrides: {e}");
                        std::process::exit(1);
                    }
                }
            }
            if let Some(ref dscp_str) = entry.dscp {
                match bngtester::dscp::parse_stream_dscp(&format!(
                    "{}={dscp_str}",
                    entry.id
                )) {
                    Ok((id, dscp_val)) => stream_overrides.dscps.push((id, dscp_val)),
                    Err(e) => {
                        eprintln!("bngtester-client: config stream_overrides: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    // Apply CLI stream overrides (these come after config, so last-match-wins gives CLI priority)
    for s in &cli.stream_dscp {
        match bngtester::dscp::parse_stream_dscp(s) {
            Ok((id, dscp_val)) => stream_overrides.dscps.push((id, dscp_val)),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    }
    for s in &cli.stream_size {
        match bngtester::stream::config::parse_stream_size(s) {
            Ok((id, size_val)) => stream_overrides.sizes.push((id, size_val)),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    }
    for s in &cli.stream_rate {
        match bngtester::stream::config::parse_stream_rate(s) {
            Ok((id, rate_val)) => stream_overrides.rates.push((id, rate_val)),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    }
    for s in &cli.stream_pattern {
        match bngtester::stream::config::parse_stream_pattern(s) {
            Ok((id, pat)) => stream_overrides.patterns.push((id, pat)),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    }

    ResolvedClient {
        server,
        mode,
        protocol,
        size,
        rate,
        duration,
        pattern,
        rrul_baseline,
        rrul_ramp_up,
        streams,
        cross_host,
        output,
        file,
        raw_file,
        dscp,
        ecn,
        client_id,
        bind_iface,
        source_ip,
        control_bind_ip,
        thresholds,
        stream_overrides,
    }
}

fn parse_mode(s: &str) -> TestMode {
    match s {
        "throughput" => TestMode::Throughput,
        "latency" => TestMode::Latency,
        "rrul" => TestMode::Rrul,
        "bidirectional" => TestMode::Bidirectional,
        _ => {
            eprintln!("bngtester-client: invalid mode '{s}'. Must be: throughput, latency, rrul, bidirectional");
            std::process::exit(1);
        }
    }
}

fn parse_pattern(s: &str) -> TrafficPattern {
    match s {
        "fixed" => TrafficPattern::Fixed,
        "imix" => TrafficPattern::Imix,
        "sweep" => TrafficPattern::Sweep,
        _ => {
            eprintln!("bngtester-client: invalid pattern '{s}'. Must be: fixed, imix, sweep");
            std::process::exit(1);
        }
    }
}

fn parse_protocol(s: &str) -> Protocol {
    match s {
        "tcp" => Protocol::Tcp,
        "udp" => Protocol::Udp,
        _ => {
            eprintln!("bngtester-client: invalid protocol '{s}'. Must be: tcp, udp");
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    // Load config file if specified
    let cfg = if let Some(ref config_path) = cli.config {
        match load_client_config(std::path::Path::new(config_path)) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let resolved = resolve_client(cli, &matches, cfg);

    let mode = parse_mode(&resolved.mode);
    let pattern = parse_pattern(&resolved.pattern);
    let protocol = parse_protocol(&resolved.protocol);

    // Parse DSCP
    let global_dscp = resolved.dscp.as_ref().map(|s| {
        bngtester::dscp::parse_dscp(s).unwrap_or_else(|e| {
            eprintln!("bngtester-client: {e}");
            std::process::exit(1);
        })
    });

    // Parse ECN
    let ecn_mode = resolved.ecn.as_ref().map(|s| {
        bngtester::dscp::parse_ecn_mode(s).unwrap_or_else(|e| {
            eprintln!("bngtester-client: {e}");
            std::process::exit(1);
        })
    }).unwrap_or(bngtester::dscp::EcnMode::Off);

    if let Some(d) = global_dscp {
        eprintln!("bngtester-client: DSCP={} ({})", bngtester::dscp::dscp_name(d), d);
    }
    if let Some(name) = ecn_mode.name() {
        eprintln!("bngtester-client: ECN={name}");
    }

    eprintln!("bngtester-client: connecting to {}", resolved.server);

    match run_test(&resolved, mode, pattern, protocol, global_dscp, ecn_mode).await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("bngtester-client: error: {e}");
            std::process::exit(1);
        }
    }
}

async fn run_test(
    resolved: &ResolvedClient,
    mode: TestMode,
    pattern: TrafficPattern,
    protocol: Protocol,
    global_dscp: Option<u8>,
    ecn_mode: bngtester::dscp::EcnMode,
) -> Result<(), Box<dyn std::error::Error>> {
    let cancel = CancellationToken::new();

    // Signal handler
    let sig_cancel = cancel.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        eprintln!("\nbngtester-client: interrupted, shutting down...");
        sig_cancel.cancel();
    });

    // --- Connect control channel ---
    let stream = if let Some(bind_ip) = resolved.control_bind_ip {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;
        bngtester::socket::bind_source_ip(&sock, bind_ip)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        sock.set_nonblocking(true)?;
        let addr = socket2::SockAddr::from(resolved.server);
        match sock.connect(&addr) {
            Ok(()) => {}
            Err(e) if e.raw_os_error() == Some(libc::EINPROGRESS) => {}
            Err(e) => return Err(e.into()),
        }
        let std_stream: std::net::TcpStream = sock.into();
        let tokio_stream = TcpStream::from_std(std_stream)?;
        tokio::time::timeout(Duration::from_secs(10), tokio_stream.writable())
            .await
            .map_err(|_| -> Box<dyn std::error::Error> { "control channel connect timeout".into() })??;
        tokio_stream
    } else {
        TcpStream::connect(resolved.server).await?
    };
    let (mut reader, mut writer) = stream.into_split();

    // --- Build per-stream config overrides for hello ---
    let mut stream_config_map: std::collections::BTreeMap<u8, StreamConfigOverride> =
        std::collections::BTreeMap::new();
    for &(id, size) in &resolved.stream_overrides.sizes {
        stream_config_map
            .entry(id)
            .or_insert_with(|| StreamConfigOverride {
                stream_id: id,
                size: None,
                rate_pps: None,
                pattern: None,
                dscp: None,
            })
            .size = Some(size);
    }
    for &(id, rate) in &resolved.stream_overrides.rates {
        stream_config_map
            .entry(id)
            .or_insert_with(|| StreamConfigOverride {
                stream_id: id,
                size: None,
                rate_pps: None,
                pattern: None,
                dscp: None,
            })
            .rate_pps = Some(rate);
    }
    for &(id, pat) in &resolved.stream_overrides.patterns {
        stream_config_map
            .entry(id)
            .or_insert_with(|| StreamConfigOverride {
                stream_id: id,
                size: None,
                rate_pps: None,
                pattern: None,
                dscp: None,
            })
            .pattern = Some(pat);
    }
    for &(id, dscp) in &resolved.stream_overrides.dscps {
        stream_config_map
            .entry(id)
            .or_insert_with(|| StreamConfigOverride {
                stream_id: id,
                size: None,
                rate_pps: None,
                pattern: None,
                dscp: None,
            })
            .dscp = Some(dscp);
    }
    let stream_config: Vec<StreamConfigOverride> = stream_config_map.into_values().collect();

    // --- Send Hello ---
    let hello = Message::Hello(HelloMsg {
        mode,
        protocol,
        duration_secs: resolved.duration,
        packet_size: resolved.size as u32,
        rate_pps: resolved.rate,
        pattern,
        streams_per_direction: resolved.streams,
        rrul_baseline_secs: resolved.rrul_baseline,
        rrul_ramp_up_ms: resolved.rrul_ramp_up,
        cross_host: resolved.cross_host,
        dscp: global_dscp,
        stream_config,
        ecn: ecn_mode.name().map(|s| s.to_string()),
        client_id: resolved.client_id.clone(),
        bind_iface: resolved.bind_iface.clone(),
        source_ip: resolved.source_ip.map(|ip| ip.to_string()),
    });
    protocol::write_message(&mut writer, &hello).await?;

    // --- Wait for Ready ---
    let ready = match protocol::read_message(&mut reader).await? {
        Some(Message::Ready(r)) => r,
        Some(Message::Error(e)) => return Err(format!("server error: {}", e.reason).into()),
        other => return Err(format!("expected ready, got {:?}", other).into()),
    };
    eprintln!(
        "bngtester-client: server ready, UDP port {}",
        ready.udp_port
    );

    // --- Clock sync ---
    let clock_mode = if resolved.cross_host {
        let mut samples = Vec::new();
        for _ in 0..bngtester::protocol::clock::sync_rounds() {
            let client_send = mono_now_ns();
            let sync_msg = Message::ClockSync(ClockSyncMsg {
                client_send_ns: client_send,
                server_recv_ns: None,
                server_send_ns: None,
            });
            protocol::write_message(&mut writer, &sync_msg).await?;

            match protocol::read_message(&mut reader).await? {
                Some(Message::ClockSync(cs)) => {
                    let client_recv = mono_now_ns();
                    if let (Some(srv_recv), Some(srv_send)) = (cs.server_recv_ns, cs.server_send_ns)
                    {
                        samples.push(ClockSample {
                            client_send_ns: client_send,
                            server_recv_ns: srv_recv,
                            server_send_ns: srv_send,
                            client_recv_ns: client_recv,
                        });
                    }
                }
                _ => break,
            }
        }
        let offset = bngtester::protocol::clock::estimate_offset(&samples).unwrap_or(0);
        eprintln!("bngtester-client: clock offset estimated: {offset}ns");
        ClockMode::SyncEstimated { offset_ns: offset }
    } else {
        ClockMode::SameHost
    };

    // --- Send Start ---
    let start = Message::Start(StartMsg {
        client_udp_port: None,
        client_tcp_ports: vec![],
    });
    protocol::write_message(&mut writer, &start).await?;

    // --- Run test ---
    let server_udp_addr: SocketAddr = SocketAddr::new(resolved.server.ip(), ready.udp_port);

    eprintln!(
        "bngtester-client: sending {:?} traffic to {} for {}s",
        mode, server_udp_addr, resolved.duration
    );

    let gen_cancel = cancel.clone();
    let stream_resolved = resolved.stream_overrides.resolve(
        0,
        resolved.size as u32,
        resolved.rate,
        pattern,
        global_dscp,
    );
    let tos = bngtester::dscp::build_tos(stream_resolved.dscp, ecn_mode);
    let gen_result = run_udp_generator(
        UdpGeneratorConfig {
            target: server_udp_addr,
            stream_id: 0,
            rate_pps: stream_resolved.rate_pps,
            duration: Duration::from_secs(resolved.duration as u64),
            packet_size: stream_resolved.size as usize,
            pattern: stream_resolved.pattern,
            tos: if tos != 0 { Some(tos) } else { None },
            bind_iface: resolved.bind_iface.clone(),
            source_ip: resolved.source_ip,
        },
        gen_cancel,
    )
    .await?;

    eprintln!(
        "bngtester-client: sent {} packets ({} bytes)",
        gen_result.packets_sent, gen_result.bytes_sent
    );

    // --- Send Stop ---
    let stop = Message::Stop;
    protocol::write_message(&mut writer, &stop).await?;

    // --- Receive results ---
    let session_status;
    let server_streams;

    match protocol::read_message(&mut reader).await? {
        Some(Message::Results(r)) => {
            session_status = r.status;
            server_streams = r.streams;
        }
        other => {
            eprintln!("bngtester-client: expected results, got {:?}", other);
            session_status = SessionStatus::Partial;
            server_streams = vec![];
        }
    }

    // --- Build merged report ---
    let mut report_streams = Vec::new();
    for sr in &server_streams {
        let loss_pct = if sr.packets_received + sr.packets_lost > 0 {
            sr.packets_lost as f64 / (sr.packets_received + sr.packets_lost) as f64 * 100.0
        } else {
            0.0
        };

        let sr_resolved = resolved.stream_overrides.resolve(
            sr.stream_id,
            resolved.size as u32,
            resolved.rate,
            pattern,
            global_dscp,
        );
        let stream_config_report = if resolved.stream_overrides.has_overrides(sr.stream_id) {
            Some(StreamConfigReport {
                size: sr_resolved.size,
                rate_pps: sr_resolved.rate_pps,
                pattern: format!("{:?}", sr_resolved.pattern).to_lowercase(),
            })
        } else {
            None
        };
        report_streams.push(StreamReport {
            id: sr.stream_id,
            stream_type: "udp_latency".to_string(),
            direction: "upstream".to_string(),
            status: sr.status,
            dscp: sr_resolved.dscp,
            dscp_name: sr_resolved.dscp.map(bngtester::dscp::dscp_name),
            ecn_mode: ecn_mode.name().map(|s| s.to_string()),
            bind_iface: resolved.bind_iface.clone(),
            source_ip: resolved.source_ip.map(|ip| ip.to_string()),
            config: stream_config_report,
            results: StreamResults {
                packets_sent: Some(gen_result.packets_sent),
                packets_received: Some(sr.packets_received),
                packets_lost: Some(sr.packets_lost),
                loss_percent: Some(loss_pct),
                packets_reordered: None,
                reorder_percent: None,
                latency_us: sr.latency_ns.clone(),
                jitter_us: sr.jitter_ns.map(|j| j / 1_000.0),
                throughput_bps: Some(sr.throughput_bps),
                throughput_pps: Some(sr.throughput_pps),
                goodput_bps: None,
                tcp_info: sr.tcp_info.clone(),
                ecn_ect_sent: if ecn_mode != bngtester::dscp::EcnMode::Off { Some(gen_result.packets_sent) } else { None },
                ecn_not_ect_received: sr.ecn_not_ect,
                ecn_ect0_received: sr.ecn_ect0,
                ecn_ect1_received: sr.ecn_ect1,
                ecn_ce_received: sr.ecn_ce,
                ecn_ce_ratio: {
                    let total = sr.ecn_not_ect.unwrap_or(0) + sr.ecn_ect0.unwrap_or(0)
                        + sr.ecn_ect1.unwrap_or(0) + sr.ecn_ce.unwrap_or(0);
                    if total > 0 {
                        Some(sr.ecn_ce.unwrap_or(0) as f64 / total as f64 * 100.0)
                    } else {
                        None
                    }
                },
            },
        });
    }

    let report = TestReport {
        status: session_status,
        clock_mode: clock_mode.name().to_string(),
        test: TestConfig {
            mode,
            duration_secs: resolved.duration,
            client: "local".to_string(),
            server: resolved.server.to_string(),
        },
        streams: report_streams,
        bufferbloat: None,
        time_series: vec![],
        histogram: None,
    };

    // --- Output report ---
    let output: Box<dyn std::io::Write> = if let Some(ref path) = resolved.file {
        Box::new(std::fs::File::create(path)?)
    } else {
        Box::new(std::io::stdout())
    };
    let mut output = output;

    match resolved.output.as_str() {
        "json" => write_json(&mut output, &report)?,
        "junit" => write_junit(&mut output, &report, &resolved.thresholds)?,
        "text" => write_text(&mut output, &report)?,
        other => {
            eprintln!("bngtester-client: unknown output format: {other}");
        }
    }

    Ok(())
}
