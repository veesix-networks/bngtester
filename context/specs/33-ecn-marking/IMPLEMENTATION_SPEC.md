# Implementation Spec: ECN Marking and Detection on Test Traffic

## Overview

Add ECN (Explicit Congestion Notification) support to bngtester. The client sets ECN-capable transport bits (ECT(0) or ECT(1)) on outgoing UDP packets. The server detects ECN-CE (Congestion Experienced) marks on received packets via `IP_RECVTOS` and `recvmsg` ancillary data. ECN metrics are reported alongside existing latency/jitter/loss metrics. This validates that the BNG's AQM (CoDel, FQ-CoDel, PIE) signals congestion via CE marking rather than dropping packets.

## Source Issue

[#33 — ECN marking and detection on test traffic](https://github.com/veesix-networks/bngtester/issues/33)

## Current State

- DSCP marking exists via `--dscp` flag (#32). The `dscp_to_tos()` function sets the full TOS byte, zeroing ECN bits.
- The `dscp.rs` module has `apply_tos_to_fd()` which calls `setsockopt(IP_TOS)`.
- The UDP receiver in `src/traffic/receiver.rs` and the inline receiver in `src/bin/server.rs` use `recv_from()` which does not return ancillary data.
- No ECN-related metrics or report fields exist.

## Design

### ECN in the TOS Byte

The TOS byte's bottom 2 bits are ECN:

```
TOS byte: [DSCP (6 bits)] [ECN (2 bits)]
          bits 7-2          bits 1-0

ECN field values:
  00 = Not-ECT (not ECN-capable)
  01 = ECT(1)
  10 = ECT(0)
  11 = CE (Congestion Experienced)
```

The sender marks packets as ECN-capable (ECT(0) or ECT(1)). Routers/BNGs with AQM can set the CE bits instead of dropping the packet when queues build up. The receiver detects CE marks and reports them.

### Sender-Side: Setting ECN Bits

The existing `dscp_to_tos()` must be updated to combine DSCP and ECN:

```rust
pub fn build_tos(dscp: Option<u8>, ecn: EcnMode) -> u8 {
    let dscp_bits = dscp.unwrap_or(0) << 2;
    let ecn_bits = match ecn {
        EcnMode::Off => 0,
        EcnMode::Ect0 => 0b10,
        EcnMode::Ect1 => 0b01,
    };
    dscp_bits | ecn_bits
}
```

This replaces the current `dscp_to_tos()` which only handled DSCP. The socket's `IP_TOS` is set once with both DSCP and ECN bits combined.

### Receiver-Side: Detecting CE Marks

To read the TOS byte of received packets, the receiver must:

1. Enable `IP_RECVTOS` on the UDP socket via `setsockopt`.
2. Use `recvmsg` instead of `recv_from` to get ancillary data (cmsg).
3. Extract the `IP_TOS` value from the `IP_TOS` cmsg.
4. Check if ECN bits are `11` (CE).

On Linux, `recvmsg` returns the TOS byte in a `IPPROTO_IP / IP_TOS` control message. This requires using raw `libc::recvmsg` since tokio's `UdpSocket::recv_from` doesn't expose ancillary data.

**Implementation approach:** Use `socket2::Socket` for the receiver socket with `set_recv_tos(true)` to enable `IP_RECVTOS`. Then use tokio's `AsyncFd` wrapper to do async `recvmsg` calls that read the cmsg data.

Alternatively, since the server receiver loop already uses a pre-bound `UdpSocket`, we can call `libc::recvmsg` on the raw fd when the socket is readable. This is simpler than switching to `AsyncFd` — we just replace the `recv_from` call with a `recvmsg` wrapper.

### ECN Metrics

New metrics tracked per stream:

| Metric | Description |
|--------|-------------|
| `ecn_ect_sent` | Packets sent with ECT(0) or ECT(1) — always equals packets_sent when ECN is enabled |
| `ecn_ce_received` | Packets received with CE mark (ECN bits = 11) |
| `ecn_ce_ratio` | `ecn_ce_received / packets_received * 100` — percentage of packets marked congested |

### Control Protocol Changes

Add ECN config to `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub ecn: Option<String>,  // "ect0", "ect1", or null
}
```

### Report Changes

Add ECN fields to `StreamResults` with `skip_serializing_if`:

```rust
pub struct StreamResults {
    // ... existing fields ...
    pub ecn_ect_sent: Option<u64>,
    pub ecn_ce_received: Option<u64>,
    pub ecn_ce_ratio: Option<f64>,
}
```

Text output:
```
  Stream 0 [UDP latency ↑ DSCP=EF ECN=ECT0] 100pps
    ...
    ECN CE:   0.5% (5/1000)
```

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--ecn` | _(off)_ | Enable ECN with ECT(0) on outgoing packets |
| `--ecn-ect1` | _(off)_ | Enable ECN with ECT(1) instead of ECT(0) |

`--ecn` and `--ecn-ect1` are mutually exclusive. ECN is combined with `--dscp` if both are set.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/dscp.rs` | Modify | Add `EcnMode` enum, `build_tos()` replacing `dscp_to_tos()`, `recvmsg` wrapper to extract TOS from cmsg |
| `src/protocol/mod.rs` | Modify | Add `ecn` field to `HelloMsg`, add ECN fields to `StreamResult` |
| `src/traffic/generator.rs` | Modify | Pass combined TOS (DSCP+ECN) to socket |
| `src/traffic/tcp.rs` | Modify | Pass combined TOS (DSCP+ECN) to socket |
| `src/bin/client.rs` | Modify | Add `--ecn` and `--ecn-ect1` CLI flags, combine with DSCP |
| `src/bin/server.rs` | Modify | Enable `IP_RECVTOS`, use recvmsg, count CE marks, include in report |
| `src/report/mod.rs` | Modify | Add `ecn_ect_sent`, `ecn_ce_received`, `ecn_ce_ratio` to `StreamResults`, ECN mode to `StreamReport` |
| `src/report/text.rs` | Modify | Show ECN mode in stream header, CE ratio in metrics |
| `src/report/json.rs` | Modify | Update test constructors |
| `src/report/junit.rs` | Modify | Update test constructors |

## Implementation Order

1. `src/dscp.rs` — `EcnMode` enum, `build_tos()`, `recvmsg_tos()` wrapper
2. Protocol changes — ECN in `HelloMsg` and `StreamResult`
3. Generator changes — pass combined TOS byte
4. Receiver changes — enable `IP_RECVTOS`, use recvmsg, count CE marks
5. CLI changes — `--ecn` and `--ecn-ect1` flags
6. Report changes — ECN metrics in StreamResults and text/JSON output

## Testing

- [ ] `build_tos()` combines DSCP and ECN correctly (DSCP=46 + ECT0 = 0xBA)
- [ ] `--ecn` sets ECT(0) bits on outgoing packets
- [ ] `--ecn-ect1` sets ECT(1) bits on outgoing packets
- [ ] `--ecn` and `--ecn-ect1` are mutually exclusive
- [ ] `--dscp EF --ecn` combines correctly (TOS = 0xBA)
- [ ] `IP_RECVTOS` enabled on receiver socket
- [ ] `recvmsg` extracts TOS byte from cmsg
- [ ] CE mark detection: ECN bits = 11 counted correctly
- [ ] ECN metrics in JSON report (ecn_ect_sent, ecn_ce_received, ecn_ce_ratio)
- [ ] ECN mode shown in text report stream header
- [ ] CE ratio shown in text report metrics
- [ ] No ECN flag = default behavior (ECN bits = 00, no ECN in report)
- [ ] JSON output without ECN unchanged (backward compatible)
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] End-to-end: tcpdump confirms ECN bits on wire through BNG

## Not In Scope

- ECN negotiation for TCP (kernel handles TCP ECN via `net.ipv4.tcp_ecn` sysctl)
- L2 ECN transparency
- Verifying BNG AQM policy (test/assertion concern — the tool reports CE marks, the test runner validates)
- IPv6 ECN (same constraint as DSCP — IPv4-only for now)
