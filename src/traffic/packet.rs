// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

/// Magic bytes: "BNGT" (0x424E4754)
pub const MAGIC: u32 = 0x424E4754;

/// Current protocol version.
pub const VERSION: u8 = 1;

/// Packet header size in bytes.
pub const HEADER_SIZE: usize = 32;

/// Flag: last packet in stream.
pub const FLAG_LAST: u16 = 0x01;

/// Threshold for detecting sequence number wrap-around vs loss.
/// Gaps larger than this are treated as wraps.
pub const SEQ_WRAP_THRESHOLD: u32 = u32::MAX / 2;

/// Wire-format packet header (32 bytes, big-endian).
///
/// ```text
/// [magic:4][version:1][stream_id:1][flags:2][seq:4][ts_sec:8][ts_nsec:4][payload_len:4][padding...]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub stream_id: u8,
    pub flags: u16,
    pub seq: u32,
    pub ts_sec: u64,
    pub ts_nsec: u32,
    pub payload_len: u32,
}

impl PacketHeader {
    /// Serialize header into a byte buffer (must be at least HEADER_SIZE bytes).
    pub fn write_to(&self, buf: &mut [u8]) {
        assert!(buf.len() >= HEADER_SIZE);
        buf[0..4].copy_from_slice(&MAGIC.to_be_bytes());
        buf[4] = VERSION;
        buf[5] = self.stream_id;
        buf[6..8].copy_from_slice(&self.flags.to_be_bytes());
        buf[8..12].copy_from_slice(&self.seq.to_be_bytes());
        buf[12..20].copy_from_slice(&self.ts_sec.to_be_bytes());
        buf[20..24].copy_from_slice(&self.ts_nsec.to_be_bytes());
        buf[24..28].copy_from_slice(&self.payload_len.to_be_bytes());
        // bytes 28..32 are reserved (zero-filled by caller)
    }

    /// Deserialize header from a byte buffer. Returns None if magic or version mismatch.
    pub fn read_from(buf: &[u8]) -> Option<Self> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != MAGIC {
            return None;
        }
        let version = buf[4];
        if version != VERSION {
            return None;
        }
        Some(PacketHeader {
            stream_id: buf[5],
            flags: u16::from_be_bytes([buf[6], buf[7]]),
            seq: u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]),
            ts_sec: u64::from_be_bytes([
                buf[12], buf[13], buf[14], buf[15], buf[16], buf[17], buf[18], buf[19],
            ]),
            ts_nsec: u32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]),
            payload_len: u32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]),
        })
    }

    /// Get the timestamp as total nanoseconds.
    pub fn timestamp_ns(&self) -> u128 {
        self.ts_sec as u128 * 1_000_000_000 + self.ts_nsec as u128
    }

    /// Check if this is the last packet in the stream.
    pub fn is_last(&self) -> bool {
        self.flags & FLAG_LAST != 0
    }
}

/// Build a packet buffer of the requested total size with the header filled in.
/// Padding bytes after the header are zeroed.
pub fn build_packet(header: &PacketHeader, total_size: usize) -> Vec<u8> {
    let size = total_size.max(HEADER_SIZE);
    let mut buf = vec![0u8; size];
    let mut h = *header;
    h.payload_len = size as u32;
    h.write_to(&mut buf);
    buf
}

/// Get current CLOCK_MONOTONIC timestamp as (seconds, nanoseconds).
pub fn clock_now() -> (u64, u32) {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime with CLOCK_MONOTONIC is always valid, ts is a valid pointer.
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
    }
    (ts.tv_sec as u64, ts.tv_nsec as u32)
}

/// Determine if a sequence number gap represents a wrap-around rather than loss.
/// Returns true if `new_seq` appears to have wrapped relative to `last_seq`.
pub fn is_seq_wrap(last_seq: u32, new_seq: u32) -> bool {
    last_seq > new_seq && (last_seq - new_seq) > SEQ_WRAP_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip() {
        let header = PacketHeader {
            stream_id: 3,
            flags: FLAG_LAST,
            seq: 42,
            ts_sec: 1234567890,
            ts_nsec: 999999999,
            payload_len: 512,
        };
        let mut buf = [0u8; HEADER_SIZE];
        header.write_to(&mut buf);
        let decoded = PacketHeader::read_from(&buf).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn bad_magic_returns_none() {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..4].copy_from_slice(&0xDEADBEEFu32.to_be_bytes());
        assert!(PacketHeader::read_from(&buf).is_none());
    }

    #[test]
    fn short_buffer_returns_none() {
        let buf = [0u8; 16];
        assert!(PacketHeader::read_from(&buf).is_none());
    }

    #[test]
    fn timestamp_ns_calculation() {
        let header = PacketHeader {
            stream_id: 0,
            flags: 0,
            seq: 0,
            ts_sec: 1,
            ts_nsec: 500_000_000,
            payload_len: 32,
        };
        assert_eq!(header.timestamp_ns(), 1_500_000_000);
    }

    #[test]
    fn build_packet_pads_to_size() {
        let header = PacketHeader {
            stream_id: 0,
            flags: 0,
            seq: 1,
            ts_sec: 0,
            ts_nsec: 0,
            payload_len: 0,
        };
        let pkt = build_packet(&header, 64);
        assert_eq!(pkt.len(), 64);
        let decoded = PacketHeader::read_from(&pkt).unwrap();
        assert_eq!(decoded.payload_len, 64);
    }

    #[test]
    fn build_packet_minimum_size() {
        let header = PacketHeader {
            stream_id: 0,
            flags: 0,
            seq: 1,
            ts_sec: 0,
            ts_nsec: 0,
            payload_len: 0,
        };
        let pkt = build_packet(&header, 10); // smaller than HEADER_SIZE
        assert_eq!(pkt.len(), HEADER_SIZE);
    }

    #[test]
    fn clock_now_returns_nonzero() {
        let (sec, nsec) = clock_now();
        // Monotonic clock should be positive after boot
        assert!(sec > 0 || nsec > 0);
    }

    #[test]
    fn seq_wrap_detection() {
        // Normal forward progress
        assert!(!is_seq_wrap(100, 101));
        assert!(!is_seq_wrap(100, 200));

        // Reordering (small backward jump, not a wrap)
        assert!(!is_seq_wrap(100, 99));
        assert!(!is_seq_wrap(1000, 500));

        // Actual wrap-around
        assert!(is_seq_wrap(u32::MAX - 10, 5));
        assert!(is_seq_wrap(u32::MAX, 0));
    }

    #[test]
    fn is_last_flag() {
        let h = PacketHeader {
            stream_id: 0,
            flags: FLAG_LAST,
            seq: 0,
            ts_sec: 0,
            ts_nsec: 0,
            payload_len: 32,
        };
        assert!(h.is_last());

        let h2 = PacketHeader {
            stream_id: 0,
            flags: 0,
            seq: 0,
            ts_sec: 0,
            ts_nsec: 0,
            payload_len: 32,
        };
        assert!(!h2.is_last());
    }
}
