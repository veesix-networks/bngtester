// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod json;
pub mod jsonl;
pub mod junit;
pub mod text;

use serde::Serialize;

use crate::metrics::timeseries::TimePoint;
use crate::protocol::{
    LatencyStats, SessionStatus, StreamStatus, TcpStats, TestMode,
};

/// Complete test report — the top-level structure all formatters consume.
#[derive(Debug, Clone, Serialize)]
pub struct TestReport {
    pub status: SessionStatus,
    pub clock_mode: String,
    pub test: TestConfig,
    pub streams: Vec<StreamReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bufferbloat: Option<BufferbloatReport>,
    pub time_series: Vec<TimePoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub histogram: Option<HistogramReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestConfig {
    pub mode: TestMode,
    pub duration_secs: u32,
    pub client: String,
    pub server: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamReport {
    pub id: u8,
    #[serde(rename = "type")]
    pub stream_type: String,
    pub direction: String,
    pub status: StreamStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dscp: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dscp_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<StreamConfigReport>,
    pub results: StreamResults,
}

/// Per-stream configuration metadata for reports.
#[derive(Debug, Clone, Serialize)]
pub struct StreamConfigReport {
    pub size: u32,
    pub rate_pps: u32,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamResults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_sent: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_lost: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loss_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packets_reordered: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reorder_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_us: Option<LatencyStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter_us: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput_bps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throughput_pps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goodput_bps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp_info: Option<TcpStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_ect_sent: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_not_ect_received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_ect0_received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_ect1_received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_ce_received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecn_ce_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BufferbloatReport {
    pub baseline_p99_us: f64,
    pub loaded_p99_us: f64,
    pub bloat_ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistogramReport {
    pub bucket_us: Vec<f64>,
    pub counts: Vec<u64>,
}

/// Combined report from multiple client sessions.
#[derive(Debug, Clone, Serialize)]
pub struct CombinedReport {
    pub combined: bool,
    pub total_clients: usize,
    pub clients: Vec<ClientReport>,
}

/// Per-client report within a combined multi-subscriber report.
#[derive(Debug, Clone, Serialize)]
pub struct ClientReport {
    pub client_id: String,
    pub peer: String,
    pub report: TestReport,
}

/// Threshold configuration for JUnit pass/fail.
#[derive(Debug, Clone, Default)]
pub struct Thresholds {
    /// Max acceptable loss percentage.
    pub loss: Option<f64>,
    /// Max acceptable p50 latency in microseconds.
    pub p50: Option<f64>,
    /// Max acceptable p95 latency in microseconds.
    pub p95: Option<f64>,
    /// Max acceptable p99 latency in microseconds.
    pub p99: Option<f64>,
    /// Max acceptable p999 latency in microseconds.
    pub p999: Option<f64>,
    /// Max acceptable jitter in microseconds.
    pub jitter: Option<f64>,
    /// Min acceptable throughput in Mbps.
    pub throughput: Option<f64>,
    /// Max acceptable bufferbloat ratio.
    pub bloat: Option<f64>,
}

impl Thresholds {
    /// Parse a "key=value" threshold string.
    pub fn parse_threshold(&mut self, s: &str) -> Result<(), String> {
        let (key, val) = s
            .split_once('=')
            .ok_or_else(|| format!("invalid threshold format: {s}"))?;
        let v: f64 = val
            .parse()
            .map_err(|_| format!("invalid threshold value: {val}"))?;
        match key {
            "loss" => self.loss = Some(v),
            "p50" => self.p50 = Some(v),
            "p95" => self.p95 = Some(v),
            "p99" => self.p99 = Some(v),
            "p999" => self.p999 = Some(v),
            "jitter" => self.jitter = Some(v),
            "throughput" => self.throughput = Some(v),
            "bloat" => self.bloat = Some(v),
            _ => return Err(format!("unknown threshold key: {key}")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_threshold_valid() {
        let mut t = Thresholds::default();
        t.parse_threshold("loss=1.0").unwrap();
        t.parse_threshold("p99=500").unwrap();
        t.parse_threshold("bloat=3.0").unwrap();
        assert_eq!(t.loss, Some(1.0));
        assert_eq!(t.p99, Some(500.0));
        assert_eq!(t.bloat, Some(3.0));
    }

    #[test]
    fn parse_threshold_invalid_key() {
        let mut t = Thresholds::default();
        assert!(t.parse_threshold("foo=1.0").is_err());
    }

    #[test]
    fn parse_threshold_invalid_format() {
        let mut t = Thresholds::default();
        assert!(t.parse_threshold("noequalssign").is_err());
    }
}
