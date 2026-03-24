# Implementation Spec: Per-Stream Configuration (Size, Rate, Pattern Overrides)

## Overview

Allow each data stream in a test to have its own packet size, rate, and traffic pattern instead of all streams sharing global defaults. This enables mixed-traffic tests — e.g., one stream sending small VoIP-like packets at high rate while another sends bulk IMIX. Per-stream overrides are communicated to the server via the hello message and shown in report output.

## Source Issue

[#34 — Per-stream configuration (size, rate, pattern overrides)](https://github.com/veesix-networks/bngtester/issues/34)

## Current State

- `UdpGeneratorConfig` has `packet_size`, `rate_pps`, and `pattern` fields.
- The client CLI has global `--size`, `--rate`, and `--pattern` flags.
- All streams in a test use the same values.
- Per-stream DSCP (`--stream-dscp`) already exists from #32 — this extends that pattern.
- The `HelloMsg` carries global config but no per-stream overrides for size/rate/pattern.
- The current codebase only creates one UDP stream (stream 0). RRUL mode is spec'd for multiple streams but not yet implemented. Per-stream config prepares the infrastructure for when multiple concurrent streams exist.

## Design

### CLI Flags

Three new repeatable CLI flags, following the same `ID=VALUE` pattern as `--stream-dscp`:

| Flag | Default | Description |
|------|---------|-------------|
| `--stream-size <ID>=<BYTES>` | _(global --size)_ | Override packet size for stream ID |
| `--stream-rate <ID>=<PPS>` | _(global --rate)_ | Override rate for stream ID |
| `--stream-pattern <ID>=<PATTERN>` | _(global --pattern)_ | Override traffic pattern for stream ID |

Unoverridden streams use the global defaults. Example:
```
bngtester-client 10.0.0.2:5000 --size 512 --rate 100 \
  --stream-size 0=64 --stream-rate 0=10000 --stream-pattern 0=fixed \
  --stream-size 1=1518 --stream-rate 1=500 --stream-pattern 1=imix
```

### Resolution Logic

A `StreamConfig` struct resolves effective values per stream:

```rust
pub struct StreamConfig {
    pub size: usize,
    pub rate_pps: u32,
    pub pattern: TrafficPattern,
    pub dscp: Option<u8>,
    pub ecn: EcnMode,
}
```

Resolution: per-stream override → global default. Same pattern as `resolve_stream_dscp()` from #32. A single `resolve_stream_config()` function resolves all fields for a given stream ID.

### Control Protocol Changes

Add per-stream config to `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub stream_config: Vec<StreamConfigOverride>,
}

pub struct StreamConfigOverride {
    pub stream_id: u8,
    pub size: Option<u32>,
    pub rate_pps: Option<u32>,
    pub pattern: Option<TrafficPattern>,
}
```

Only overridden fields are sent — `None` means "use global default". The server uses this for labeling reports and for configuring reverse-path streams in bidirectional/RRUL modes.

### Report Changes

Add per-stream config to `StreamReport` so the report shows what each stream was configured with:

```rust
pub struct StreamReport {
    // ... existing fields ...
    pub config: Option<StreamConfigReport>,
}

pub struct StreamConfigReport {
    pub size: usize,
    pub rate_pps: u32,
    pub pattern: String,
}
```

Text output:
```
  Stream 0 [UDP latency ↑ DSCP=EF 64B@10000pps fixed]
  Stream 1 [UDP latency ↑ 1518B@500pps imix]
```

Only shown when per-stream overrides are active (skip when all streams use global defaults).

### Generator Integration

The `UdpGeneratorConfig` already has `packet_size`, `rate_pps`, and `pattern`. The client resolves per-stream config before creating each generator. No changes needed to the generator itself — it already accepts per-stream values.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/dscp.rs` | Modify | Add `StreamConfig` resolution, parsing helpers for `--stream-size`, `--stream-rate`, `--stream-pattern` |
| `src/protocol/mod.rs` | Modify | Add `stream_config` to `HelloMsg`, add `StreamConfigOverride` struct |
| `src/bin/client.rs` | Modify | Add 3 new CLI flags, resolve per-stream config, pass to generators |
| `src/bin/server.rs` | Modify | Read per-stream config from hello, include in report |
| `src/report/mod.rs` | Modify | Add `config` and `StreamConfigReport` to `StreamReport` |
| `src/report/text.rs` | Modify | Show per-stream config in stream header when overrides active |
| `src/report/json.rs` | Modify | Update test constructors |
| `src/report/junit.rs` | Modify | Update test constructors |

## Implementation Order

1. Parsing helpers — `parse_stream_size()`, `parse_stream_rate()`, `parse_stream_pattern()`, `resolve_stream_config()`
2. Protocol changes — `StreamConfigOverride` in `HelloMsg`
3. CLI changes — 3 new flags, resolve config per stream
4. Report changes — `StreamConfigReport` in output
5. Server changes — read from hello, include in report

## Testing

- [ ] `parse_stream_size("0=64")` returns (0, 64)
- [ ] `parse_stream_rate("1=10000")` returns (1, 10000)
- [ ] `parse_stream_pattern("0=imix")` returns (0, Imix)
- [ ] Invalid formats rejected with clear error
- [ ] Resolution: override wins over global default
- [ ] Resolution: unoverridden stream uses global default
- [ ] Per-stream config in HelloMsg serialization round-trip
- [ ] Per-stream config shown in text report when overrides active
- [ ] Per-stream config in JSON report
- [ ] No overrides = no config in report (backward compatible)
- [ ] `cargo test` passes all existing + new tests
- [ ] End-to-end: client with --stream-size 0=64 sends 64-byte packets

## Not In Scope

- Config file format (CLI flags only)
- Dynamic stream reconfiguration during a test
- Validation that stream IDs match actual streams (IDs are resolved at generator creation time)
