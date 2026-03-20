// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;

/// A single time-series data point (per-second bucket).
#[derive(Debug, Clone, Serialize)]
pub struct TimePoint {
    /// Seconds since test start.
    pub t: u32,
    /// p99 latency in microseconds (None for TCP-only streams).
    pub latency_p99_us: Option<f64>,
    /// Throughput in Mbps.
    pub throughput_mbps: f64,
    /// Loss percentage.
    pub loss_pct: f64,
}

/// Collects per-second metric buckets.
pub struct TimeSeriesCollector {
    /// Start time in nanoseconds (monotonic).
    start_ns: Option<u128>,
    /// Accumulated data per second.
    buckets: Vec<SecondBucket>,
}

/// Internal per-second accumulator.
struct SecondBucket {
    latencies: Vec<f64>,
    bytes: u64,
    packets_received: u64,
}

impl SecondBucket {
    fn new() -> Self {
        Self {
            latencies: Vec::new(),
            bytes: 0,
            packets_received: 0,

        }
    }
}

impl TimeSeriesCollector {
    pub fn new() -> Self {
        Self {
            start_ns: None,
            buckets: Vec::new(),
        }
    }

    /// Record a data point at the given timestamp.
    pub fn record(
        &mut self,
        timestamp_ns: u128,
        bytes: u64,
        latency_ns: Option<f64>,
    ) {
        if self.start_ns.is_none() {
            self.start_ns = Some(timestamp_ns);
        }
        let elapsed_ns = timestamp_ns.saturating_sub(self.start_ns.unwrap());
        let bucket_idx = (elapsed_ns / 1_000_000_000) as usize;

        // Extend buckets if needed
        while self.buckets.len() <= bucket_idx {
            self.buckets.push(SecondBucket::new());
        }

        let bucket = &mut self.buckets[bucket_idx];
        bucket.bytes += bytes;
        bucket.packets_received += 1;
        if let Some(lat) = latency_ns {
            bucket.latencies.push(lat);
        }
    }

    /// Finalize and return time-series data points.
    pub fn finalize(&mut self) -> Vec<TimePoint> {
        self.buckets
            .iter_mut()
            .enumerate()
            .map(|(i, bucket)| {
                let latency_p99_us = if bucket.latencies.is_empty() {
                    None
                } else {
                    bucket.latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let idx = ((bucket.latencies.len() as f64 - 1.0) * 0.99).ceil() as usize;
                    let idx = idx.min(bucket.latencies.len() - 1);
                    Some(bucket.latencies[idx] / 1_000.0)
                };
                let throughput_mbps = bucket.bytes as f64 * 8.0 / 1_000_000.0;
                let loss_pct = 0.0; // Loss is computed at stream level, not per-second
                TimePoint {
                    t: i as u32,
                    latency_p99_us,
                    throughput_mbps,
                    loss_pct,
                }
            })
            .collect()
    }
}

impl Default for TimeSeriesCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collector() {
        let mut c = TimeSeriesCollector::new();
        let points = c.finalize();
        assert!(points.is_empty());
    }

    #[test]
    fn single_second() {
        let mut c = TimeSeriesCollector::new();
        c.record(0, 1000, Some(50_000.0));
        c.record(500_000_000, 1000, Some(60_000.0));
        let points = c.finalize();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].t, 0);
        assert!(points[0].latency_p99_us.is_some());
    }

    #[test]
    fn multiple_seconds() {
        let mut c = TimeSeriesCollector::new();
        // Second 0
        c.record(0, 125_000_000, None); // 125MB = 1Gbps
        // Second 1
        c.record(1_000_000_000, 125_000_000, None);
        // Second 2
        c.record(2_000_000_000, 125_000_000, None);

        let points = c.finalize();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0].t, 0);
        assert_eq!(points[1].t, 1);
        assert_eq!(points[2].t, 2);
        // Each second has 125MB = 1000Mbps
        assert!((points[0].throughput_mbps - 1000.0).abs() < 1.0);
    }
}
