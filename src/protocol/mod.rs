// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod clock;
pub mod session;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Control protocol message, length-prefixed JSON over TCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Hello(HelloMsg),
    Ready(ReadyMsg),
    ClockSync(ClockSyncMsg),
    Start(StartMsg),
    Heartbeat,
    Stop,
    Results(ResultsMsg),
    Error(ErrorMsg),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloMsg {
    pub mode: TestMode,
    pub protocol: Protocol,
    pub duration_secs: u32,
    pub packet_size: u32,
    pub rate_pps: u32,
    pub pattern: TrafficPattern,
    pub streams_per_direction: u32,
    pub rrul_baseline_secs: u32,
    pub rrul_ramp_up_ms: u32,
    pub cross_host: bool,
}

/// Port assignment for a specific stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAssignment {
    pub stream_id: u8,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadyMsg {
    pub udp_port: u16,
    pub tcp_ports: Vec<PortAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockSyncMsg {
    pub client_send_ns: u128,
    pub server_recv_ns: Option<u128>,
    pub server_send_ns: Option<u128>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMsg {
    pub client_udp_port: Option<u16>,
    pub client_tcp_ports: Vec<PortAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsMsg {
    pub status: SessionStatus,
    pub streams: Vec<StreamResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResult {
    pub stream_id: u8,
    pub status: StreamStatus,
    pub packets_received: u64,
    pub packets_lost: u64,
    pub packets_reordered: u64,
    pub latency_ns: Option<LatencyStats>,
    pub jitter_ns: Option<f64>,
    pub throughput_bps: u64,
    pub throughput_pps: u64,
    pub tcp_info: Option<TcpStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min: f64,
    pub avg: f64,
    pub max: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpStats {
    pub rtt_us: f64,
    pub rtt_var_us: f64,
    pub retransmissions: u32,
    pub cwnd_max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMsg {
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestMode {
    Throughput,
    Latency,
    Rrul,
    Bidirectional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficPattern {
    Fixed,
    Imix,
    Sweep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Complete,
    Interrupted,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamStatus {
    Complete,
    Failed,
    EarlyExit,
}

/// Write a length-prefixed JSON message to an async writer.
pub async fn write_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &Message,
) -> std::io::Result<()> {
    let json = serde_json::to_vec(msg).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let len = json.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&json).await?;
    writer.flush().await?;
    Ok(())
}

/// Read a length-prefixed JSON message from an async reader.
/// Returns None on clean EOF.
pub async fn read_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<Option<Message>> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > 1_048_576 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("message too large: {len} bytes"),
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    let msg: Message = serde_json::from_slice(&buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn message_round_trip() {
        let msg = Message::Heartbeat;
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let decoded = read_message(&mut cursor).await.unwrap().unwrap();
        assert!(matches!(decoded, Message::Heartbeat));
    }

    #[tokio::test]
    async fn hello_round_trip() {
        let msg = Message::Hello(HelloMsg {
            mode: TestMode::Rrul,
            protocol: Protocol::Tcp,
            duration_secs: 30,
            packet_size: 512,
            rate_pps: 100,
            pattern: TrafficPattern::Fixed,
            streams_per_direction: 2,
            rrul_baseline_secs: 5,
            rrul_ramp_up_ms: 100,
            cross_host: false,
        });
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let decoded = read_message(&mut cursor).await.unwrap().unwrap();
        match decoded {
            Message::Hello(h) => {
                assert_eq!(h.mode, TestMode::Rrul);
                assert_eq!(h.duration_secs, 30);
                assert_eq!(h.streams_per_direction, 2);
            }
            _ => panic!("expected Hello"),
        }
    }

    #[tokio::test]
    async fn ready_with_ports() {
        let tcp_ports = vec![
            PortAssignment { stream_id: 0, port: 5002 },
            PortAssignment { stream_id: 1, port: 5003 },
        ];
        let msg = Message::Ready(ReadyMsg {
            udp_port: 5001,
            tcp_ports,
        });
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let decoded = read_message(&mut cursor).await.unwrap().unwrap();
        match decoded {
            Message::Ready(r) => {
                assert_eq!(r.udp_port, 5001);
                assert_eq!(r.tcp_ports.len(), 2);
                assert_eq!(r.tcp_ports[0].stream_id, 0);
                assert_eq!(r.tcp_ports[0].port, 5002);
            }
            _ => panic!("expected Ready"),
        }
    }

    #[tokio::test]
    async fn eof_returns_none() {
        let buf: Vec<u8> = Vec::new();
        let mut cursor = std::io::Cursor::new(buf);
        let result = read_message(&mut cursor).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn error_message() {
        let msg = Message::Error(ErrorMsg {
            reason: "test failure".to_string(),
        });
        let mut buf = Vec::new();
        write_message(&mut buf, &msg).await.unwrap();

        let mut cursor = std::io::Cursor::new(buf);
        let decoded = read_message(&mut cursor).await.unwrap().unwrap();
        match decoded {
            Message::Error(e) => assert_eq!(e.reason, "test failure"),
            _ => panic!("expected Error"),
        }
    }
}
