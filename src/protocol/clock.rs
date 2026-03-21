// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::traffic::packet::clock_now;

/// Number of ping-pong rounds for clock offset estimation.
const SYNC_ROUNDS: usize = 8;

/// Clock synchronization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockMode {
    /// Both endpoints share a kernel — timestamps used directly.
    SameHost,
    /// Clock offset estimated via control channel ping-pong.
    SyncEstimated { offset_ns: i128 },
}

impl ClockMode {
    /// Apply clock correction to a raw one-way latency measurement.
    /// Returns corrected latency in nanoseconds.
    pub fn correct_latency(&self, raw_latency_ns: i128) -> i128 {
        match self {
            ClockMode::SameHost => raw_latency_ns,
            ClockMode::SyncEstimated { offset_ns } => raw_latency_ns - offset_ns,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ClockMode::SameHost => "same-host",
            ClockMode::SyncEstimated { .. } => "sync-estimated",
        }
    }
}

/// Get current monotonic time in nanoseconds.
pub fn mono_now_ns() -> u128 {
    let (sec, nsec) = clock_now();
    sec as u128 * 1_000_000_000 + nsec as u128
}

/// A single clock sync sample: client sends, server responds, client receives.
#[derive(Debug, Clone, Copy)]
pub struct ClockSample {
    pub client_send_ns: u128,
    pub server_recv_ns: u128,
    pub server_send_ns: u128,
    pub client_recv_ns: u128,
}

impl ClockSample {
    /// Estimated round-trip time.
    pub fn rtt_ns(&self) -> u128 {
        self.client_recv_ns - self.client_send_ns
    }

    /// Estimated clock offset: server_time - client_time.
    /// Assumes symmetric path delay.
    pub fn offset_ns(&self) -> i128 {
        let t1 = self.client_send_ns as i128;
        let t2 = self.server_recv_ns as i128;
        let t3 = self.server_send_ns as i128;
        let t4 = self.client_recv_ns as i128;
        ((t2 - t1) + (t3 - t4)) / 2
    }
}

/// Estimate clock offset from multiple samples.
/// Uses the sample with the lowest RTT (most likely to have symmetric delay).
pub fn estimate_offset(samples: &[ClockSample]) -> Option<i128> {
    samples
        .iter()
        .min_by_key(|s| s.rtt_ns())
        .map(|s| s.offset_ns())
}

/// Number of rounds to perform for clock sync.
pub fn sync_rounds() -> usize {
    SYNC_ROUNDS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_host_no_correction() {
        let mode = ClockMode::SameHost;
        assert_eq!(mode.correct_latency(1000), 1000);
    }

    #[test]
    fn estimated_offset_correction() {
        let mode = ClockMode::SyncEstimated { offset_ns: 500 };
        assert_eq!(mode.correct_latency(1500), 1000);
    }

    #[test]
    fn clock_sample_offset() {
        let sample = ClockSample {
            client_send_ns: 1000,
            server_recv_ns: 1600,
            server_send_ns: 1700,
            client_recv_ns: 2200,
        };
        // RTT = 2200 - 1000 = 1200
        assert_eq!(sample.rtt_ns(), 1200);
        // offset = ((1600-1000) + (1700-2200)) / 2 = (600 + (-500)) / 2 = 50
        assert_eq!(sample.offset_ns(), 50);
    }

    #[test]
    fn estimate_picks_lowest_rtt() {
        let samples = vec![
            ClockSample {
                client_send_ns: 0,
                server_recv_ns: 1000,
                server_send_ns: 1000,
                client_recv_ns: 5000, // RTT = 5000
            },
            ClockSample {
                client_send_ns: 10000,
                server_recv_ns: 10500,
                server_send_ns: 10500,
                client_recv_ns: 11000, // RTT = 1000 (best)
            },
        ];
        let offset = estimate_offset(&samples).unwrap();
        // From best sample: ((10500-10000) + (10500-11000)) / 2 = (500 + (-500)) / 2 = 0
        assert_eq!(offset, 0);
    }

    #[test]
    fn mono_now_returns_nonzero() {
        assert!(mono_now_ns() > 0);
    }
}
