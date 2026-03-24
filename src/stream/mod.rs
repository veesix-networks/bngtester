// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod config;

use crate::protocol::{PortAssignment, StreamStatus};

/// Tracks the state of a single data stream.
#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub stream_id: u8,
    pub port: u16,
    pub direction: StreamDirection,
    pub stream_type: StreamType,
    pub status: StreamStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamDirection {
    Upstream,
    Downstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    UdpLatency,
    UdpThroughput,
    TcpThroughput,
}

/// Registry that maps stream IDs to their configuration and ports.
pub struct StreamRegistry {
    streams: Vec<StreamInfo>,
}

impl StreamRegistry {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    /// Register a new stream.
    pub fn register(
        &mut self,
        stream_id: u8,
        port: u16,
        direction: StreamDirection,
        stream_type: StreamType,
    ) {
        self.streams.push(StreamInfo {
            stream_id,
            port,
            direction,
            stream_type,
            status: StreamStatus::Complete, // default, updated on failure
        });
    }

    /// Update a stream's status.
    pub fn set_status(&mut self, stream_id: u8, status: StreamStatus) {
        if let Some(s) = self.streams.iter_mut().find(|s| s.stream_id == stream_id) {
            s.status = status;
        }
    }

    /// Get a stream by ID.
    pub fn get(&self, stream_id: u8) -> Option<&StreamInfo> {
        self.streams.iter().find(|s| s.stream_id == stream_id)
    }

    /// Get all streams.
    pub fn all(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// Get port assignments for upstream TCP streams (for ready message).
    pub fn upstream_tcp_ports(&self) -> Vec<PortAssignment> {
        self.streams
            .iter()
            .filter(|s| {
                s.direction == StreamDirection::Upstream
                    && s.stream_type == StreamType::TcpThroughput
            })
            .map(|s| PortAssignment {
                stream_id: s.stream_id,
                port: s.port,
            })
            .collect()
    }

    /// Get port assignments for downstream TCP streams (for start message).
    pub fn downstream_tcp_ports(&self) -> Vec<PortAssignment> {
        self.streams
            .iter()
            .filter(|s| {
                s.direction == StreamDirection::Downstream
                    && s.stream_type == StreamType::TcpThroughput
            })
            .map(|s| PortAssignment {
                stream_id: s.stream_id,
                port: s.port,
            })
            .collect()
    }
}

impl Default for StreamRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mut reg = StreamRegistry::new();
        reg.register(0, 5001, StreamDirection::Upstream, StreamType::UdpLatency);
        reg.register(1, 5002, StreamDirection::Upstream, StreamType::TcpThroughput);

        assert_eq!(reg.all().len(), 2);
        assert_eq!(reg.get(0).unwrap().port, 5001);
        assert_eq!(reg.get(1).unwrap().stream_type, StreamType::TcpThroughput);
        assert!(reg.get(99).is_none());
    }

    #[test]
    fn port_assignments() {
        let mut reg = StreamRegistry::new();
        reg.register(0, 5001, StreamDirection::Upstream, StreamType::UdpLatency);
        reg.register(1, 5002, StreamDirection::Upstream, StreamType::TcpThroughput);
        reg.register(2, 6002, StreamDirection::Downstream, StreamType::TcpThroughput);

        let upstream_tcp = reg.upstream_tcp_ports();
        assert_eq!(upstream_tcp.len(), 1);
        assert_eq!(upstream_tcp[0].stream_id, 1);

        let downstream_tcp = reg.downstream_tcp_ports();
        assert_eq!(downstream_tcp.len(), 1);
        assert_eq!(downstream_tcp[0].stream_id, 2);
    }

    #[test]
    fn update_status() {
        let mut reg = StreamRegistry::new();
        reg.register(0, 5001, StreamDirection::Upstream, StreamType::UdpLatency);
        reg.set_status(0, StreamStatus::Failed);
        assert_eq!(reg.get(0).unwrap().status, StreamStatus::Failed);
    }
}
