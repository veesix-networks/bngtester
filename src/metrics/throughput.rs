// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

/// Tracks throughput: bytes and packets over time.
pub struct ThroughputTracker {
    total_bytes: u64,
    total_packets: u64,
    start_ns: Option<u128>,
    last_ns: Option<u128>,
}

impl ThroughputTracker {
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            total_packets: 0,
            start_ns: None,
            last_ns: None,
        }
    }

    /// Record a received packet.
    pub fn record(&mut self, bytes: u64, timestamp_ns: u128) {
        self.total_bytes += bytes;
        self.total_packets += 1;
        if self.start_ns.is_none() {
            self.start_ns = Some(timestamp_ns);
        }
        self.last_ns = Some(timestamp_ns);
    }

    /// Total bytes received.
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Total packets received.
    pub fn total_packets(&self) -> u64 {
        self.total_packets
    }

    /// Duration in nanoseconds between first and last packet.
    pub fn duration_ns(&self) -> u128 {
        match (self.start_ns, self.last_ns) {
            (Some(start), Some(last)) if last > start => last - start,
            _ => 0,
        }
    }

    /// Throughput in bits per second.
    pub fn bits_per_sec(&self) -> u64 {
        let dur = self.duration_ns();
        if dur == 0 {
            return 0;
        }
        (self.total_bytes as u128 * 8 * 1_000_000_000 / dur) as u64
    }

    /// Throughput in packets per second.
    pub fn packets_per_sec(&self) -> u64 {
        let dur = self.duration_ns();
        if dur == 0 {
            return 0;
        }
        (self.total_packets as u128 * 1_000_000_000 / dur) as u64
    }
}

impl Default for ThroughputTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tracker() {
        let t = ThroughputTracker::new();
        assert_eq!(t.total_bytes(), 0);
        assert_eq!(t.bits_per_sec(), 0);
        assert_eq!(t.packets_per_sec(), 0);
    }

    #[test]
    fn single_packet_zero_throughput() {
        let mut t = ThroughputTracker::new();
        t.record(1000, 1_000_000_000);
        // Single packet = zero duration = zero rate
        assert_eq!(t.bits_per_sec(), 0);
    }

    #[test]
    fn known_throughput() {
        let mut t = ThroughputTracker::new();
        // 1000 bytes over 1 second = 8000 bps
        t.record(500, 0);
        t.record(500, 1_000_000_000);
        assert_eq!(t.total_bytes(), 1000);
        assert_eq!(t.bits_per_sec(), 8000);
        assert_eq!(t.packets_per_sec(), 2);
    }

    #[test]
    fn high_rate() {
        let mut t = ThroughputTracker::new();
        // Simulate 1Gbps: ~125MB in 1 second
        let bytes_per_pkt: u64 = 1500;
        let pkts = 83333; // ~125MB
        let ns_per_pkt = 1_000_000_000u128 / pkts;
        for i in 0..pkts {
            t.record(bytes_per_pkt, i * ns_per_pkt);
        }
        let bps = t.bits_per_sec();
        // Should be approximately 1Gbps
        assert!(bps > 900_000_000, "got {bps} bps");
        assert!(bps < 1_100_000_000, "got {bps} bps");
    }
}
