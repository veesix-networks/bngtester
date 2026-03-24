// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::protocol::TrafficPattern;
use crate::traffic::packet::HEADER_SIZE;

/// Parse a "ID=BYTES" string for per-stream packet size override.
/// Size must be >= HEADER_SIZE (32 bytes).
pub fn parse_stream_size(s: &str) -> Result<(u8, u32), String> {
    let (id_str, val_str) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid stream-size format: '{s}' (expected ID=BYTES)"))?;
    let id: u8 = id_str
        .parse()
        .map_err(|_| format!("invalid stream ID: '{id_str}'"))?;
    let size: u32 = val_str
        .parse()
        .map_err(|_| format!("invalid size value: '{val_str}'"))?;
    if (size as usize) < HEADER_SIZE {
        return Err(format!(
            "stream size {size} is below minimum ({HEADER_SIZE} bytes)"
        ));
    }
    Ok((id, size))
}

/// Parse a "ID=PPS" string for per-stream rate override.
/// 0 is valid and means unlimited.
pub fn parse_stream_rate(s: &str) -> Result<(u8, u32), String> {
    let (id_str, val_str) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid stream-rate format: '{s}' (expected ID=PPS)"))?;
    let id: u8 = id_str
        .parse()
        .map_err(|_| format!("invalid stream ID: '{id_str}'"))?;
    let rate: u32 = val_str
        .parse()
        .map_err(|_| format!("invalid rate value: '{val_str}'"))?;
    Ok((id, rate))
}

/// Parse a "ID=PATTERN" string for per-stream traffic pattern override.
pub fn parse_stream_pattern(s: &str) -> Result<(u8, TrafficPattern), String> {
    let (id_str, val_str) = s
        .split_once('=')
        .ok_or_else(|| format!("invalid stream-pattern format: '{s}' (expected ID=PATTERN)"))?;
    let id: u8 = id_str
        .parse()
        .map_err(|_| format!("invalid stream ID: '{id_str}'"))?;
    let pattern = match val_str.to_lowercase().as_str() {
        "fixed" => TrafficPattern::Fixed,
        "imix" => TrafficPattern::Imix,
        "sweep" => TrafficPattern::Sweep,
        _ => {
            return Err(format!(
                "invalid traffic pattern: '{val_str}'. Must be: fixed, imix, sweep"
            ))
        }
    };
    Ok((id, pattern))
}

/// Collection of per-stream overrides for size, rate, pattern, and DSCP.
#[derive(Debug, Default)]
pub struct StreamOverrides {
    pub sizes: Vec<(u8, u32)>,
    pub rates: Vec<(u8, u32)>,
    pub patterns: Vec<(u8, TrafficPattern)>,
    pub dscps: Vec<(u8, u8)>,
}

/// Resolved configuration for a single stream.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStreamConfig {
    pub size: u32,
    pub rate_pps: u32,
    pub pattern: TrafficPattern,
    pub dscp: Option<u8>,
}

impl StreamOverrides {
    /// Resolve the effective configuration for a given stream ID.
    /// Uses last-match-wins semantics for each field.
    pub fn resolve(
        &self,
        stream_id: u8,
        global_size: u32,
        global_rate: u32,
        global_pattern: TrafficPattern,
        global_dscp: Option<u8>,
    ) -> ResolvedStreamConfig {
        let size = self
            .sizes
            .iter()
            .rev()
            .find(|(id, _)| *id == stream_id)
            .map(|(_, v)| *v)
            .unwrap_or(global_size);

        let rate_pps = self
            .rates
            .iter()
            .rev()
            .find(|(id, _)| *id == stream_id)
            .map(|(_, v)| *v)
            .unwrap_or(global_rate);

        let pattern = self
            .patterns
            .iter()
            .rev()
            .find(|(id, _)| *id == stream_id)
            .map(|(_, v)| *v)
            .unwrap_or(global_pattern);

        let dscp = self
            .dscps
            .iter()
            .rev()
            .find(|(id, _)| *id == stream_id)
            .map(|(_, v)| Some(*v))
            .unwrap_or(global_dscp);

        ResolvedStreamConfig {
            size,
            rate_pps,
            pattern,
            dscp,
        }
    }

    /// Returns true if any overrides exist for the given stream ID.
    pub fn has_overrides(&self, stream_id: u8) -> bool {
        self.sizes.iter().any(|(id, _)| *id == stream_id)
            || self.rates.iter().any(|(id, _)| *id == stream_id)
            || self.patterns.iter().any(|(id, _)| *id == stream_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_valid() {
        let (id, size) = parse_stream_size("0=64").unwrap();
        assert_eq!(id, 0);
        assert_eq!(size, 64);
    }

    #[test]
    fn parse_size_at_minimum() {
        let (id, size) = parse_stream_size("1=32").unwrap();
        assert_eq!(id, 1);
        assert_eq!(size, 32);
    }

    #[test]
    fn parse_size_below_minimum() {
        assert!(parse_stream_size("0=16").is_err());
        assert!(parse_stream_size("0=31").is_err());
        assert!(parse_stream_size("0=0").is_err());
    }

    #[test]
    fn parse_size_invalid_format() {
        assert!(parse_stream_size("no_equals").is_err());
        assert!(parse_stream_size("abc=64").is_err());
        assert!(parse_stream_size("0=abc").is_err());
    }

    #[test]
    fn parse_rate_valid() {
        let (id, rate) = parse_stream_rate("1=10000").unwrap();
        assert_eq!(id, 1);
        assert_eq!(rate, 10000);
    }

    #[test]
    fn parse_rate_unlimited() {
        let (id, rate) = parse_stream_rate("0=0").unwrap();
        assert_eq!(id, 0);
        assert_eq!(rate, 0);
    }

    #[test]
    fn parse_rate_invalid() {
        assert!(parse_stream_rate("no_equals").is_err());
        assert!(parse_stream_rate("abc=100").is_err());
        assert!(parse_stream_rate("0=abc").is_err());
    }

    #[test]
    fn parse_pattern_valid() {
        let (id, p) = parse_stream_pattern("0=fixed").unwrap();
        assert_eq!(id, 0);
        assert_eq!(p, TrafficPattern::Fixed);

        let (id, p) = parse_stream_pattern("1=imix").unwrap();
        assert_eq!(id, 1);
        assert_eq!(p, TrafficPattern::Imix);

        let (id, p) = parse_stream_pattern("2=sweep").unwrap();
        assert_eq!(id, 2);
        assert_eq!(p, TrafficPattern::Sweep);
    }

    #[test]
    fn parse_pattern_case_insensitive() {
        let (_, p) = parse_stream_pattern("0=IMIX").unwrap();
        assert_eq!(p, TrafficPattern::Imix);

        let (_, p) = parse_stream_pattern("0=Fixed").unwrap();
        assert_eq!(p, TrafficPattern::Fixed);
    }

    #[test]
    fn parse_pattern_invalid() {
        assert!(parse_stream_pattern("no_equals").is_err());
        assert!(parse_stream_pattern("0=invalid").is_err());
        assert!(parse_stream_pattern("abc=fixed").is_err());
    }

    #[test]
    fn resolve_no_overrides() {
        let overrides = StreamOverrides::default();
        let resolved = overrides.resolve(0, 512, 100, TrafficPattern::Fixed, Some(46));
        assert_eq!(resolved.size, 512);
        assert_eq!(resolved.rate_pps, 100);
        assert_eq!(resolved.pattern, TrafficPattern::Fixed);
        assert_eq!(resolved.dscp, Some(46));
    }

    #[test]
    fn resolve_with_overrides() {
        let overrides = StreamOverrides {
            sizes: vec![(0, 64)],
            rates: vec![(0, 10000)],
            patterns: vec![(0, TrafficPattern::Imix)],
            dscps: vec![(0, 34)],
        };
        let resolved = overrides.resolve(0, 512, 100, TrafficPattern::Fixed, Some(46));
        assert_eq!(resolved.size, 64);
        assert_eq!(resolved.rate_pps, 10000);
        assert_eq!(resolved.pattern, TrafficPattern::Imix);
        assert_eq!(resolved.dscp, Some(34));
    }

    #[test]
    fn resolve_last_match_wins() {
        let overrides = StreamOverrides {
            sizes: vec![(0, 64), (0, 128)],
            rates: vec![(0, 100), (0, 200)],
            patterns: vec![(0, TrafficPattern::Imix), (0, TrafficPattern::Sweep)],
            dscps: vec![(0, 34), (0, 46)],
        };
        let resolved = overrides.resolve(0, 512, 50, TrafficPattern::Fixed, None);
        assert_eq!(resolved.size, 128);
        assert_eq!(resolved.rate_pps, 200);
        assert_eq!(resolved.pattern, TrafficPattern::Sweep);
        assert_eq!(resolved.dscp, Some(46));
    }

    #[test]
    fn resolve_different_streams() {
        let overrides = StreamOverrides {
            sizes: vec![(0, 64), (1, 1518)],
            rates: vec![(0, 10000)],
            patterns: vec![],
            dscps: vec![],
        };
        let r0 = overrides.resolve(0, 512, 100, TrafficPattern::Fixed, None);
        assert_eq!(r0.size, 64);
        assert_eq!(r0.rate_pps, 10000);

        let r1 = overrides.resolve(1, 512, 100, TrafficPattern::Fixed, None);
        assert_eq!(r1.size, 1518);
        assert_eq!(r1.rate_pps, 100); // falls back to global

        let r2 = overrides.resolve(2, 512, 100, TrafficPattern::Fixed, None);
        assert_eq!(r2.size, 512); // falls back to global
    }

    #[test]
    fn has_overrides_detection() {
        let overrides = StreamOverrides {
            sizes: vec![(0, 64)],
            rates: vec![],
            patterns: vec![(1, TrafficPattern::Imix)],
            dscps: vec![],
        };
        assert!(overrides.has_overrides(0));
        assert!(overrides.has_overrides(1));
        assert!(!overrides.has_overrides(2));
    }

    #[test]
    fn resolve_global_dscp_none() {
        let overrides = StreamOverrides::default();
        let resolved = overrides.resolve(0, 512, 100, TrafficPattern::Fixed, None);
        assert_eq!(resolved.dscp, None);
    }
}
