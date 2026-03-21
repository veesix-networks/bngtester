// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Serialize;
use std::io::Write;

/// A single per-packet record for JSONL export.
#[derive(Debug, Serialize)]
pub struct PacketRecord {
    pub stream: u8,
    pub seq: u32,
    pub send_ts_ns: u128,
    pub recv_ts_ns: u128,
    pub size: u32,
    pub latency_ns: i128,
}

/// Writer for per-packet JSONL output.
pub struct JsonlWriter<W: Write> {
    writer: W,
}

impl<W: Write> JsonlWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a single packet record as one JSON line.
    pub fn write_record(&mut self, record: &PacketRecord) -> std::io::Result<()> {
        serde_json::to_writer(&mut self.writer, record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    /// Flush the underlying writer.
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_single_record() {
        let mut buf = Vec::new();
        let mut w = JsonlWriter::new(&mut buf);
        w.write_record(&PacketRecord {
            stream: 0,
            seq: 1,
            send_ts_ns: 1000000000,
            recv_ts_ns: 1000000450,
            size: 64,
            latency_ns: 450,
        })
        .unwrap();
        let line = String::from_utf8(buf).unwrap();
        assert!(line.ends_with('\n'));
        let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed["stream"], 0);
        assert_eq!(parsed["seq"], 1);
        assert_eq!(parsed["latency_ns"], 450);
    }

    #[test]
    fn jsonl_multiple_records() {
        let mut buf = Vec::new();
        let mut w = JsonlWriter::new(&mut buf);
        for i in 0..3 {
            w.write_record(&PacketRecord {
                stream: 0,
                seq: i,
                send_ts_ns: i as u128 * 1000,
                recv_ts_ns: i as u128 * 1000 + 500,
                size: 64,
                latency_ns: 500,
            })
            .unwrap();
        }
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.trim().lines().collect();
        assert_eq!(lines.len(), 3);
    }
}
