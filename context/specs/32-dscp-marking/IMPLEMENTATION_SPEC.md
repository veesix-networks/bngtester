# Implementation Spec: DSCP/TOS Marking on Outgoing Packets

## Overview

Add DSCP (Differentiated Services Code Point) marking to bngtester-client and bngtester-server **data stream** sockets via the `IP_TOS` socket option. Supports a global `--dscp` flag and per-stream overrides via `--stream-dscp`. Enables QoS class validation — proving the BNG classifies and treats different traffic classes correctly. Control channel sockets are not marked — this feature targets test traffic only.

## Source Issue

[#32 — DSCP/TOS marking on outgoing packets](https://github.com/veesix-networks/bngtester/issues/32)

## Current State

- `bngtester-client` and `bngtester-server` exist with UDP latency and TCP throughput support.
- All outgoing packets use the system default TOS (0 — best effort).
- No mechanism to set DSCP/TOS on sockets.
- The `UdpGeneratorConfig` and `TcpGeneratorConfig` structs don't carry a DSCP field.
- Reports don't include DSCP information per stream.
- All socket binds use IPv4 wildcard (`0.0.0.0:0`). IPv6 endpoints are not currently supported.

## Design

### DSCP Background

The TOS byte in the IP header contains 6 DSCP bits (bits 7-2) and 2 ECN bits (bits 1-0). This spec only handles DSCP; ECN is issue #33.

```
TOS byte:  [DSCP (6 bits)] [ECN (2 bits)]
IP_TOS:    DSCP << 2 | ECN
```

To set DSCP, `setsockopt(IP_TOS)` is called with `dscp_value << 2`. This sets the full TOS byte including ECN bits to 0. When issue #33 adds ECN support, the helper must be updated to preserve ECN bits via read-modify-write (`getsockopt` → set DSCP bits → `setsockopt`).

### Scope: Data Streams Only

DSCP marking applies to **data stream sockets only** (UDP latency probes, UDP throughput, TCP throughput). Control channel TCP sockets are not marked — they carry protocol messages, not test traffic. The BNG classifies traffic based on data plane packets, not control signaling.

### IPv4-Only Constraint

This feature is IPv4-only. The current codebase hardcodes IPv4 wildcard binds for data sockets. Setting DSCP on an IPv6 socket requires `IPV6_TCLASS` instead of `IP_TOS`. The helper function must check the socket address family and fail with a clear error if an IPv6 endpoint is used with `--dscp`. IPv6 DSCP support is a future enhancement.

### Standard DSCP Names

The CLI accepts both numeric values (0-63) and standard PHB names:

| Name | Value | Description |
|------|-------|-------------|
| `BE` / `CS0` | 0 | Best Effort (default) |
| `CS1`-`CS7` | 8,16,24,32,40,48,56 | Class Selector |
| `AF11`-`AF43` | 10,12,14,18,20,22,26,28,30,34,36,38 | Assured Forwarding |
| `EF` | 46 | Expedited Forwarding |

Numeric values must be 0-63. Unknown names and out-of-range values are rejected with an immediate error and process exit.

### Socket-Level Application

DSCP is set via `socket2` crate for both UDP and TCP:

**UDP:** Create `socket2::Socket` → `set_tos()` → convert to `tokio::net::UdpSocket`. The `socket2` crate provides `set_tos()` directly.

**TCP:** Create `socket2::Socket` → `set_tos()` → `connect()` → convert to `tokio::net::TcpStream`. This ensures the SYN packet carries the correct DSCP marking. Using `TcpStream::connect()` directly would send the SYN before `setsockopt`, causing BNGs that classify on SYN to miss the marking.

### Fail-Fast on setsockopt Failure

If `setsockopt(IP_TOS)` fails (e.g., no `CAP_NET_ADMIN` for certain DSCP values), the test **must not start**. Silent fallback to best-effort traffic would produce misleading results — the report would claim EF marking was used when it wasn't. The behavior is:

1. Attempt `set_tos()` on the socket immediately after creation.
2. If it fails, log the error with the DSCP value and socket details.
3. Abort the session before any data is sent.
4. Exit with a non-zero exit code.

### Per-Stream DSCP

Each stream can have its own DSCP value. The flow:

1. Client CLI: `--dscp EF` sets global default for all streams. `--stream-dscp 0=AF41 --stream-dscp 1=BE` overrides specific streams.
2. Client sends DSCP config in the `hello` message so the server knows what DSCP to apply on reverse-path streams and what to label in reports.
3. Each data socket gets its DSCP set via `set_tos()` before sending.
4. Reports include the DSCP value per stream for correlation.

### Control Protocol Changes

Add DSCP config to the `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub dscp: Option<u8>,                    // global default (DSCP value 0-63)
    pub stream_dscp: Vec<StreamDscpConfig>,  // per-stream overrides
}

pub struct StreamDscpConfig {
    pub stream_id: u8,
    pub dscp: u8,
}
```

The server applies DSCP from the hello message to its reverse-path data stream sockets (bidirectional/RRUL modes). The generator implementation is shared between client and server — both apply DSCP if configured, regardless of which side initiates the stream.

### Report Changes

Add `dscp` fields to `StreamReport` with `skip_serializing_if` to maintain backward compatibility — reports without DSCP will not change shape:

```rust
pub struct StreamReport {
    // ... existing fields ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dscp: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dscp_name: Option<String>,
}
```

Text output shows DSCP per stream:
```
  Stream 0 [UDP latency ↑ DSCP=AF41] 100pps
```

JSON output includes `dscp` and `dscp_name` fields per stream only when DSCP is configured.

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--dscp <CODEPOINT>` | _(unset, BE)_ | Global DSCP for all data streams. Accepts name (AF41, EF, CS6) or number (0-63). |
| `--stream-dscp <ID>=<CODEPOINT>` | _(unset)_ | Per-stream DSCP override. Repeatable. |

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modify | Add `socket2` dependency |
| `src/dscp.rs` | Create | DSCP name parsing, TOS byte conversion, socket2 set_tos helper with IPv4 assertion |
| `src/lib.rs` | Modify | Add `pub mod dscp;` |
| `src/protocol/mod.rs` | Modify | Add DSCP fields to `HelloMsg`, add `StreamDscpConfig` struct |
| `src/traffic/generator.rs` | Modify | Add `dscp` field to `UdpGeneratorConfig`, create socket via socket2, apply set_tos, fail-fast |
| `src/traffic/tcp.rs` | Modify | Add `dscp` field to `TcpGeneratorConfig`, create socket via socket2, set TOS before connect |
| `src/bin/client.rs` | Modify | Add `--dscp` and `--stream-dscp` CLI flags, resolve per-stream DSCP, pass to configs |
| `src/bin/server.rs` | Modify | Read DSCP from hello message, apply to reverse-path streams, include in report |
| `src/report/mod.rs` | Modify | Add `dscp` and `dscp_name` fields to `StreamReport` with skip_serializing_if |
| `src/report/text.rs` | Modify | Show DSCP in stream header line |
| `src/report/junit.rs` | Modify | Update test constructors for new StreamReport fields |
| `src/report/json.rs` | Modify | Update test constructors for new StreamReport fields |

## Implementation Order

1. `Cargo.toml` — add `socket2` dependency
2. `src/dscp.rs` — DSCP name parser, TOS conversion, socket2 set_tos helper with fail-fast and IPv4 assertion
3. Protocol changes — add DSCP to `HelloMsg` and `StreamDscpConfig`
4. Generator changes — create sockets via socket2, apply DSCP, fail-fast on error
5. CLI changes — add `--dscp` and `--stream-dscp` flags to client
6. Report changes — add DSCP to `StreamReport` (skip_serializing_if), update text/JSON/JUnit output and tests
7. Server changes — read DSCP from hello, apply to reverse-path streams, include in report

## Testing

- [ ] DSCP name parsing: BE, CS0-CS7, AF11-AF43, EF, numeric 0-63
- [ ] Invalid DSCP values rejected (>63, unknown names)
- [ ] TOS byte calculation: DSCP 46 (EF) → TOS 184 (46 << 2)
- [ ] `set_tos()` applied to UDP socket via socket2
- [ ] TCP socket created via socket2 with TOS set before connect
- [ ] setsockopt failure causes immediate abort (fail-fast), not silent fallback
- [ ] IPv6 endpoint with --dscp produces a clear error
- [ ] Per-stream DSCP override works (stream 0 = AF41, stream 1 = BE)
- [ ] DSCP included in JSON report per stream (absent when not configured)
- [ ] DSCP shown in text report stream header
- [ ] Global --dscp applies to all streams without overrides
- [ ] No DSCP flag = default behavior (no TOS change, no DSCP in report)
- [ ] JSON output without DSCP is identical to pre-change output (backward compatible)
- [ ] `cargo test` passes all existing + new tests
- [ ] End-to-end: client with --dscp EF sends packets, tcpdump confirms TOS byte

## Not In Scope

- ECN marking (issue #33) — when implemented, the set_tos helper must be updated to preserve ECN bits
- Verifying received DSCP via IP_RECVTOS (separate follow-up)
- Verifying BNG QoS policy application (test/assertion concern)
- Raw socket based marking
- IPv6 traffic class (`IPV6_TCLASS`) — future enhancement, currently rejected with error
- DSCP on control channel sockets — only data streams are marked
