# Spec Critique: CGNAT-Aware Reporting (#50)

The feature is worth doing, but the current spec treats the control socket as if it were a trustworthy proxy for subscriber dataplane identity. That assumption leaks into both the fallback-address design and the text rendering rules, and it also makes the backward-compatibility claim stronger than the actual schema change supports.

## Findings

### HIGH: falling back to the control socket local address can misreport the management/control-plane IP as the subscriber IP

- The spec says `subscriber_ip` should come from `HelloMsg.source_ip`, with a fallback to "the control channel socket's local address" at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:27-29` and again at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:33-40`.
- In the current client, the control channel is its own path:
  - it uses `resolved.control_bind_ip` when set, otherwise plain `TcpStream::connect(resolved.server)` at `src/bin/client.rs:492-516`;
  - the hello currently sends only `resolved.source_ip` at `src/bin/client.rs:573-590`.
- The actual dataplane socket is configured separately in `run_udp_generator()`, where `bind_iface` / `source_ip` are applied to the UDP socket used for test traffic at `src/traffic/generator.rs:68-98`.
- So the control socket local address is not "the subscriber IP" in any strong sense. It may be:
  - the explicit `--control-bind-ip`,
  - a management IP chosen by the control-plane route,
  - or some other non-dataplane source selected by the kernel.
- This matters because the spec's fallback turns "unknown subscriber IP" into a confidently wrong value. In mixed control/data-path topologies, that is worse than leaving the field absent.
- Phase 4 should tighten the contract:
  - use `--source-ip` when explicitly set;
  - otherwise derive the address from the actual dataplane socket only if the implementation can prove that mapping;
  - otherwise leave `subscriber_ip` unset.
- If the goal is "always populate", the spec needs a real dataplane-address discovery design. The current hello-before-ready sequence does not get that for free.

### HIGH: adding `subscriber_ip` inside `TestConfig` is not strictly backward compatible for existing JSON consumers

- The spec adds `subscriber_ip` to `TestConfig` at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:44-49` and calls the result backward compatible at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:101-102`.
- Today `TestConfig` is the core `test` object for every single-report JSON payload at `src/report/mod.rs:31-37`. Changing it changes the schema for all `TestReport` JSON, not just combined multi-client output.
- There are two separate compatibility issues here:
  - If Phase 5 literally follows the spec snippet, `Option<String>` without `#[serde(skip_serializing_if = "Option::is_none")]` will serialize as `"subscriber_ip": null` when absent, which contradicts the spec's own "field omitted" test.
  - Even with `skip_serializing_if`, this is only additive-schema compatible for consumers that ignore unknown fields. Any strict decoder or schema validator for the existing `test` object will break once `subscriber_ip` appears.
- The repo already uses `skip_serializing_if` for optional report fields elsewhere at `src/report/mod.rs:24-28` and `src/report/mod.rs:46-57`, so the spec should state that requirement explicitly instead of leaving it implicit.
- Phase 4 should make an explicit contract decision:
  - either call this what it is, an additive JSON schema change that requires tolerant consumers,
  - or keep `TestConfig` unchanged and put the new field somewhere that does not mutate the established single-report `test` object.

### MEDIUM: the "simplified when `peer == subscriber_ip`" rule compares unlike values and will not hold once ports are involved

- The spec says to render the simplified header when `peer == subscriber_ip` at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:67-70` and tests the same condition at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:99-100`.
- In the current code, `peer` is always a stringified `SocketAddr` from `peer.to_string()` at `src/bin/server.rs:501-502` and `src/bin/server.rs:891-895`. That means it includes a port.
- The proposed `subscriber_ip` value comes from `HelloMsg.source_ip`, which is IP-only today at `src/protocol/mod.rs:25-48`.
- A raw equality check therefore compares different shapes:
  - `peer`: `10.255.0.2:43210`
  - `subscriber_ip`: `10.255.0.2`
- So the simplified branch never fires unless the implementation silently switches from string equality to IP-only comparison. The sample output at `context/specs/50-cgnat-reporting/IMPLEMENTATION_SPEC.md:68-69` already assumes that IP-only comparison, because it keeps the peer port in the simplified rendering.
- Phase 4 should say that explicitly: parse `peer` as a socket address, compare `peer.ip()` to `subscriber_ip`, and only use the simplified form when the IPs match. On parse failure, fall back to the fully labeled `peer/subscriber` form.
- The test plan should also cover the real edge case here: same IP but different rendered values because `peer` includes `:port`.
