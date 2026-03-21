// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use std::net::SocketAddr;
use std::time::Duration;

use clap::Parser;
use tokio::net::TcpStream;
use tokio::signal;
use tokio_util::sync::CancellationToken;

use bngtester::protocol::clock::{mono_now_ns, ClockMode, ClockSample};
use bngtester::protocol::{
    self, ClockSyncMsg, HelloMsg, Message, Protocol, SessionStatus, StartMsg, TestMode,
    TrafficPattern,
};
use bngtester::report::json::write_json;
use bngtester::report::junit::write_junit;
use bngtester::report::text::write_text;
use bngtester::report::{StreamReport, StreamResults, TestConfig, TestReport, Thresholds};
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

    eprintln!("bngtester-client: connecting to {}", cli.server);

    match run_test(&cli, mode, pattern, protocol, &thresholds).await {
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
    let stream = TcpStream::connect(cli.server).await?;
    let (mut reader, mut writer) = stream.into_split();

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
    let gen_result = run_udp_generator(
        UdpGeneratorConfig {
            target: server_udp_addr,
            stream_id: 0,
            rate_pps: cli.rate,
            duration: Duration::from_secs(cli.duration as u64),
            packet_size: cli.size,
            pattern,
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

        report_streams.push(StreamReport {
            id: sr.stream_id,
            stream_type: "udp_latency".to_string(),
            direction: "upstream".to_string(),
            status: sr.status,
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
