# Implementation Spec: DSCP/TOS Marking on Outgoing Packets

## Overview

Add DSCP (Differentiated Services Code Point) marking to bngtester-client and bngtester-server outgoing packets via the `IP_TOS` socket option. Supports a global `--dscp` flag and per-stream overrides via `--stream-dscp`. Enables QoS class validation — proving the BNG classifies and treats different traffic classes correctly.

## Source Issue

[#32 — DSCP/TOS marking on outgoing packets](https://github.com/veesix-networks/bngtester/issues/32)

## Current State

- `bngtester-client` and `bngtester-server` exist with UDP latency and TCP throughput support.
- All outgoing packets use the system default TOS (0 — best effort).
- No mechanism to set DSCP/TOS on sockets.
- The `UdpGeneratorConfig` and `TcpGeneratorConfig` structs don't carry a DSCP field.
- Reports don't include DSCP information per stream.

## Design

### DSCP Background

The TOS byte in the IP header contains 6 DSCP bits (bits 7-2) and 2 ECN bits (bits 1-0). This spec only handles DSCP; ECN is issue #33.

```
TOS byte:  [DSCP (6 bits)] [ECN (2 bits)]
IP_TOS:    DSCP << 2 | ECN
```

To set DSCP without disturbing ECN bits, `setsockopt(IP_TOS)` is called with `dscp_value << 2`. On Linux, this sets the full TOS byte — ECN bits default to 0 unless explicitly set.

### Standard DSCP Names

The CLI accepts both numeric values (0-63) and standard PHB names:

| Name | Value | Description |
|------|-------|-------------|
| `BE` / `CS0` | 0 | Best Effort (default) |
| `CS1`-`CS7` | 8,16,24,32,40,48,56 | Class Selector |
| `AF11`-`AF43` | 10,12,14,18,20,22,26,28,30,34,36,38 | Assured Forwarding |
| `EF` | 46 | Expedited Forwarding |

### Socket-Level Application

DSCP is set on sockets before data transmission begins:

```rust
// For UDP
let tos = dscp_value << 2;
socket.set_tos(tos)?;

// For TCP (same mechanism)
let tos = dscp_value << 2;
stream.set_tos(tos)?;
```

On Linux, `IP_TOS` is set via `setsockopt` on the raw fd. Tokio's `UdpSocket` and `TcpStream` don't expose `set_tos` directly — we use `socket2` or raw `setsockopt` via `libc`.

### Per-Stream DSCP

Each stream can have its own DSCP value. The flow:

1. Client CLI: `--dscp EF` sets global default for all streams. `--stream-dscp 0=AF41 --stream-dscp 1=BE` overrides specific streams.
2. Client sends DSCP config in the `hello` message so the server knows what to expect (for report labeling).
3. Each UDP/TCP socket gets its DSCP set via `setsockopt` before sending.
4. Server reports include the DSCP value per stream for correlation.

### Control Protocol Changes

Add DSCP config to the `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub dscp: Option<u8>,                    // global default
    pub stream_dscp: Vec<StreamDscpConfig>,  // per-stream overrides
}

pub struct StreamDscpConfig {
    pub stream_id: u8,
    pub dscp: u8,
}
```

The server doesn't need to set DSCP on its sockets (it receives, doesn't send data streams in latency mode). For bidirectional/RRUL reverse-path streams where the server sends, the server applies the DSCP from the hello message.

### Report Changes

Add `dscp` field to `StreamReport`:

```rust
pub struct StreamReport {
    // ... existing fields ...
    pub dscp: Option<u8>,       // DSCP value used (numeric)
    pub dscp_name: Option<String>, // Human-readable name (e.g., "AF41")
}
```

Text output shows DSCP per stream:
```
  Stream 0 [UDP latency ↑ DSCP=AF41] 100pps
```

JSON output includes `dscp` and `dscp_name` fields per stream.

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--dscp <CODEPOINT>` | _(unset, BE)_ | Global DSCP for all streams. Accepts name (AF41, EF, CS6) or number (0-63). |
| `--stream-dscp <ID>=<CODEPOINT>` | _(unset)_ | Per-stream DSCP override. Repeatable. |

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/dscp.rs` | Create | DSCP name parsing, TOS byte conversion, setsockopt helper |
| `src/lib.rs` | Modify | Add `pub mod dscp;` |
| `src/protocol/mod.rs` | Modify | Add DSCP fields to `HelloMsg`, add `StreamDscpConfig` struct |
| `src/traffic/generator.rs` | Modify | Add `dscp` field to `UdpGeneratorConfig`, apply via setsockopt |
| `src/traffic/tcp.rs` | Modify | Add `dscp` field to `TcpGeneratorConfig`, apply via setsockopt |
| `src/bin/client.rs` | Modify | Add `--dscp` and `--stream-dscp` CLI flags, pass to configs |
| `src/bin/server.rs` | Modify | Read DSCP from hello message, include in report |
| `src/report/mod.rs` | Modify | Add `dscp` and `dscp_name` fields to `StreamReport` |
| `src/report/text.rs` | Modify | Show DSCP in stream header line |
| `src/report/json.rs` | Modify | Include DSCP fields (automatic via serde) |

## Implementation Order

1. `src/dscp.rs` — DSCP name parser, TOS conversion, setsockopt helper
2. Protocol changes — add DSCP to `HelloMsg` and `StreamDscpConfig`
3. Generator changes — apply DSCP via setsockopt on UDP and TCP sockets
4. CLI changes — add `--dscp` and `--stream-dscp` flags to client
5. Report changes — add DSCP to `StreamReport` and text/JSON output
6. Server changes — read DSCP from hello, include in report

## Testing

- [ ] DSCP name parsing: BE, CS0-CS7, AF11-AF43, EF, numeric 0-63
- [ ] Invalid DSCP values rejected (>63, unknown names)
- [ ] TOS byte calculation: DSCP 46 (EF) → TOS 184 (46 << 2)
- [ ] `setsockopt(IP_TOS)` applied to UDP socket
- [ ] `setsockopt(IP_TOS)` applied to TCP socket
- [ ] Per-stream DSCP override works (stream 0 = AF41, stream 1 = BE)
- [ ] DSCP included in JSON report per stream
- [ ] DSCP shown in text report stream header
- [ ] Global --dscp applies to all streams without overrides
- [ ] No DSCP flag = default behavior (TOS 0)
- [ ] `cargo test` passes all existing + new tests
- [ ] End-to-end: client with --dscp EF sends packets, tcpdump confirms TOS byte

## Not In Scope

- ECN marking (issue #33)
- Verifying BNG QoS policy application (test/assertion concern)
- Raw socket based marking
- IPv6 traffic class (DSCP in IPv6 uses `IPV6_TCLASS` — future enhancement)
