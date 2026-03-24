// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Write;

use crate::report::{CombinedReport, TestReport};

/// Write a full JSON report to the given writer.
pub fn write_json<W: Write>(writer: &mut W, report: &TestReport) -> std::io::Result<()> {
    serde_json::to_writer_pretty(writer, report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

/// Render a full JSON report to a String.
pub fn to_json_string(report: &TestReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

/// Write a combined multi-client JSON report to the given writer.
pub fn write_combined_json<W: Write>(
    writer: &mut W,
    report: &CombinedReport,
) -> std::io::Result<()> {
    serde_json::to_writer_pretty(writer, report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{SessionStatus, StreamStatus, TestMode};
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
                    packets_received: Some(999),
                    packets_lost: Some(1),
                    loss_percent: Some(0.1),
                    packets_reordered: Some(0),
                    reorder_percent: Some(0.0),
                    latency_us: None,
                    jitter_us: Some(8.7),
                    throughput_bps: Some(800000),
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
    fn json_output_valid() {
        let report = sample_report();
        let json = to_json_string(&report).unwrap();
        // Parse it back to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["status"], "complete");
        assert_eq!(parsed["test"]["mode"], "latency");
        assert_eq!(parsed["streams"][0]["id"], 0);
    }

    #[test]
    fn json_write_to_buffer() {
        let report = sample_report();
        let mut buf = Vec::new();
        write_json(&mut buf, &report).unwrap();
        assert!(!buf.is_empty());
        // Verify it's parseable
        let _: serde_json::Value = serde_json::from_slice(&buf).unwrap();
    }
}
