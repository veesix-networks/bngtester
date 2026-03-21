// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::protocol::LatencyStats;

/// Collects latency samples and computes statistics.
pub struct LatencyCollector {
    samples: Vec<f64>,
}

impl LatencyCollector {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(65536),
        }
    }

    /// Record a latency sample in nanoseconds.
    pub fn record(&mut self, latency_ns: f64) {
        self.samples.push(latency_ns);
    }

    /// Number of samples collected.
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// Compute statistics. Returns None if no samples.
    pub fn stats(&mut self) -> Option<LatencyStats> {
        if self.samples.is_empty() {
            return None;
        }
        self.samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = self.samples.len();
        let sum: f64 = self.samples.iter().sum();
        Some(LatencyStats {
            min: self.samples[0],
            avg: sum / n as f64,
            max: self.samples[n - 1],
            p50: self.percentile(50.0),
            p95: self.percentile(95.0),
            p99: self.percentile(99.0),
            p999: self.percentile(99.9),
        })
    }

    fn percentile(&self, p: f64) -> f64 {
        let n = self.samples.len();
        if n == 1 {
            return self.samples[0];
        }
        let rank = (p / 100.0) * (n - 1) as f64;
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        if lower == upper {
            self.samples[lower]
        } else {
            let frac = rank - lower as f64;
            self.samples[lower] * (1.0 - frac) + self.samples[upper] * frac
        }
    }

    /// Get a reference to the raw samples (sorted after stats() call).
    pub fn samples(&self) -> &[f64] {
        &self.samples
    }
}

impl Default for LatencyCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Latency histogram with configurable buckets.
pub struct LatencyHistogram {
    /// Upper bounds of each bucket in nanoseconds.
    pub boundaries: Vec<f64>,
    /// Count per bucket (len = boundaries.len() + 1, last bucket is overflow).
    pub counts: Vec<u64>,
}

impl LatencyHistogram {
    /// Create with default buckets: 10us steps to 1ms, 100us steps to 10ms, 1ms steps above.
    pub fn default_buckets() -> Self {
        let mut boundaries = Vec::new();
        // 10us to 1ms in 10us steps
        let mut b = 10_000.0;
        while b <= 1_000_000.0 {
            boundaries.push(b);
            b += 10_000.0;
        }
        // 1ms to 10ms in 100us steps
        b = 1_100_000.0;
        while b <= 10_000_000.0 {
            boundaries.push(b);
            b += 100_000.0;
        }
        // 10ms to 100ms in 1ms steps
        b = 11_000_000.0;
        while b <= 100_000_000.0 {
            boundaries.push(b);
            b += 1_000_000.0;
        }
        let count = boundaries.len() + 1;
        Self {
            boundaries,
            counts: vec![0; count],
        }
    }

    /// Record a latency sample in nanoseconds.
    pub fn record(&mut self, latency_ns: f64) {
        let idx = self
            .boundaries
            .iter()
            .position(|&b| latency_ns <= b)
            .unwrap_or(self.boundaries.len());
        self.counts[idx] += 1;
    }

    /// Get bucket boundaries in microseconds for reporting.
    pub fn boundaries_us(&self) -> Vec<f64> {
        self.boundaries.iter().map(|b| b / 1_000.0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collector_returns_none() {
        let mut c = LatencyCollector::new();
        assert!(c.stats().is_none());
    }

    #[test]
    fn single_sample() {
        let mut c = LatencyCollector::new();
        c.record(1000.0);
        let s = c.stats().unwrap();
        assert_eq!(s.min, 1000.0);
        assert_eq!(s.max, 1000.0);
        assert_eq!(s.avg, 1000.0);
        assert_eq!(s.p50, 1000.0);
        assert_eq!(s.p99, 1000.0);
    }

    #[test]
    fn multiple_samples() {
        let mut c = LatencyCollector::new();
        for i in 1..=100 {
            c.record(i as f64 * 1000.0);
        }
        let s = c.stats().unwrap();
        assert_eq!(s.min, 1000.0);
        assert_eq!(s.max, 100_000.0);
        assert!((s.avg - 50_500.0).abs() < 1.0);
        // p50 should be around 50,000-51,000
        assert!(s.p50 >= 49_000.0 && s.p50 <= 52_000.0);
        // p99 should be around 99,000-100,000
        assert!(s.p99 >= 98_000.0 && s.p99 <= 100_000.0);
    }

    #[test]
    fn histogram_default_buckets() {
        let h = LatencyHistogram::default_buckets();
        assert!(!h.boundaries.is_empty());
        // First bucket: 10us = 10_000ns
        assert_eq!(h.boundaries[0], 10_000.0);
        // counts should have one more entry than boundaries
        assert_eq!(h.counts.len(), h.boundaries.len() + 1);
    }

    #[test]
    fn histogram_record() {
        let mut h = LatencyHistogram::default_buckets();
        // Record a 5us sample — should go into first bucket (<=10us)
        h.record(5_000.0);
        assert_eq!(h.counts[0], 1);
        // Record a 15us sample — should go into second bucket (<=20us)
        h.record(15_000.0);
        assert_eq!(h.counts[1], 1);
        // Record a huge sample — should go into overflow
        h.record(999_000_000_000.0);
        assert_eq!(h.counts[h.counts.len() - 1], 1);
    }
}
