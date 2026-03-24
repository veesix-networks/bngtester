# Implementation Spec: Per-Stream Configuration (Size, Rate, Pattern Overrides)

## Overview

Allow each data stream in a test to have its own packet size, rate, and traffic pattern instead of all streams sharing global defaults. This enables mixed-traffic tests — e.g., one stream sending small VoIP-like packets at high rate while another sends bulk IMIX. Per-stream overrides are communicated to the server via a unified `StreamConfigOverride` in the hello message (consolidating the existing per-stream DSCP from #32) and shown in report output. Scoped to the current UDP stream path — TCP/RRUL multi-stream wiring is future work.

## Source Issue

[#34 — Per-stream configuration (size, rate, pattern overrides)](https://github.com/veesix-networks/bngtester/issues/34)

## Current State

- `UdpGeneratorConfig` has `packet_size`, `rate_pps`, and `pattern` fields.
- The client CLI has global `--size`, `--rate`, and `--pattern` flags.
- All streams in a test use the same values.
- Per-stream DSCP (`--stream-dscp`) and ECN (`--ecn`) exist from #32/#33.
- The `HelloMsg` has separate `stream_dscp` and `stream_config` vectors — these will be consolidated.
- The current codebase only creates one UDP stream (stream 0). RRUL mode is spec'd for multiple streams but not yet implemented.

## Design

### CLI Flags

Three new repeatable CLI flags, following the same `ID=VALUE` pattern as `--stream-dscp`:

| Flag | Default | Description |
|------|---------|-------------|
| `--stream-size <ID>=<BYTES>` | _(global --size)_ | Override packet size for stream ID. Must be >= 32 (packet header size). |
| `--stream-rate <ID>=<PPS>` | _(global --rate)_ | Override rate for stream ID. 0 = unlimited. |
| `--stream-pattern <ID>=<PATTERN>` | _(global --pattern)_ | Override traffic pattern for stream ID. |

Unoverridden streams use the global defaults.

### Validation

- **Size:** Must be >= `HEADER_SIZE` (32 bytes). Sizes below this are rejected at parse time with a clear error — not silently clamped by `build_packet()`.
- **Rate:** 0 is valid and means unlimited (send as fast as possible). Rendered as "unlimited" in text reports, `0` in JSON.
- **Pattern:** Must be one of `fixed`, `imix`, `sweep`.

### Resolution Logic

Resolution uses "last match wins" for CLI consistency — if `--stream-size 0=64 --stream-size 0=128` is given, stream 0 gets 128. A `resolve_stream_config()` function in `src/stream/config.rs` resolves all fields (size, rate, pattern, dscp, ecn) for a given stream ID.

### Consolidation of Per-Stream Overrides

The existing `stream_dscp: Vec<StreamDscpConfig>` in `HelloMsg` is consolidated into a unified `StreamConfigOverride`:

```rust
pub struct StreamConfigOverride {
    pub stream_id: u8,
    pub size: Option<u32>,
    pub rate_pps: Option<u32>,
    pub pattern: Option<TrafficPattern>,
    pub dscp: Option<u8>,
}
```

This replaces the separate `StreamDscpConfig` struct. The `--stream-dscp` CLI flag is preserved — it populates the `dscp` field within `StreamConfigOverride`. Only overridden fields are sent — `None` means "use global default".

### Scope: Current UDP Path Only

This spec covers the currently implemented UDP stream path (stream 0 in latency mode). TCP generator config (`TcpGeneratorConfig`) does not carry size/rate/pattern — extending it is future work when RRUL multi-stream is implemented.

### Report Changes

Add per-stream config to `StreamReport` (not `StreamResults` — config is input metadata, not measurement):

```rust
pub struct StreamReport {
    // ... existing fields ...
    pub config: Option<StreamConfigReport>,
}

pub struct StreamConfigReport {
    pub size: u32,
    pub rate_pps: u32,      // 0 = unlimited
    pub pattern: String,
}
```

Text output when overrides are active:
```
  Stream 0 [UDP latency ↑ DSCP=EF 64B@10000pps fixed]
  Stream 1 [UDP latency ↑ 1518B@unlimited imix]
```

When no overrides — config section omitted from report (backward compatible).

### RRUL Stream ID Mapping (Future Reference)

When RRUL multi-stream is implemented, the stream ID mapping will be:
- 0: UDP latency probe (upstream)
- 1: UDP latency probe (downstream)
- 2-3: TCP throughput (upstream)
- 4-5: TCP throughput (downstream)

This is documented here for future reference but not enforced in this spec since RRUL multi-stream is not yet implemented.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/stream/config.rs` | Create | `StreamConfig`, `resolve_stream_config()`, parsing helpers for size/rate/pattern overrides |
| `src/stream/mod.rs` | Modify | Add `pub mod config;` |
| `src/protocol/mod.rs` | Modify | Consolidate `stream_dscp` into `StreamConfigOverride`, remove `StreamDscpConfig` |
| `src/bin/client.rs` | Modify | Add 3 new CLI flags, consolidate overrides, resolve per-stream config |
| `src/bin/server.rs` | Modify | Read per-stream config from hello, include in report |
| `src/report/mod.rs` | Modify | Add `config` and `StreamConfigReport` to `StreamReport` |
| `src/report/text.rs` | Modify | Show per-stream config in stream header when overrides active |
| `src/report/json.rs` | Modify | Update test constructors |
| `src/report/junit.rs` | Modify | Update test constructors |

## Implementation Order

1. `src/stream/config.rs` — `StreamConfig`, parsing helpers with validation, `resolve_stream_config()` with last-match-wins
2. Protocol changes — consolidate `StreamDscpConfig` into `StreamConfigOverride`, remove old struct
3. CLI changes — 3 new flags, consolidate all per-stream overrides
4. Report changes — `StreamConfigReport` in `StreamReport`, text output
5. Server changes — read from hello, include in report
6. Update `--stream-dscp` callers to use consolidated struct

## Testing

- [ ] `parse_stream_size("0=64")` returns (0, 64)
- [ ] `parse_stream_size("0=16")` rejected (below HEADER_SIZE)
- [ ] `parse_stream_rate("1=10000")` returns (1, 10000)
- [ ] `parse_stream_rate("0=0")` returns (0, 0) — unlimited is valid
- [ ] `parse_stream_pattern("0=imix")` returns (0, Imix)
- [ ] Invalid formats rejected with clear error
- [ ] Resolution: last match wins for repeated IDs
- [ ] Resolution: unoverridden stream uses global default
- [ ] `StreamConfigOverride` includes dscp (consolidation from #32)
- [ ] Per-stream config in HelloMsg serialization round-trip
- [ ] Per-stream config shown in text report when overrides active
- [ ] Rate 0 rendered as "unlimited" in text report
- [ ] Per-stream config in JSON report
- [ ] No overrides = no config in report (backward compatible)
- [ ] `cargo test` passes all existing + new tests
- [ ] End-to-end: client with --stream-size 0=64 sends 64-byte packets

## Not In Scope

- Config file format (CLI flags only)
- Dynamic stream reconfiguration during a test
- TCP generator per-stream config (future work when RRUL multi-stream exists)
- Enforcing stream IDs match actual streams (resolved at generator creation time)
