# Implementation Spec: CGNAT-Aware Reporting

## Overview

Make reports CGNAT-aware by showing both the translated address (what the server sees after CGNAT) and the subscriber's real address (from the client's `--source-ip` or hello message). Ensures `client_id` is the primary identifier for multi-subscriber scenarios where CGNAT makes IPs unreliable.

## Source Issue

[#50 — CGNAT-aware reporting (translated vs real subscriber addresses)](https://github.com/veesix-networks/bngtester/issues/50)

## Current State

- `HelloMsg` already carries `source_ip: Option<String>` (set when `--source-ip` is used).
- `ClientReport` has `peer: String` (TCP socket address the server sees — CGNAT translated if CGNAT in path).
- `ClientReport` has `client_id: String`.
- `StreamReport` has `source_ip: Option<String>` (from bind-interface).

## Design

### Dual Addressing

- `peer` — the address the server's TCP socket sees (CGNAT translated address, includes port)
- `subscriber_ip` — the client's self-reported real address from `HelloMsg.source_ip` (set via `--source-ip`)

**No fallback to control socket local address.** The control channel may route via a management IP, not the data path subscriber IP. If `--source-ip` is not set, `subscriber_ip` is omitted — not populated with a potentially wrong address. Users behind CGNAT should always set `--source-ip` (or use `--config` with `source_ip` field).

### Protocol

No new fields needed — `HelloMsg.source_ip` already exists. The client only sends it when `--source-ip` is explicitly set (no change to current behavior).

### Report Changes

**TestConfig** — add `subscriber_ip` with `skip_serializing_if`:
```rust
pub struct TestConfig {
    pub mode: TestMode,
    pub duration_secs: u32,
    pub client: String,           // peer address (CGNAT translated)
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriber_ip: Option<String>,  // client's real address (from --source-ip)
}
```

**ClientReport** (combined mode) — add `subscriber_ip` with `skip_serializing_if`:
```rust
pub struct ClientReport {
    pub client_id: String,
    pub peer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriber_ip: Option<String>,
    pub report: TestReport,
}
```

This is an **additive JSON schema change**. Existing consumers that ignore unknown fields are unaffected. Strict schema validators need to tolerate the new optional field. When `subscriber_ip` is absent (no `--source-ip` set), the field is omitted entirely from JSON output.

### Text Output

**Combined report headers** — show dual addressing when CGNAT detected (IPs differ):
```
--- subscriber-1 (peer: 198.51.100.5:43210, subscriber: 10.255.0.2) ---
```

**Simplified** when peer IP matches subscriber IP (no CGNAT):
```
--- subscriber-1 (10.255.0.2:43210) ---
```

**No subscriber_ip** (--source-ip not set):
```
--- subscriber-1 (198.51.100.5:43210) ---
```

**Comparison logic:** Parse `peer` as `SocketAddr`, compare `peer.ip()` to `subscriber_ip` parsed as `IpAddr`. String comparison would fail because peer includes port. On parse failure, fall back to dual display.

### Single-client report

Same logic in `TestConfig.client` vs `TestConfig.subscriber_ip`:
```
Client: 198.51.100.5:43210 (subscriber: 10.255.0.2)
```

Or simplified when IPs match or subscriber_ip absent.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/report/mod.rs` | Modify | Add `subscriber_ip` to `TestConfig` and `ClientReport` with skip_serializing_if |
| `src/bin/server.rs` | Modify | Read source_ip from hello, populate subscriber_ip in TestConfig and ClientReport |
| `src/report/text.rs` | Modify | Dual/simplified addressing in combined headers and single-client output |
| `src/report/json.rs` | Modify | Update test constructors |
| `src/report/junit.rs` | Modify | Update test constructors |

## Implementation Order

1. Report structs — add subscriber_ip fields
2. Server — read source_ip from hello, populate in reports
3. Text formatter — dual/simplified addressing logic
4. Test constructors — update with new fields

## Testing

- [ ] subscriber_ip populated from hello source_ip when --source-ip set
- [ ] subscriber_ip omitted when --source-ip not set
- [ ] Combined text: dual addressing when peer IP != subscriber IP
- [ ] Combined text: simplified when peer IP == subscriber IP
- [ ] Combined text: no subscriber_ip shows peer only
- [ ] JSON: subscriber_ip present with skip_serializing_if (omitted when None)
- [ ] IP comparison is IP-only (strips port from peer)
- [ ] `cargo test` passes all existing + new tests

## Not In Scope

- CGNAT detection (whether CGNAT is actually in the path)
- NAT traversal for reverse-path streams
- STUN/TURN for NAT discovery
- Fallback to control socket local address (explicitly rejected — may be wrong IP)
