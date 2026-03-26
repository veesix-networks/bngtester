// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Write as FmtWrite;
use std::io::Write;

use crate::report::{CombinedReport, TestReport, Thresholds};

/// A single JUnit test case result.
struct TestCase {
    name: String,
    classname: String,
    output: String,
    failure: Option<String>,
}

/// Generate JUnit XML from a test report with optional thresholds.
pub fn write_junit<W: Write>(
    writer: &mut W,
    report: &TestReport,
    thresholds: &Thresholds,
) -> std::io::Result<()> {
    let xml = to_junit_string(report, thresholds);
    writer.write_all(xml.as_bytes())
}

/// Render JUnit XML as a String.
pub fn to_junit_string(report: &TestReport, thresholds: &Thresholds) -> String {
    let mut cases: Vec<TestCase> = Vec::new();
    let mode = format!("{:?}", report.test.mode).to_lowercase();

    for stream in &report.streams {
        let classname = format!("bngtester.{}.{}", mode, stream.direction);
        let r = &stream.results;

        // Loss
        if let Some(loss) = r.loss_percent {
            let mut tc = TestCase {
                name: "packet_loss".to_string(),
                classname: classname.clone(),
                output: String::new(),
                failure: None,
            };
            if let Some(max_loss) = thresholds.loss {
                write!(tc.output, "loss={:.3}% (threshold: <{}%)", loss, max_loss).unwrap();
                if loss > max_loss {
                    tc.failure = Some(format!(
                        "loss={:.3}% exceeds threshold {}%",
                        loss, max_loss
                    ));
                }
            } else {
                write!(tc.output, "loss={:.3}%", loss).unwrap();
            }
            cases.push(tc);
        }

        // Latency percentiles
        if let Some(ref lat) = r.latency_us {
            for (name, value, threshold) in [
                ("latency_p50", lat.p50, thresholds.p50),
                ("latency_p95", lat.p95, thresholds.p95),
                ("latency_p99", lat.p99, thresholds.p99),
                ("latency_p999", lat.p999, thresholds.p999),
            ] {
                let mut tc = TestCase {
                    name: name.to_string(),
                    classname: classname.clone(),
                    output: String::new(),
                    failure: None,
                };
                if let Some(max) = threshold {
                    write!(tc.output, "{}={:.1}us (threshold: <{}us)", name, value, max).unwrap();
                    if value > max {
                        tc.failure = Some(format!(
                            "{}={:.1}us exceeds threshold {}us",
                            name, value, max
                        ));
                    }
                } else {
                    write!(tc.output, "{}={:.1}us", name, value).unwrap();
                }
                cases.push(tc);
            }
        }

        // Jitter
        if let Some(jitter) = r.jitter_us {
            let mut tc = TestCase {
                name: "jitter".to_string(),
                classname: classname.clone(),
                output: String::new(),
                failure: None,
            };
            if let Some(max) = thresholds.jitter {
                write!(tc.output, "jitter={:.1}us (threshold: <{}us)", jitter, max).unwrap();
                if jitter > max {
                    tc.failure = Some(format!(
                        "jitter={:.1}us exceeds threshold {}us",
                        jitter, max
                    ));
                }
            } else {
                write!(tc.output, "jitter={:.1}us", jitter).unwrap();
            }
            cases.push(tc);
        }

        // Throughput
        if let Some(bps) = r.throughput_bps.or(r.goodput_bps) {
            let mbps = bps as f64 / 1_000_000.0;
            let mut tc = TestCase {
                name: "throughput".to_string(),
                classname: classname.clone(),
                output: String::new(),
                failure: None,
            };
            if let Some(min) = thresholds.throughput {
                write!(tc.output, "throughput={:.1}Mbps (threshold: >{}Mbps)", mbps, min).unwrap();
                if mbps < min {
                    tc.failure = Some(format!(
                        "throughput={:.1}Mbps below threshold {}Mbps",
                        mbps, min
                    ));
                }
            } else {
                write!(tc.output, "throughput={:.1}Mbps", mbps).unwrap();
            }
            cases.push(tc);
        }
    }

    // Bufferbloat
    if let Some(ref bb) = report.bufferbloat {
        let classname = format!("bngtester.{}", mode);
        let mut tc = TestCase {
            name: "bufferbloat".to_string(),
            classname,
            output: String::new(),
            failure: None,
        };
        if let Some(max) = thresholds.bloat {
            write!(
                tc.output,
                "bloat_ratio={:.2} (threshold: <{})",
                bb.bloat_ratio, max
            )
            .unwrap();
            if bb.bloat_ratio > max {
                tc.failure = Some(format!(
                    "bloat_ratio={:.2} exceeds threshold {}. Loaded p99 ({:.1}us) is {:.1}x baseline p99 ({:.1}us). Check BNG AQM configuration.",
                    bb.bloat_ratio, max, bb.loaded_p99_us, bb.bloat_ratio, bb.baseline_p99_us
                ));
            }
        } else {
            write!(tc.output, "bloat_ratio={:.2}", bb.bloat_ratio).unwrap();
        }
        cases.push(tc);
    }

    // Build XML
    let failures = cases.iter().filter(|c| c.failure.is_some()).count();
    let mut xml = String::new();
    writeln!(xml, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(xml, "<testsuites>").unwrap();
    writeln!(
        xml,
        "  <testsuite name=\"bngtester.{}\" tests=\"{}\" failures=\"{}\">",
        mode,
        cases.len(),
        failures
    )
    .unwrap();

    for tc in &cases {
        writeln!(
            xml,
            "    <testcase name=\"{}\" classname=\"{}\">",
            escape_xml(&tc.name),
            escape_xml(&tc.classname)
        )
        .unwrap();
        if let Some(ref msg) = tc.failure {
            writeln!(
                xml,
                "      <failure message=\"{}\">{}</failure>",
                escape_xml(msg),
                escape_xml(msg)
            )
            .unwrap();
        }
        writeln!(
            xml,
            "      <system-out>{}</system-out>",
            escape_xml(&tc.output)
        )
        .unwrap();
        writeln!(xml, "    </testcase>").unwrap();
    }

    writeln!(xml, "  </testsuite>").unwrap();
    writeln!(xml, "</testsuites>").unwrap();
    xml
}

/// Write a combined multi-client JUnit XML report to the given writer.
pub fn write_combined_junit<W: Write>(
    writer: &mut W,
    report: &CombinedReport,
    thresholds: &Thresholds,
) -> std::io::Result<()> {
    let xml = to_combined_junit_string(report, thresholds);
    writer.write_all(xml.as_bytes())
}

/// Render a combined multi-client JUnit XML report as a String.
/// Each client becomes a separate testsuite element.
pub fn to_combined_junit_string(report: &CombinedReport, thresholds: &Thresholds) -> String {
    let mut xml = String::new();
    writeln!(xml, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(xml, "<testsuites>").unwrap();

    for cr in &report.clients {
        let per_client = to_junit_string(&cr.report, thresholds);
        // Extract the inner <testsuite> from the per-client output, skipping
        // the <?xml?> header and the outer <testsuites> wrapper.
        for line in per_client.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("<?xml") || trimmed == "<testsuites>" || trimmed == "</testsuites>" {
                continue;
            }
            // Rewrite testsuite name to include client_id
            if trimmed.starts_with("<testsuite name=\"") {
                let rewritten = line.replacen(
                    "<testsuite name=\"",
                    &format!("<testsuite name=\"{}/", escape_xml(&cr.client_id)),
                    1,
                );
                writeln!(xml, "{rewritten}").unwrap();
            } else {
                writeln!(xml, "{line}").unwrap();
            }
        }
    }

    writeln!(xml, "</testsuites>").unwrap();
    xml
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{LatencyStats, SessionStatus, StreamStatus, TestMode};
    use crate::report::*;

    fn sample_report() -> TestReport {
        TestReport {
            status: SessionStatus::Complete,
            clock_mode: "same-host".to_string(),
            test: TestConfig {
                mode: TestMode::Latency,
                duration_secs: 10,
                client: "10.0.0.2".to_string(),
                server: "10.0.0.1:5000".to_string(),
                subscriber_ip: None,
            },
            streams: vec![StreamReport {
                id: 0,
                stream_type: "udp_latency".to_string(),
                direction: "upstream".to_string(),
                status: StreamStatus::Complete,
                dscp: None,
                dscp_name: None,
                ecn_mode: None,
                bind_iface: None,
                source_ip: None,
                config: None,
                results: StreamResults {
                    packets_sent: Some(1000),
                    packets_received: Some(999),
                    packets_lost: Some(1),
                    loss_percent: Some(0.1),
                    packets_reordered: Some(0),
                    reorder_percent: Some(0.0),
                    latency_us: Some(LatencyStats {
                        min: 10.0,
                        avg: 45.0,
                        max: 200.0,
                        p50: 40.0,
                        p95: 80.0,
                        p99: 150.0,
                        p999: 190.0,
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
        }
    }

    #[test]
    fn junit_no_thresholds() {
        let report = sample_report();
        let xml = to_junit_string(&report, &Thresholds::default());
        assert!(xml.contains("<testsuites>"));
        assert!(xml.contains("failures=\"0\""));
        assert!(xml.contains("packet_loss"));
        assert!(xml.contains("latency_p99"));
        assert!(xml.contains("jitter"));
        assert!(xml.contains("throughput"));
    }

    #[test]
    fn junit_threshold_pass() {
        let report = sample_report();
        let mut t = Thresholds::default();
        t.loss = Some(1.0); // 0.1% < 1.0% = pass
        t.p99 = Some(500.0); // 150us < 500us = pass
        let xml = to_junit_string(&report, &t);
        assert!(xml.contains("failures=\"0\""));
    }

    #[test]
    fn junit_threshold_fail() {
        let report = sample_report();
        let mut t = Thresholds::default();
        t.p99 = Some(100.0); // 150us > 100us = fail
        let xml = to_junit_string(&report, &t);
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("<failure"));
        assert!(xml.contains("exceeds threshold"));
    }

    #[test]
    fn junit_bufferbloat_fail() {
        let mut report = sample_report();
        report.test.mode = TestMode::Rrul;
        report.bufferbloat = Some(BufferbloatReport {
            baseline_p99_us: 45.0,
            loaded_p99_us: 225.0,
            bloat_ratio: 5.0,
        });
        let mut t = Thresholds::default();
        t.bloat = Some(3.0);
        let xml = to_junit_string(&report, &t);
        assert!(xml.contains("<failure"));
        assert!(xml.contains("bloat_ratio=5.00 exceeds threshold 3"));
    }
}
