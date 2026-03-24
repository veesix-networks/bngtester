// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Write as FmtWrite;
use std::io::Write;

use crate::report::TestReport;

/// Write a human-readable text report to the given writer.
pub fn write_text<W: Write>(writer: &mut W, report: &TestReport) -> std::io::Result<()> {
    let text = to_text_string(report);
    writer.write_all(text.as_bytes())
}

/// Render a human-readable text report as a String.
pub fn to_text_string(report: &TestReport) -> String {
    let mut out = String::new();
    let mode = format!("{:?}", report.test.mode).to_uppercase();

    writeln!(
        out,
        "bngtester {} test — {}s duration",
        mode, report.test.duration_secs
    )
    .unwrap();
    writeln!(out, "{}", "═".repeat(50)).unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "Status: {:?} | Clock: {}",
        report.status, report.clock_mode
    )
    .unwrap();
    writeln!(out).unwrap();

    // Bufferbloat summary
    if let Some(ref bb) = report.bufferbloat {
        writeln!(
            out,
            "Bufferbloat: {:.2}x (baseline p99: {:.1}µs → loaded p99: {:.1}µs)",
            bb.bloat_ratio, bb.baseline_p99_us, bb.loaded_p99_us
        )
        .unwrap();
        writeln!(out).unwrap();
    }

    // Per-stream details
    for stream in &report.streams {
        let dir = match stream.direction.as_str() {
            "upstream" => "↑",
            "downstream" => "↓",
            _ => "?",
        };
        let r = &stream.results;

        let rate_info = if let Some(pps) = r.throughput_pps {
            format!(" {}pps", pps)
        } else {
            String::new()
        };

        let dscp_info = match &stream.dscp_name {
            Some(name) => format!(" DSCP={name}"),
            None => String::new(),
        };

        let ecn_info = match &stream.ecn_mode {
            Some(mode) => format!(" ECN={mode}"),
            None => String::new(),
        };

        let config_info = match &stream.config {
            Some(cfg) => {
                let rate_str = if cfg.rate_pps == 0 {
                    "unlimited".to_string()
                } else {
                    format!("{}pps", cfg.rate_pps)
                };
                format!(" {}B@{} {}", cfg.size, rate_str, cfg.pattern)
            }
            None => String::new(),
        };

        writeln!(
            out,
            "  Stream {} [{} {}{}{}{}]{}",
            stream.id, stream.stream_type, dir, dscp_info, ecn_info, config_info, rate_info
        )
        .unwrap();

        // UDP latency metrics
        if let Some(ref lat) = r.latency_us {
            writeln!(
                out,
                "    Latency:  min={:.1}µs avg={:.1}µs max={:.1}µs p99={:.1}µs",
                lat.min, lat.avg, lat.max, lat.p99
            )
            .unwrap();
        }
        if let Some(jitter) = r.jitter_us {
            writeln!(out, "    Jitter:   {:.1}µs", jitter).unwrap();
        }
        if let Some(loss) = r.loss_percent {
            let lost = r.packets_lost.unwrap_or(0);
            let recv = r.packets_received.unwrap_or(0);
            writeln!(out, "    Loss:     {:.3}% ({}/{})", loss, lost, recv + lost).unwrap();
        }
        if let Some(reorder) = r.reorder_percent {
            writeln!(out, "    Reorder:  {:.1}%", reorder).unwrap();
        }

        // ECN breakdown
        if let Some(ce) = r.ecn_ce_received {
            let ratio = r.ecn_ce_ratio.unwrap_or(0.0);
            let ect0 = r.ecn_ect0_received.unwrap_or(0);
            let ect1 = r.ecn_ect1_received.unwrap_or(0);
            let not_ect = r.ecn_not_ect_received.unwrap_or(0);
            writeln!(
                out,
                "    ECN:      CE={} ({:.1}%) ECT0={} ECT1={} Not-ECT={}",
                ce, ratio, ect0, ect1, not_ect
            )
            .unwrap();
        }

        // TCP metrics
        if let Some(ref tcp) = r.tcp_info {
            if let Some(goodput) = r.goodput_bps {
                writeln!(out, "    Goodput:  {:.1} Mbps", goodput as f64 / 1_000_000.0).unwrap();
            }
            writeln!(out, "    RTT:      avg={:.0}µs", tcp.rtt_us).unwrap();
            writeln!(out, "    Retrans:  {}", tcp.retransmissions).unwrap();
        } else if let Some(bps) = r.throughput_bps {
            writeln!(out, "    Throughput: {:.1} Mbps", bps as f64 / 1_000_000.0).unwrap();
        }

        writeln!(out).unwrap();
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{LatencyStats, SessionStatus, StreamStatus, TestMode};
    use crate::report::*;

    #[test]
    fn text_output_readable() {
        let report = TestReport {
            status: SessionStatus::Complete,
            clock_mode: "same-host".to_string(),
            test: TestConfig {
                mode: TestMode::Latency,
                duration_secs: 10,
                client: "10.0.0.2".to_string(),
                server: "10.0.0.1:5000".to_string(),
            },
            streams: vec![StreamReport {
                id: 0,
                stream_type: "udp_latency".to_string(),
                direction: "upstream".to_string(),
                status: StreamStatus::Complete,
                dscp: None,
                dscp_name: None,
                ecn_mode: None,
                config: None,
                results: StreamResults {
                    packets_sent: Some(1000),
                    packets_received: Some(998),
                    packets_lost: Some(2),
                    loss_percent: Some(0.2),
                    packets_reordered: Some(1),
                    reorder_percent: Some(0.1),
                    latency_us: Some(LatencyStats {
                        min: 12.4,
                        avg: 45.2,
                        max: 312.8,
                        p50: 38.1,
                        p95: 89.4,
                        p99: 201.3,
                        p999: 290.0,
                    }),
                    jitter_us: Some(8.7),
                    throughput_bps: Some(800_000),
                    throughput_pps: Some(100),
                    goodput_bps: None,
                    tcp_info: None,
                    ecn_ect_sent: None,
                    ecn_not_ect_received: None,
                    ecn_ect0_received: None,
                    ecn_ect1_received: None,
                    ecn_ce_received: None,
                    ecn_ce_ratio: None,
                },
            }],
            bufferbloat: None,
            time_series: vec![],
            histogram: None,
        };

        let text = to_text_string(&report);
        assert!(text.contains("LATENCY test"));
        assert!(text.contains("same-host"));
        assert!(text.contains("p99=201.3µs"));
        assert!(text.contains("Jitter:   8.7µs"));
        assert!(text.contains("Loss:     0.200%"));
        assert!(text.contains("Reorder:  0.1%"));
    }

    #[test]
    fn text_with_bufferbloat() {
        let report = TestReport {
            status: SessionStatus::Complete,
            clock_mode: "same-host".to_string(),
            test: TestConfig {
                mode: TestMode::Rrul,
                duration_secs: 30,
                client: "10.0.0.2".to_string(),
                server: "10.0.0.1:5000".to_string(),
            },
            streams: vec![],
            bufferbloat: Some(BufferbloatReport {
                baseline_p99_us: 45.2,
                loaded_p99_us: 201.3,
                bloat_ratio: 4.45,
            }),
            time_series: vec![],
            histogram: None,
        };

        let text = to_text_string(&report);
        assert!(text.contains("Bufferbloat: 4.45x"));
        assert!(text.contains("baseline p99: 45.2µs"));
    }
}
