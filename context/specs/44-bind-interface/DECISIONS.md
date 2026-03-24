# Decisions: 44-bind-interface

## Accepted

### SO_BINDTODEVICE relaxed since kernel 5.7
- **Source:** GEMINI (G1)
- **Severity:** LOW
- **Resolution:** Documentation updated to note CAP_NET_RAW only required on kernels < 5.7.

### Use socket2::bind_device() instead of raw libc
- **Source:** GEMINI (G2)
- **Severity:** LOW
- **Resolution:** Using `socket2::Socket::bind_device(Some(iface.as_bytes()))`. Handles null termination, is idiomatic, already available in deps.

### Interface name null termination handled by socket2
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Resolution:** Moot — using socket2::bind_device() handles this safely.

### Server needs --data-bind-iface for hairpin validation
- **Source:** GEMINI (G4)
- **Severity:** MEDIUM
- **Resolution:** Added `--data-bind-iface` flag to server for constraining receiver socket to a specific interface.

### Loopback requires rp_filter documentation
- **Source:** GEMINI (G5)
- **Severity:** HIGH
- **Resolution:** Added "Loopback / Single-Host Testing" section documenting rp_filter sysctl requirements and network namespace alternative.

### Default paths bypass socket2 — must route through socket2 for all pre-connect options
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** Socket2 used whenever any pre-connect option is set (tos, source_ip, bind_iface). Default tokio-direct path preserved when no options set. Testing explicitly covers bind-without-TOS case.

### Socket helpers in src/socket.rs, not dscp.rs
- **Source:** CODEX (C2)
- **Severity:** MEDIUM
- **Resolution:** Created `src/socket.rs` for generic socket helpers (bind_to_device, bind_source_ip, setup_socket). Keeps dscp.rs focused on DSCP/ECN.

### Safer setup ordering: SO_BINDTODEVICE before TOS
- **Source:** CODEX (C3)
- **Severity:** MEDIUM
- **Resolution:** Setup order: new() → SO_BINDTODEVICE → bind(source_ip) → set_tos() → connect(). Privilege-sensitive operation fails first. Any failure discards socket — no partial reuse.

### Source IP validation via kernel bind(), not user-space precheck
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** Rely on bind() EADDRNOTAVAIL for validation. No user-space IP enumeration. Clear error message on failure.
