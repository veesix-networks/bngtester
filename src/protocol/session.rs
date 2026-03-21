// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use std::time::{Duration, Instant};

/// Heartbeat interval.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// Number of missed heartbeats before session timeout.
pub const HEARTBEAT_MISS_LIMIT: u32 = 3;

/// Heartbeat timeout = interval * miss_limit.
pub const HEARTBEAT_TIMEOUT: Duration =
    Duration::from_secs(HEARTBEAT_INTERVAL.as_secs() * HEARTBEAT_MISS_LIMIT as u64);

/// TCP stream connect timeout.
pub const STREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Graceful shutdown timeout after SIGINT/SIGTERM.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Session states for the control channel state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Initial state before hello is sent/received.
    Init,
    /// Hello sent/received, waiting for ready.
    Negotiating,
    /// Clock sync in progress.
    Syncing,
    /// Ready to start data streams.
    Ready,
    /// Data streams are running.
    Running,
    /// Stop sent, collecting results.
    Collecting,
    /// Results exchanged, session complete.
    Done,
    /// Session failed due to an error.
    Failed(FailureReason),
}

/// Why a session failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
    /// Control channel heartbeat timeout.
    HeartbeatTimeout,
    /// Control channel TCP connection lost.
    ControlChannelLost,
    /// Clock sync failed or produced unreasonable offset.
    ClockSyncFailed,
    /// Interrupted by signal (SIGINT/SIGTERM).
    SignalInterrupt,
    /// Protocol error (unexpected message type).
    ProtocolError,
}

/// Tracks heartbeat state for one side of the connection.
pub struct HeartbeatTracker {
    last_received: Instant,
    last_sent: Instant,
}

impl HeartbeatTracker {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_received: now,
            last_sent: now,
        }
    }

    /// Record that a heartbeat was received.
    pub fn received(&mut self) {
        self.last_received = Instant::now();
    }

    /// Record that a heartbeat was sent.
    pub fn sent(&mut self) {
        self.last_sent = Instant::now();
    }

    /// Check if we should send a heartbeat.
    pub fn should_send(&self) -> bool {
        self.last_sent.elapsed() >= HEARTBEAT_INTERVAL
    }

    /// Check if the remote side has timed out.
    pub fn is_timed_out(&self) -> bool {
        self.last_received.elapsed() >= HEARTBEAT_TIMEOUT
    }

    /// Time until next heartbeat should be sent.
    pub fn time_until_send(&self) -> Duration {
        HEARTBEAT_INTERVAL.saturating_sub(self.last_sent.elapsed())
    }
}

impl Default for HeartbeatTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let tracker = HeartbeatTracker::new();
        assert!(!tracker.is_timed_out());
        assert!(!tracker.should_send());
    }

    #[test]
    fn heartbeat_timeout_constant() {
        assert_eq!(HEARTBEAT_TIMEOUT, Duration::from_secs(15));
    }

    #[test]
    fn session_state_transitions() {
        // Just verify states can be compared
        assert_ne!(SessionState::Init, SessionState::Running);
        assert_eq!(
            SessionState::Failed(FailureReason::HeartbeatTimeout),
            SessionState::Failed(FailureReason::HeartbeatTimeout)
        );
    }
}
