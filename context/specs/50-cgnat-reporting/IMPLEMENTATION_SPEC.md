# Implementation Spec: CGNAT-Aware Reporting

## Overview

Make reports CGNAT-aware by showing both the translated address (what the server sees after CGNAT) and the subscriber's real address (from the client's hello message). Ensures `client_id` is the primary identifier for multi-subscriber scenarios where CGNAT makes IPs unreliable.

## Source Issue

[#50 — CGNAT-aware reporting (translated vs real subscriber addresses)](https://github.com/veesix-networks/bngtester/issues/50)

## Current State

- `HelloMsg` already carries `source_ip: Option<String>` (the client's real IP when `--source-ip` is set).
- `ClientReport` has `peer: String` (the TCP socket address the server sees — this is the CGNAT translated address).
- `ClientReport` has `client_id: String` (explicit identity).
- `StreamReport` has `source_ip: Option<String>` (from bind-interface).
- The server's per-session `TestReport` uses `peer.to_string()` for the `client` field.
- **Gap:** No `subscriber_ip` field showing the client's real address alongside the translated `peer`. When CGNAT is active, `peer` is the public translated IP:port, not the subscriber's access-side address.

## Design

### Dual Addressing

Add `subscriber_ip` to report output alongside existing `peer`:

- `peer` — the address the server's TCP socket sees (CGNAT translated if CGNAT is in path, or real if direct)
- `subscriber_ip` — the client's self-reported real address from `HelloMsg.source_ip`, or the client's local address if not explicitly set

The client always sends its real address in the hello message. Currently `source_ip` is only sent when `--source-ip` is set. Change: always send the client's local IP (from the control channel socket's local address) as a fallback.

### Protocol Change

No new fields needed — `HelloMsg.source_ip` already exists. Just ensure the client always populates it:

```rust
// Current: only set when --source-ip is used
pub source_ip: Option<String>,

// New: always set — from --source-ip if provided, else from control socket local addr
```

### Report Changes

**TestReport** — add `subscriber_ip`:
```rust
pub struct TestConfig {
    // existing: mode, duration_secs, client (peer addr), server
    pub subscriber_ip: Option<String>,  // client's real address
}
```

**ClientReport** (combined mode) — add `subscriber_ip`:
```rust
pub struct ClientReport {
    pub client_id: String,
    pub peer: String,              // CGNAT translated address
    pub subscriber_ip: Option<String>,  // client's real address
    pub report: TestReport,
}
```

**Text output:**
```
--- subscriber-1 (peer: 198.51.100.5:43210, subscriber: 10.255.0.2) ---
```

Without CGNAT (peer == subscriber), show simplified:
```
--- subscriber-1 (10.255.0.2:43210) ---
```

### Client-ID as Primary Identifier

Already implemented — `--client-id` is used for multi-subscriber grouping. This spec just reinforces that `client_id` takes precedence and documents that CGNAT users should always set `--client-id` since multiple subscribers may share the same CGNAT public IP.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/bin/client.rs` | Modify | Always send local IP in hello source_ip (fallback from control socket) |
| `src/bin/server.rs` | Modify | Read source_ip from hello, add subscriber_ip to TestReport and ClientReport |
| `src/report/mod.rs` | Modify | Add `subscriber_ip` to TestConfig and ClientReport |
| `src/report/text.rs` | Modify | Show subscriber_ip in combined report headers |
| `src/report/json.rs` | Modify | Update test constructors |
| `src/report/junit.rs` | Modify | Update test constructors |

## Implementation Order

1. Client — always populate source_ip in hello
2. Report structs — add subscriber_ip fields
3. Server — read source_ip from hello, include in reports
4. Text formatter — show dual addressing in combined headers

## Testing

- [ ] Client always sends source_ip in hello (even without --source-ip flag)
- [ ] Server report includes subscriber_ip from hello
- [ ] Combined report shows both peer and subscriber_ip per client
- [ ] Text output shows dual addressing when peer != subscriber_ip
- [ ] Text output shows simplified when peer == subscriber_ip (no CGNAT)
- [ ] JSON report includes subscriber_ip field
- [ ] No subscriber_ip = field omitted (backward compatible)
- [ ] `cargo test` passes all existing + new tests

## Not In Scope

- CGNAT detection (whether CGNAT is actually in the path)
- NAT traversal for reverse-path streams
- STUN/TURN for NAT discovery
