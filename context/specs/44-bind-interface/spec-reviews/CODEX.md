# Spec Critique: Bind Interface / Source IP (#44)

The feature direction is useful, but the spec currently overstates how much of the client socket setup already flows through `socket2`. The main implementation risk is that bind behavior gets added only to the TOS-specific branches and is therefore skipped on the default no-DSCP/no-ECN paths that bare-metal users are most likely to hit.

## Findings

### HIGH: the current socket-construction paths do bypass `socket2`, so bind logic will be missed unless the spec replaces those constructors instead of patching only the TOS branches

- `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:13-17` says all socket creation already uses `socket2` for TOS/DSCP support and that adding bind is straightforward.
- That is not true in the current code:
  - UDP uses `socket2` only when `tos` is set; the default path is [`src/traffic/generator.rs`](/home/brandon/osvbng-dev/bngtester/src/traffic/generator.rs#L66) via `UdpSocket::bind("0.0.0.0:0")` at [`src/traffic/generator.rs`](/home/brandon/osvbng-dev/bngtester/src/traffic/generator.rs#L81).
  - TCP uses `socket2` only when `tos` is set; the default path is `TcpStream::connect()` at [`src/traffic/tcp.rs`](/home/brandon/osvbng-dev/bngtester/src/traffic/tcp.rs#L130).
  - The control channel always uses `TcpStream::connect()` at [`src/bin/client.rs`](/home/brandon/osvbng-dev/bngtester/src/bin/client.rs#L264).
- There is a second scope wrinkle: the current client runtime only instantiates `UdpGeneratorConfig` / `run_udp_generator()` at [`src/bin/client.rs`](/home/brandon/osvbng-dev/bngtester/src/bin/client.rs#L409), so the live path today is UDP plus the control TCP socket, not the dormant TCP generator code in [`src/traffic/tcp.rs`](/home/brandon/osvbng-dev/bngtester/src/traffic/tcp.rs#L101).
- Phase 4 should make the constructor strategy explicit:
  - either always build client data/control sockets through `socket2`, or
  - route through `socket2` whenever any pre-connect socket option is requested (`tos`, `source_ip`, `bind_iface`, `control_bind_ip`).
- The testing section also needs explicit no-TOS coverage, otherwise the implementation can pass tests while still skipping bind on the default path.

### MEDIUM: `bind_to_device()` should not be added to `src/dscp.rs`; this is the point where the repo needs a generic socket helpers module

- The spec places `bind_to_device()` in `src/dscp.rs` at `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:39-60` and again in the file plan at `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:95-110`.
- `src/dscp.rs` already contains raw socket helpers, but they are still DSCP/ECN-specific: `apply_tos_to_fd()`, `apply_tos_to_socket()`, and `enable_recv_tos()` at [`src/dscp.rs`](/home/brandon/osvbng-dev/bngtester/src/dscp.rs#L193).
- The project summary already set the direction of keeping `dscp.rs` focused: `context/SUMMARY.md:154` says per-stream parsing helpers belong outside `dscp.rs` to avoid turning it into a junk drawer.
- `SO_BINDTODEVICE` is not DSCP/ECN-specific. Adding it here makes `dscp.rs` a general Linux socket-options module without naming it as such.
- Phase 4 should introduce something like `src/socket.rs` or `src/net/socket.rs` for generic socket-option helpers. If that feels too large for this issue, the fallback should still be "new generic module for bind helper", not "add another unrelated helper to `dscp.rs`."

### MEDIUM: the spec leaves the partial-configuration failure path implicit; it should either state that the socket is discarded on any setup error or reorder the setup steps

- The TCP sequence in `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:35` is `new()` -> `set_tos()` -> `bind(source_ip)` -> `SO_BINDTODEVICE` -> `connect()`. UDP only says to call `SO_BINDTODEVICE` "after creation" at `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:33`.
- If `SO_BINDTODEVICE` fails after TOS has already been set, the underlying file descriptor is still a valid socket. It is only safe in bngtester if the implementation returns immediately and lets that `socket2::Socket` drop without ever converting or reusing it.
- That matches the current fail-fast shape in the TOS code paths at [`src/traffic/generator.rs`](/home/brandon/osvbng-dev/bngtester/src/traffic/generator.rs#L68) and [`src/traffic/tcp.rs`](/home/brandon/osvbng-dev/bngtester/src/traffic/tcp.rs#L107): an error returns before the socket escapes into Tokio.
- The spec should make that contract explicit: any pre-connect setup failure aborts the test startup and discards the socket; no partially configured socket is reused.
- If Phase 4 wants a cleaner ordering, the safer sequence is `new()` -> `SO_BINDTODEVICE` -> `bind(source_ip)` -> `set_tos()` -> `connect()`. That makes the privilege/interface-sensitive failure happen before the final TOS mutation while still preserving the "TOS before connect" requirement for TCP SYN marking.

### MEDIUM: `--source-ip` needs an explicit validation contract, but "must already be assigned to the machine" is not necessarily the right one

- The spec introduces `--source-ip` at `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:25-35` and tests the happy path at `context/specs/44-bind-interface/IMPLEMENTATION_SPEC.md:117-129`, but it never says whether this is parse-only validation or local-address validation.
- A blanket user-space precheck for "is this IP assigned somewhere on the machine?" is not automatically better than `bind()`:
  - the kernel `bind()` call is the authoritative check and already fails fast on typical hosts when the address is not local;
  - a machine-wide precheck can reject valid advanced setups that intentionally allow non-local bind;
  - if `--bind-iface` and `--source-ip` are combined, "assigned somewhere on the machine" is too weak anyway. The more useful validation would be "assigned to the selected interface."
- Given the current repo has no local address-enumeration helper or dependency, the minimum spec should explicitly rely on `bind()` failure and require a clear startup error for `EADDRNOTAVAIL` / related cases.
- If Phase 4 wants friendlier UX beyond that, it should scope the validation precisely: validate against the selected interface when both flags are present, not merely against the machine as a whole.
