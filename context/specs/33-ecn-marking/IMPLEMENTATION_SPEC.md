# Implementation Spec: ECN Marking and Detection on Test Traffic

## Overview

Add ECN (Explicit Congestion Notification) support to bngtester. The client sets ECN-capable transport bits (ECT(0) or ECT(1)) on outgoing UDP packets. The server detects ECN state on received packets via `IP_RECVTOS` and `recvmsg` ancillary data, tracking all four ECN codepoints (Not-ECT, ECT(0), ECT(1), CE). This validates that the BNG's AQM (CoDel, FQ-CoDel, PIE) signals congestion via CE marking rather than dropping packets, and detects BNG misconfiguration that strips ECN bits.

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

The sender marks packets as ECN-capable (ECT(0) or ECT(1)). Routers/BNGs with AQM can set the CE bits instead of dropping the packet when queues build up. The receiver detects CE marks and reports them. If ECT packets arrive as Not-ECT, the BNG is stripping ECN — a misconfiguration.

### Sender-Side: Setting ECN Bits

The existing `dscp_to_tos()` is replaced by `build_tos()` which combines DSCP and ECN:

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

All callers (UDP/TCP generators) are updated to pass both DSCP and ECN to `build_tos()`. The socket's `IP_TOS` is set once with the combined byte.

### Receiver-Side: Detecting ECN State

To read the TOS byte of received packets, the receiver must:

1. Enable `IP_RECVTOS` on the UDP socket via `setsockopt`. **Fail-fast:** If `IP_RECVTOS` cannot be enabled and ECN observation was requested, fail the receiver setup before the test starts.
2. Use tokio-safe `recvmsg` via `UdpSocket::readable().await` + `try_io()` wrapping raw `libc::recvmsg`. This preserves async cancellation — the receiver can still be cancelled via `CancellationToken` while waiting for readiness. **Never call blocking `libc::recvmsg` directly** inside a tokio task without readiness gating.
3. Extract the `IP_TOS` value from the `IPPROTO_IP / IP_TOS` control message. The cmsg data is a `libc::c_int` (not a single byte) — cast to `u8` after extraction.
4. Classify the ECN bits (bottom 2 bits of TOS):
   - `00` = Not-ECT
   - `01` = ECT(1)
   - `10` = ECT(0)
   - `11` = CE

**Missing cmsg handling:** If `IP_RECVTOS` was enabled but a packet arrives without a `IP_TOS` cmsg, that packet's ECN state is counted as `ecn_unknown`. The `ecn_ce_ratio` is only computed from packets with known ECN state. If all packets lack cmsg, the ECN metrics are omitted from the report (not reported as zero).

### ECN Metrics

New metrics tracked per stream, covering all four ECN codepoints:

| Metric | Description |
|--------|-------------|
| `ecn_ect_sent` | Packets sent with ECT(0) or ECT(1) — equals packets_sent when ECN is enabled |
| `ecn_not_ect_received` | Received packets with ECN=00 (BNG stripped ECN if sender set ECT) |
| `ecn_ect0_received` | Received packets with ECN=10 (ECT(0) preserved) |
| `ecn_ect1_received` | Received packets with ECN=01 (ECT(1) preserved) |
| `ecn_ce_received` | Received packets with ECN=11 (congestion experienced) |
| `ecn_ce_ratio` | `ecn_ce_received / (total observed) * 100` |

Tracking all four states lets the test runner detect:
- **CE marks** — AQM is signaling congestion (expected under load)
- **ECN stripping** — BNG re-marked ECT to Not-ECT (misconfiguration)
- **ECN preservation** — ECT bits passed through unchanged (correct behavior when not congested)

### Control Protocol Changes

Add ECN config to `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub ecn: Option<String>,  // "ect0", "ect1", or null (off)
}
```

Add ECN observation fields to `StreamResult`:

```rust
pub struct StreamResult {
    // ... existing fields ...
    pub ecn_not_ect: Option<u64>,
    pub ecn_ect0: Option<u64>,
    pub ecn_ect1: Option<u64>,
    pub ecn_ce: Option<u64>,
}
```

### Report Changes

Add ECN fields to `StreamReport` and `StreamResults` with `skip_serializing_if`. All ECN report fields are **omitted** (not zero) when ECN is disabled or observation is unavailable. Zero values mean "observed zero of that ECN codepoint", not "ECN not observed".

```rust
pub struct StreamReport {
    // ... existing fields ...
    pub ecn_mode: Option<String>,  // "ect0", "ect1", or omitted
}

pub struct StreamResults {
    // ... existing fields ...
    pub ecn_ect_sent: Option<u64>,
    pub ecn_not_ect_received: Option<u64>,
    pub ecn_ect0_received: Option<u64>,
    pub ecn_ect1_received: Option<u64>,
    pub ecn_ce_received: Option<u64>,
    pub ecn_ce_ratio: Option<f64>,
}
```

Text output:
```
  Stream 0 [UDP latency ↑ DSCP=EF ECN=ECT0] 100pps
    ...
    ECN:      CE=5 (0.5%) ECT0=990 ECT1=0 Not-ECT=5
```

### CLI Flag

Single `--ecn <MODE>` flag replacing the two mutually exclusive flags:

| Flag | Default | Description |
|------|---------|-------------|
| `--ecn <MODE>` | _(off)_ | Enable ECN. MODE: `ect0` or `ect1` |

Combined with `--dscp` if both set: `--dscp EF --ecn ect0` → TOS = 0xBA.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/dscp.rs` | Modify | Add `EcnMode` enum, `build_tos()` replacing `dscp_to_tos()`, `recvmsg_tos()` wrapper with tokio readiness integration, `enable_recv_tos()` helper |
| `src/protocol/mod.rs` | Modify | Add `ecn` field to `HelloMsg`, add ECN observation fields to `StreamResult` |
| `src/traffic/generator.rs` | Modify | Pass combined TOS (DSCP+ECN) via `build_tos()` |
| `src/traffic/tcp.rs` | Modify | Pass combined TOS (DSCP+ECN) via `build_tos()` |
| `src/traffic/receiver.rs` | Modify | Enable `IP_RECVTOS`, switch to `recvmsg` via tokio `readable()` + `try_io()`, track all 4 ECN states, add ECN counters to `UdpReceiverResult` |
| `src/bin/client.rs` | Modify | Add `--ecn <MODE>` CLI flag, combine with DSCP via `build_tos()` |
| `src/bin/server.rs` | Modify | Enable `IP_RECVTOS` on receiver socket, switch to `recvmsg`, track ECN states, include in report |
| `src/report/mod.rs` | Modify | Add ECN fields to `StreamReport` and `StreamResults` with `skip_serializing_if` |
| `src/report/text.rs` | Modify | Show ECN mode in stream header, ECN breakdown in metrics |
| `src/report/json.rs` | Modify | Update test constructors |
| `src/report/junit.rs` | Modify | Update test constructors |

## Implementation Order

1. `src/dscp.rs` — `EcnMode` enum, `build_tos()`, `enable_recv_tos()`, `recvmsg_tos()` with tokio-safe readiness wrapping, cmsg parsing as `c_int`
2. Protocol changes — ECN in `HelloMsg` and `StreamResult`
3. Generator changes — pass combined TOS byte via `build_tos()`
4. Receiver changes — both `src/traffic/receiver.rs` and `src/bin/server.rs`: enable `IP_RECVTOS`, switch to `recvmsg`, track all 4 ECN states, handle missing cmsg
5. CLI changes — `--ecn <MODE>` flag
6. Report changes — ECN metrics in StreamReport/StreamResults and text/JSON output, omitted when ECN disabled

## Testing

- [ ] `build_tos()` combines DSCP and ECN correctly (DSCP=46 + ECT0 = 0xBA)
- [ ] `EcnMode` parsing: "ect0", "ect1", invalid strings rejected
- [ ] `--ecn ect0` sets ECT(0) bits on outgoing packets
- [ ] `--ecn ect1` sets ECT(1) bits on outgoing packets
- [ ] `--dscp EF --ecn ect0` combines correctly (TOS = 0xBA)
- [ ] `IP_RECVTOS` enabled on receiver socket — fail-fast if unsupported
- [ ] `recvmsg` extracts TOS byte from cmsg (parsed as `c_int`)
- [ ] All 4 ECN states tracked: Not-ECT, ECT(0), ECT(1), CE
- [ ] Missing cmsg → counted as unknown, CE ratio excludes unknowns
- [ ] ECN metrics in JSON report (all fields, omitted when ECN off)
- [ ] ECN mode shown in text report stream header
- [ ] ECN breakdown shown in text report metrics
- [ ] No ECN flag = default behavior (no ECN in report, fields omitted)
- [ ] JSON output without ECN unchanged (backward compatible)
- [ ] `src/traffic/receiver.rs` updated with ECN support (not left stale)
- [ ] Negative test: `IP_RECVTOS` failure produces clear error
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] End-to-end: tcpdump confirms ECN bits on wire through BNG

## Not In Scope

- ECN negotiation for TCP (kernel handles TCP ECN via `net.ipv4.tcp_ecn` sysctl)
- L2 ECN transparency
- Verifying BNG AQM policy (test/assertion concern — the tool reports CE marks, the test runner validates)
- IPv6 ECN (same constraint as DSCP — IPv4-only for now)
- Received DSCP verification (separate follow-up — same recvmsg path but different scope)
