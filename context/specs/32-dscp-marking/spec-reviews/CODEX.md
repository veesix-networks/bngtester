# Spec Critique: DSCP/TOS Marking on Outgoing Packets (Codex)

The core mechanism is reasonable, but the spec is too loose in the places that matter most for a QoS-validation feature. In this repo today, the live outbound sockets are the client control TCP stream, the UDP data generator, and the server's accepted control TCP stream. The spec mostly discusses data sockets, leaves `setsockopt()` failure behavior undefined, and treats IPv6 as out of scope even though some current socket paths can already be IPv6.

## Findings

### HIGH: `setsockopt()` failure behavior is unspecified, which risks silently running the test with best-effort traffic

- The spec says DSCP is applied via `setsockopt()` before sending (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:45-57`) and that the feature exists to validate QoS classification (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:5`), but it never defines what happens if the socket option call fails.
- For this feature, silent fallback is the dangerous outcome. If the user asked for `--dscp EF` and the socket stays at default TOS 0, the test can still complete and produce a report, but the result is no longer evidence about QoS treatment for the requested class.
- `CAP_NET_ADMIN` is a real failure mode for some `IP_TOS` settings, but it should not be the only one the spec plans for. Linux `ip(7)` only says that some high-priority TOS levels may require `CAP_NET_ADMIN`; the broader implementation contract still needs to cover any `setsockopt()` failure path. Source: `ip(7)` on man7.org: <https://man7.org/linux/man-pages/man7/ip.7.html>.
- The spec should require one explicit behavior:
  - Fail fast before traffic starts if any requested DSCP cannot be applied to a required socket, or
  - Mark the affected stream/session failed and surface the reason in the control-plane error/report.
- The spec should also add a negative test for the unprivileged failure case instead of only positive `tcpdump` verification (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:139-150`).

### HIGH: the file plan does not cover all current socket creation paths implied by the overview

- The overview says this adds marking to `bngtester-client` and `bngtester-server` outgoing packets (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:5`).
- In the current code, outbound traffic does not come only from `src/traffic/generator.rs` and `src/traffic/tcp.rs`:
  - The client opens a control `TcpStream` to the server at `src/bin/client.rs:168` and sends `hello`, `clock_sync`, `start`, and `stop` messages on it (`src/bin/client.rs:172-237`, `src/bin/client.rs:267-268`).
  - The server accepts a control `TcpStream` and sends `error`, `ready`, `clock_sync`, `heartbeat`, and `results` messages on it (`src/bin/server.rs:74-83`, `src/bin/server.rs:104-107`, `src/bin/server.rs:128-132`, `src/bin/server.rs:142-147`, `src/bin/server.rs:299-300`, `src/bin/server.rs:331-335`).
  - The UDP generator in `src/traffic/generator.rs:65-66` and the standalone TCP generator in `src/traffic/tcp.rs:104-106` are only part of that surface.
- The spec then says the server does not need to set DSCP on its sockets because it does not send data streams in latency mode (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:85`). That is true only if the scope is "data-plane streams only". It is not true for "outgoing packets" in general, because the server already sends control-plane packets on TCP.
- The spec needs to choose and state one scope clearly:
  - If the feature is for all outbound packets, the file plan must include control-channel sockets in `src/bin/client.rs` and `src/bin/server.rs`.
  - If the feature is for test traffic only, the overview/current-state/report language should be narrowed so it does not claim full client/server packet coverage.

### MEDIUM: IPv6 cannot be treated as purely future work without an explicit IPv4-only contract

- The spec explicitly pushes `IPV6_TCLASS` out of scope (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:152-157`) while the design text still talks about setting DSCP on "sockets" generically (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:43-57`).
- Today, some current socket paths can already be IPv6 because the CLIs use `SocketAddr` and the control channel uses `TcpStream::connect(cli.server)` / `TcpListener::bind(cli.listen)` (`src/bin/client.rs:168`, `src/bin/server.rs:67`). If the helper always calls `IP_TOS`, IPv6 control connections will not be handled correctly.
- Linux `ipv6(7)` also notes that an IPv6 socket can be used with IPv4-mapped IPv6 addresses when `IPV6_V6ONLY` is false, so the implementation cannot infer the right socket option from CLI intent alone; it needs to respect the actual socket family on the fd. Source: `ipv6(7)` on man7.org: <https://man7.org/linux/man-pages/man7/ipv6.7.html>.
- There is a second IPv6 problem in the current codebase: the UDP data path hardcodes IPv4 wildcard binds (`src/traffic/generator.rs:65`, `src/bin/server.rs:120`), so an IPv6 target is already a mismatch even before DSCP marking is applied.
- The spec should either:
  - Explicitly reject IPv6 endpoints for this issue and document that the feature is IPv4-only for now, or
  - Include family-aware socket option handling plus the necessary IPv6 bind strategy now.

### MEDIUM: the JSON change is additive, but not strictly backward-compatible as written

- The spec adds `dscp` and `dscp_name` to `StreamReport` (`context/specs/32-dscp-marking/IMPLEMENTATION_SPEC.md:87-105`). The current JSON formatter is just `serde_json::to_writer_pretty(report)` over the report structs (`src/report/json.rs:10-17`), and `StreamReport` does not currently use `skip_serializing_if` on new top-level fields (`src/report/mod.rs:39-47`).
- That means the proposed `Option` fields will still change every serialized stream object shape when they are `None`, because they will be emitted as `null` unless the implementation adds `#[serde(skip_serializing_if = "Option::is_none")]`.
- Consumers that ignore unknown fields will probably be fine, but that is not the same as backward compatibility. Existing consumers that compare exact JSON, deserialize into strict schemas, or distinguish "field absent" from `"field": null` can break. The original report contract documented in spec #5 also does not include these fields (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:242-299`).
- If backward compatibility matters, the spec should require one of:
  - `skip_serializing_if` on the new fields so reports without DSCP do not change shape, or
  - An explicit schema/version bump note stating that JSON consumers must tolerate the new fields.
- There is also a practical file-plan gap here: adding fields to `StreamReport` will require updates to the report sample constructors/tests in `src/report/json.rs`, `src/report/text.rs`, and `src/report/junit.rs`, but `src/report/junit.rs` is not in the file plan.

## Reference Notes

- Linux `ip(7)`: <https://man7.org/linux/man-pages/man7/ip.7.html>
- Linux `ipv6(7)`: <https://man7.org/linux/man-pages/man7/ipv6.7.html>
