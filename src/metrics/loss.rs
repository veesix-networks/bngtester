// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::traffic::packet::is_seq_wrap;

/// Tracks packet loss and reordering from sequence numbers.
pub struct LossTracker {
    highest_seq: Option<u32>,
    received: u64,
    reordered: u64,
    duplicates: u64,
}

impl LossTracker {
    pub fn new() -> Self {
        Self {
            highest_seq: None,
            received: 0,
            reordered: 0,
            duplicates: 0,
        }
    }

    /// Record a received packet's sequence number.
    pub fn record(&mut self, seq: u32) {
        self.received += 1;

        match self.highest_seq {
            None => {
                self.highest_seq = Some(seq);
            }
            Some(highest) => {
                if is_seq_wrap(highest, seq) {
                    // Wrap-around: seq is after highest in wrapped space
                    self.highest_seq = Some(seq);
                } else if seq > highest {
                    // Normal forward progress
                    self.highest_seq = Some(seq);
                } else if seq < highest {
                    // Out of order (or duplicate)
                    self.reordered += 1;
                } else {
                    // seq == highest — duplicate
                    self.duplicates += 1;
                }
            }
        }
    }

    /// Total packets received.
    pub fn received(&self) -> u64 {
        self.received
    }

    /// Packets detected as out-of-order.
    pub fn reordered(&self) -> u64 {
        self.reordered
    }

    /// Estimated packets lost based on highest sequence seen.
    /// This is an approximation — wraps are handled but late arrivals
    /// after a wrap may inflate the count.
    pub fn estimated_lost(&self) -> u64 {
        match self.highest_seq {
            None => 0,
            Some(highest) => {
                let expected = highest as u64 + 1; // sequences are 0-based
                expected.saturating_sub(self.received)
            }
        }
    }

    /// Loss percentage.
    pub fn loss_percent(&self) -> f64 {
        match self.highest_seq {
            None => 0.0,
            Some(highest) => {
                let expected = highest as u64 + 1;
                if expected == 0 {
                    return 0.0;
                }
                let lost = expected.saturating_sub(self.received);
                lost as f64 / expected as f64 * 100.0
            }
        }
    }

    /// Reorder percentage.
    pub fn reorder_percent(&self) -> f64 {
        if self.received == 0 {
            return 0.0;
        }
        self.reordered as f64 / self.received as f64 * 100.0
    }
}

impl Default for LossTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_loss_sequential() {
        let mut t = LossTracker::new();
        for i in 0..100 {
            t.record(i);
        }
        assert_eq!(t.received(), 100);
        assert_eq!(t.estimated_lost(), 0);
        assert_eq!(t.reordered(), 0);
        assert!(t.loss_percent() < 0.001);
    }

    #[test]
    fn detects_loss() {
        let mut t = LossTracker::new();
        // Send 0, 1, 2, skip 3, send 4
        t.record(0);
        t.record(1);
        t.record(2);
        t.record(4);
        assert_eq!(t.received(), 4);
        assert_eq!(t.estimated_lost(), 1); // seq 3 missing
    }

    #[test]
    fn detects_reordering() {
        let mut t = LossTracker::new();
        t.record(0);
        t.record(1);
        t.record(3); // skip 2
        t.record(2); // out of order
        assert_eq!(t.reordered(), 1);
    }

    #[test]
    fn handles_wrap_around() {
        let mut t = LossTracker::new();
        t.record(u32::MAX - 2);
        t.record(u32::MAX - 1);
        t.record(u32::MAX);
        t.record(0); // wrap
        t.record(1);
        assert_eq!(t.received(), 5);
        assert_eq!(t.reordered(), 0);
    }

    #[test]
    fn empty_tracker() {
        let t = LossTracker::new();
        assert_eq!(t.received(), 0);
        assert_eq!(t.estimated_lost(), 0);
        assert_eq!(t.loss_percent(), 0.0);
        assert_eq!(t.reorder_percent(), 0.0);
    }
}
