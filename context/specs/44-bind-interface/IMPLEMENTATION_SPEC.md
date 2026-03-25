# Implementation Spec: Bind Interface / Source IP for Bare Metal and Loopback Testing

## Overview

Add `--bind-iface` and `--source-ip` flags to bngtester-client, and `--data-bind-iface` to bngtester-server, so data sockets can be bound to specific network interfaces or source IPs. This enables bare metal BNG testing and loopback/hairpin testing on a single host.

## Source Issue

[#44 — Bind interface / source IP for bare metal and loopback testing](https://github.com/veesix-networks/bngtester/issues/44)

## Current State

- UDP data sockets bind to `0.0.0.0:0` via tokio directly (default path) or via socket2 (when TOS is set).
- TCP data sockets use `TcpStream::connect()` (default path) or socket2 (when TOS is set).
- Control channel uses `TcpStream::connect()` with no bind.
- **The default no-TOS paths bypass socket2 entirely** — bind logic must cover these paths too.

## Design

### CLI Flags

| Flag | Where | Default | Description |
|------|-------|---------|-------------|
| `--bind-iface <NAME>` | Client | _(none)_ | Bind data sockets to a specific interface via `SO_BINDTODEVICE` |
| `--source-ip <ADDR>` | Client | _(any)_ | Bind data sockets to a specific source IP |
| `--control-bind-ip <ADDR>` | Client | _(any)_ | Bind control channel TCP to a specific source IP |
| `--data-bind-iface <NAME>` | Server | _(none)_ | Bind receiver UDP socket to a specific interface |

### Socket Construction Strategy

**Always route through socket2 when any pre-connect option is requested** (tos, source_ip, bind_iface). This replaces the current split where socket2 is only used for TOS paths:

```rust
// Before: two branches (socket2 for TOS, tokio direct for default)
// After: socket2 whenever tos OR source_ip OR bind_iface is set
let needs_socket2 = config.tos.is_some()
    || config.source_ip.is_some()
    || config.bind_iface.is_some();
```

When no pre-connect options are set, the existing tokio-direct path is preserved (no unnecessary overhead).

### Socket Setup Ordering

Safer ordering with privilege-sensitive operations first:

```
new() → SO_BINDTODEVICE → bind(source_ip:0) → set_tos() → connect()
```

This ensures `SO_BINDTODEVICE` (which may require privileges) fails before TOS is set. Any setup failure discards the socket — no partially configured socket is reused.

### SO_BINDTODEVICE via socket2

Use `socket2::Socket::bind_device(Some(iface.as_bytes()))` instead of raw `libc::setsockopt`. This is idiomatic, handles null termination, and is already available since the project depends on `socket2` with `all` features.

**Permissions note:** Since Linux 5.7, `SO_BINDTODEVICE` no longer requires `CAP_NET_RAW` for unprivileged processes. Older kernels require root or `CAP_NET_RAW`.

### Source IP Validation

No user-space precheck — rely on `bind()` failure from the kernel. The kernel's `EADDRNOTAVAIL` is the authoritative check and already fails fast. A user-space precheck would reject valid advanced setups (non-local bind) and doesn't add value when `--bind-iface` is combined.

### Fail-Fast

Any pre-connect socket setup failure (SO_BINDTODEVICE, bind, set_tos) aborts the test startup and discards the socket. Same pattern as DSCP/ECN fail-fast. The error message includes the specific operation that failed, the interface/IP, and the OS error.

### Server-Side Bind

Add optional `--data-bind-iface` to the server. When set, the UDP receiver socket calls `SO_BINDTODEVICE` to constrain traffic to a specific interface. This validates the traffic path in hairpin/multi-homed scenarios.

### Loopback / Single-Host Testing

When client and server run on the same machine with traffic hairpinning through a BNG:

```
[server eth2: 10.0.0.2] ←── core ──→ [BNG] ←── access ──→ [client eth1: 10.255.0.2]
                        (same host)
```

**Important:** Linux kernel protections may block this:

1. **`rp_filter` (Reverse Path Filtering):** Packets arriving on `eth2` with a source IP from `eth1` are dropped as martian packets. Disable with: `sysctl -w net.ipv4.conf.all.rp_filter=0` and `sysctl -w net.ipv4.conf.<iface>.rp_filter=0`.
2. **Local routing optimization:** The kernel may short-circuit traffic destined to a local IP. Using `SO_BINDTODEVICE` on both sides + the BNG's routing should prevent this, but network namespaces (`ip netns`) provide the cleanest isolation.

### Report Changes

Add bind info to `StreamReport` when active:

Text output:
```
  Stream 0 [UDP latency ↑ DSCP=EF via eth1 (10.255.0.2)] 100pps
```

### Protocol Changes

Add bind info to `HelloMsg` for report labeling:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub bind_iface: Option<String>,
    pub source_ip: Option<String>,
}
```

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/socket.rs` | Create | Generic socket helpers: `bind_to_device()`, `bind_source_ip()`, combined `setup_socket()` |
| `src/lib.rs` | Modify | Add `pub mod socket;` |
| `src/traffic/generator.rs` | Modify | Add `bind_iface`/`source_ip` to config, route through socket2 when any option set |
| `src/traffic/tcp.rs` | Modify | Add `bind_iface`/`source_ip` to config, route through socket2 when any option set |
| `src/protocol/mod.rs` | Modify | Add `bind_iface`/`source_ip` to `HelloMsg` |
| `src/bin/client.rs` | Modify | Add 3 CLI flags, pass through to generators and control channel |
| `src/bin/server.rs` | Modify | Add `--data-bind-iface`, apply to receiver socket, read bind info from hello for report |
| `src/report/mod.rs` | Modify | Add bind fields to `StreamReport` |
| `src/report/text.rs` | Modify | Show bind info in stream header |

## Implementation Order

1. `src/socket.rs` — generic socket helpers using socket2
2. Generator changes — bind_iface/source_ip with socket2 routing
3. Protocol — bind fields in HelloMsg
4. CLI — client flags + server --data-bind-iface
5. Report — bind info in stream header
6. Server — apply data-bind-iface to receiver, read hello bind info for report

## Testing

- [ ] `bind_to_device()` via socket2 with invalid interface returns clear error
- [ ] `--source-ip` binds UDP socket to specified address (kernel bind() validates)
- [ ] `--bind-iface` calls SO_BINDTODEVICE on UDP socket
- [ ] `--source-ip` + `--bind-iface` combined works
- [ ] Socket2 path used when bind_iface set but no TOS (default path covered)
- [ ] Socket2 path used when source_ip set but no TOS
- [ ] TCP data socket respects source-ip and bind-iface
- [ ] `--control-bind-ip` binds control channel TCP
- [ ] Server `--data-bind-iface` constrains receiver socket
- [ ] Partial setup failure discards socket (no reuse)
- [ ] Bind info shown in text report stream header
- [ ] Bind info in JSON report
- [ ] No bind flags = default behavior unchanged (tokio direct path)
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] Bare metal: client binds to access interface, traffic through BNG

## Not In Scope

- VLAN creation on bare metal (user manages interfaces)
- DHCP on bare metal (user manages addressing)
- Auto-discovery of interfaces
- User-space source IP precheck (kernel bind() is authoritative)
