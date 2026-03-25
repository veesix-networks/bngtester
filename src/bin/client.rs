// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::net::SocketAddr;
use std::time::Duration;

use clap::Parser;
use tokio::net::TcpStream;
use tokio::signal;
use tokio_util::sync::CancellationToken;

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
    server: SocketAddr,

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
    let cli = Cli::parse();

    let mut thresholds = Thresholds::default();
    for t in &cli.thresholds {
        if let Err(e) = thresholds.parse_threshold(t) {
            eprintln!("bngtester-client: {e}");
            std::process::exit(1);
        }
    }

    let mode = parse_mode(&cli.mode);
    let pattern = parse_pattern(&cli.pattern);
    let protocol = parse_protocol(&cli.protocol);

    // Parse DSCP
    let global_dscp = cli.dscp.as_ref().map(|s| {
        bngtester::dscp::parse_dscp(s).unwrap_or_else(|e| {
            eprintln!("bngtester-client: {e}");
            std::process::exit(1);
        })
    });

    // Build per-stream overrides
    let mut stream_overrides = StreamOverrides::default();
    for s in &cli.stream_dscp {
        match bngtester::dscp::parse_stream_dscp(s) {
            Ok((id, dscp)) => stream_overrides.dscps.push((id, dscp)),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    }
    for s in &cli.stream_size {
        match bngtester::stream::config::parse_stream_size(s) {
            Ok((id, size)) => stream_overrides.sizes.push((id, size)),
            Err(e) => {
                eprintln!("bngtester-client: {e}");
                std::process::exit(1);
            }
        }
    }
    for s in &cli.stream_rate {
        match bngtester::stream::config::parse_stream_rate(s) {
            Ok((id, rate)) => stream_overrides.rates.push((id, rate)),
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

    // Parse ECN
    let ecn_mode = cli.ecn.as_ref().map(|s| {
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

    eprintln!("bngtester-client: connecting to {}", cli.server);

    match run_test(&cli, mode, pattern, protocol, &thresholds, global_dscp, &stream_overrides, ecn_mode).await {
        Ok(()) => {}
        Err(e) => {
            eprintln!("bngtester-client: error: {e}");
            std::process::exit(1);
        }
    }
}

async fn run_test(
    cli: &Cli,
    mode: TestMode,
    pattern: TrafficPattern,
    protocol: Protocol,
    thresholds: &Thresholds,
    global_dscp: Option<u8>,
    stream_overrides: &StreamOverrides,
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
    let stream = if let Some(bind_ip) = cli.control_bind_ip {
        let sock = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;
        bngtester::socket::bind_source_ip(&sock, bind_ip)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        sock.set_nonblocking(true)?;
        let addr = socket2::SockAddr::from(cli.server);
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
        TcpStream::connect(cli.server).await?
    };
    let (mut reader, mut writer) = stream.into_split();

    // --- Build per-stream config overrides for hello ---
    let mut stream_config_map: std::collections::BTreeMap<u8, StreamConfigOverride> =
        std::collections::BTreeMap::new();
    for &(id, size) in &stream_overrides.sizes {
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
    for &(id, rate) in &stream_overrides.rates {
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
    for &(id, pat) in &stream_overrides.patterns {
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
    for &(id, dscp) in &stream_overrides.dscps {
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
        duration_secs: cli.duration,
        packet_size: cli.size as u32,
        rate_pps: cli.rate,
        pattern,
        streams_per_direction: cli.streams,
        rrul_baseline_secs: cli.rrul_baseline,
        rrul_ramp_up_ms: cli.rrul_ramp_up,
        cross_host: cli.cross_host,
        dscp: global_dscp,
        stream_config,
        ecn: ecn_mode.name().map(|s| s.to_string()),
        client_id: cli.client_id.clone(),
        bind_iface: cli.bind_iface.clone(),
        source_ip: cli.source_ip.map(|ip| ip.to_string()),
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
    let clock_mode = if cli.cross_host {
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
    let server_udp_addr: SocketAddr = SocketAddr::new(cli.server.ip(), ready.udp_port);

    eprintln!(
        "bngtester-client: sending {:?} traffic to {} for {}s",
        mode, server_udp_addr, cli.duration
    );

    let gen_cancel = cancel.clone();
    let resolved = stream_overrides.resolve(
        0,
        cli.size as u32,
        cli.rate,
        pattern,
        global_dscp,
    );
    let tos = bngtester::dscp::build_tos(resolved.dscp, ecn_mode);
    let gen_result = run_udp_generator(
        UdpGeneratorConfig {
            target: server_udp_addr,
            stream_id: 0,
            rate_pps: resolved.rate_pps,
            duration: Duration::from_secs(cli.duration as u64),
            packet_size: resolved.size as usize,
            pattern: resolved.pattern,
            tos: if tos != 0 { Some(tos) } else { None },
            bind_iface: cli.bind_iface.clone(),
            source_ip: cli.source_ip,
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

        let sr_resolved = stream_overrides.resolve(
            sr.stream_id,
            cli.size as u32,
            cli.rate,
            pattern,
            global_dscp,
        );
        let stream_config_report = if stream_overrides.has_overrides(sr.stream_id) {
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
            bind_iface: cli.bind_iface.clone(),
            source_ip: cli.source_ip.map(|ip| ip.to_string()),
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
            duration_secs: cli.duration,
            client: "local".to_string(),
            server: cli.server.to_string(),
        },
        streams: report_streams,
        bufferbloat: None,
        time_series: vec![],
        histogram: None,
    };

    // --- Output report ---
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
            eprintln!("bngtester-client: unknown output format: {other}");
        }
    }

    Ok(())
}
