# Implementation Spec: Bind Interface / Source IP for Bare Metal and Loopback Testing

## Overview

Add `--bind-iface` and `--source-ip` flags to bngtester-client so it can bind data sockets to a specific network interface or source IP address. This enables bare metal BNG testing (client on a physical server, traffic through a real BNG) and loopback/hairpin testing (client and server on the same machine, different interfaces).

## Source Issue

[#44 — Bind interface / source IP for bare metal and loopback testing](https://github.com/veesix-networks/bngtester/issues/44)

## Current State

- UDP data sockets bind to `0.0.0.0:0` (any interface, OS-assigned port) in `src/traffic/generator.rs`.
- TCP data sockets are created via `socket2::Socket::new()` without binding before connect in `src/traffic/tcp.rs`.
- The control channel TCP socket uses `TcpStream::connect()` with no bind in `src/bin/client.rs`.
- All socket creation already uses `socket2` for TOS/DSCP support — adding bind is straightforward.
- The server's UDP receiver socket also binds to `0.0.0.0:0`.

## Design

### CLI Flags

| Flag | Where | Default | Description |
|------|-------|---------|-------------|
| `--bind-iface <NAME>` | Client | _(none)_ | Bind data sockets to a specific interface via `SO_BINDTODEVICE` |
| `--source-ip <ADDR>` | Client | _(any)_ | Bind data sockets to a specific source IP |
| `--control-bind-ip <ADDR>` | Client | _(any)_ | Bind control channel TCP to a specific source IP |

`--bind-iface` and `--source-ip` can be combined. `--bind-iface` requires `CAP_NET_RAW` or root on Linux.

### Socket Binding

**UDP data sockets:** Currently bind to `0.0.0.0:0`. With `--source-ip`, bind to `<source_ip>:0`. With `--bind-iface`, call `setsockopt(SO_BINDTODEVICE)` after creation.

**TCP data sockets:** Currently use `socket2::Socket::new()` → `set_tos()` → `connect()`. Add bind step: `socket2::Socket::new()` → `set_tos()` → `bind(<source_ip>:0)` → `SO_BINDTODEVICE` → `connect()`.

**Control channel TCP:** Currently `TcpStream::connect()`. With `--control-bind-ip`, create via socket2 → `bind(<control_bind_ip>:0)` → `connect()`.

### SO_BINDTODEVICE

```rust
pub fn bind_to_device(fd: RawFd, iface: &str) -> Result<(), String> {
    let iface_bytes = iface.as_bytes();
    let ret = unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_BINDTODEVICE,
            iface_bytes.as_ptr() as *const libc::c_void,
            iface_bytes.len() as libc::socklen_t,
        )
    };
    if ret != 0 {
        return Err(format!("SO_BINDTODEVICE({iface}): {}", std::io::Error::last_os_error()));
    }
    Ok(())
}
```

**Fail-fast:** If `SO_BINDTODEVICE` fails (interface doesn't exist, no permissions), abort before the test starts. Same pattern as DSCP/ECN fail-fast.

### Generator Config Changes

Add bind options to `UdpGeneratorConfig` and `TcpGeneratorConfig`:

```rust
pub struct UdpGeneratorConfig {
    // ... existing fields ...
    pub bind_iface: Option<String>,
    pub source_ip: Option<IpAddr>,
}
```

### Control Protocol

Add bind info to `HelloMsg` for report labeling:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub bind_iface: Option<String>,
    pub source_ip: Option<String>,
}
```

### Report Changes

Add bind info to `StreamReport` when bind is active:

Text output:
```
  Stream 0 [UDP latency ↑ DSCP=EF via eth1 (10.255.0.2)] 100pps
```

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/dscp.rs` | Modify | Add `bind_to_device()` helper |
| `src/traffic/generator.rs` | Modify | Add `bind_iface` and `source_ip` to `UdpGeneratorConfig`, apply on socket creation |
| `src/traffic/tcp.rs` | Modify | Add `bind_iface` and `source_ip` to `TcpGeneratorConfig`, apply before connect |
| `src/protocol/mod.rs` | Modify | Add `bind_iface` and `source_ip` to `HelloMsg` |
| `src/bin/client.rs` | Modify | Add `--bind-iface`, `--source-ip`, `--control-bind-ip` CLI flags, pass through |
| `src/bin/server.rs` | Modify | Read bind info from hello, include in report |
| `src/report/mod.rs` | Modify | Add bind fields to `StreamReport` |
| `src/report/text.rs` | Modify | Show bind info in stream header |

## Implementation Order

1. `src/dscp.rs` — `bind_to_device()` helper with fail-fast
2. Generator changes — bind_iface/source_ip on UDP and TCP sockets
3. Protocol — bind fields in HelloMsg
4. CLI — 3 new flags on client
5. Report — bind info in stream header
6. Server — read from hello, include in report

## Testing

- [ ] `bind_to_device()` with invalid interface name returns clear error
- [ ] `--source-ip` binds UDP socket to specified address
- [ ] `--bind-iface` calls SO_BINDTODEVICE on UDP socket
- [ ] `--source-ip` + `--bind-iface` combined
- [ ] TCP data socket respects source-ip binding
- [ ] `--control-bind-ip` binds control channel TCP
- [ ] Bind info shown in text report stream header
- [ ] Bind info in JSON report
- [ ] No bind flags = default behavior unchanged
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] Bare metal: client binds to access interface, traffic through BNG

## Not In Scope

- VLAN creation on bare metal (user manages interfaces)
- DHCP on bare metal (user manages addressing)
- Auto-discovery of interfaces
- Server-side bind (server already binds to `--listen` address)
