// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

/// RFC 3550 jitter computation.
///
/// J(i) = J(i-1) + (|D(i-1,i)| - J(i-1)) / 16
///
/// Where D(i-1,i) is the difference in one-way delays between
/// consecutive packets.
pub struct JitterTracker {
    last_delay_ns: Option<f64>,
    jitter: f64,
    count: u64,
}

impl JitterTracker {
    pub fn new() -> Self {
        Self {
            last_delay_ns: None,
            jitter: 0.0,
            count: 0,
        }
    }

    /// Record a one-way delay sample in nanoseconds.
    pub fn record(&mut self, delay_ns: f64) {
        self.count += 1;
        if let Some(last) = self.last_delay_ns {
            let d = (delay_ns - last).abs();
            self.jitter += (d - self.jitter) / 16.0;
        }
        self.last_delay_ns = Some(delay_ns);
    }

    /// Current jitter estimate in nanoseconds.
    pub fn jitter_ns(&self) -> f64 {
        self.jitter
    }

    /// Current jitter estimate in microseconds.
    pub fn jitter_us(&self) -> f64 {
        self.jitter / 1_000.0
    }

    /// Number of samples processed.
    pub fn count(&self) -> u64 {
        self.count
    }
}

impl Default for JitterTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_delay_zero_jitter() {
        let mut j = JitterTracker::new();
        for _ in 0..100 {
            j.record(1000.0);
        }
        assert!(j.jitter_ns() < 0.001);
    }

    #[test]
    fn varying_delay_nonzero_jitter() {
        let mut j = JitterTracker::new();
        for i in 0..100 {
            let delay = if i % 2 == 0 { 1000.0 } else { 2000.0 };
            j.record(delay);
        }
        assert!(j.jitter_ns() > 0.0);
    }

    #[test]
    fn single_sample_zero_jitter() {
        let mut j = JitterTracker::new();
        j.record(1000.0);
        assert_eq!(j.jitter_ns(), 0.0);
        assert_eq!(j.count(), 1);
    }

    #[test]
    fn exponential_smoothing() {
        let mut j = JitterTracker::new();
        j.record(1000.0);
        j.record(2000.0); // D = 1000, J = 0 + (1000 - 0) / 16 = 62.5
        assert!((j.jitter_ns() - 62.5).abs() < 0.01);
    }
}
