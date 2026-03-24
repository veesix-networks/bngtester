# Spec Critique: ECN Marking and Detection on Test Traffic (Codex)

The sender-side ECN bit-setting is straightforward. The weaker part of the spec is the receiver story: it depends on Linux ancillary data, but it does not define the failure contract clearly enough, and it treats replacing `recv_from()` with `recvmsg()` as a local mechanical swap when the current receiver loops are built around Tokio's async readiness model.

## Findings

### HIGH: `IP_RECVTOS` / missing-cmsg failure behavior is underspecified, which can turn "could not observe ECN" into a false zero-CE result

- The spec says the receiver must enable `IP_RECVTOS`, switch to `recvmsg`, extract `IP_TOS` from ancillary data, and count CE marks (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:55-68`), but it never defines what happens if that observation path is unavailable.
- There are two materially different failure modes here:
  - `setsockopt(IP_RECVTOS)` fails at socket setup, for example because the platform/socket path does not support it. Linux `ip(7)` documents `IP_RECVTOS` as an ancillary-data feature for incoming packets, but the spec never says whether receiver startup should fail fast when it cannot be enabled. Source: <https://man7.org/linux/man-pages/man7/ip.7.html>.
  - `recvmsg` returns a datagram but no usable `IP_TOS` cmsg. The spec does not say whether that packet should be treated as `Not-ECT`, ignored for ECN accounting, counted as "unknown", or treated as a stream/session error.
- For this feature, silently treating "no cmsg observed" as "CE not set" is the dangerous outcome. It produces a clean-looking `ecn_ce_received = 0` result even though the tool may have failed to observe the ECN state at all.
- The spec should require an explicit contract:
  - If `IP_RECVTOS` cannot be enabled on a required receiver socket, fail the receiver setup before the test starts.
  - If a packet arrives without usable ECN ancillary data after receiver setup succeeded, do not fold that packet into a false "no CE" result. Either mark ECN observation unavailable for the stream/session, or add an explicit "unknown ECN observations" path and omit CE ratio when observation is not trustworthy.
- The testing section only covers positive extraction and CE-counting cases (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:146-160`). It should add negative tests for unsupported `IP_RECVTOS` and for `recvmsg` returning no usable ECN cmsg.

### HIGH: the "just replace `recv_from()` with `recvmsg()`" approach is not enough to preserve the current Tokio async pattern

- The spec presents two implementation approaches and leans toward the simpler one: call raw `libc::recvmsg` on the existing `tokio::net::UdpSocket` fd instead of moving to `AsyncFd` (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:64-68`).
- In the current code, both UDP receiver loops rely on `tokio::select!` with an async receive future:
  - `src/traffic/receiver.rs:46-79`
  - `src/bin/server.rs:190-230`
- A direct blocking `recvmsg` call inside those async tasks is not a drop-in replacement for `recv_from().await`. It changes the runtime behavior in three important ways:
  - It bypasses Tokio's readiness tracking unless the implementation uses `UdpSocket::readable()` / `try_io()` or `AsyncFd`.
  - It can block the task on the raw fd, which means the cancellation branch in `tokio::select!` is no longer able to preempt the receive wait.
  - It introduces `WouldBlock` / false-readiness handling that the spec never mentions.
- Tokio's own `UdpSocket` docs explicitly pair readiness methods with `try_*` / `try_io` calls and document cancel-safety around that model. Source: <https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html>.
- This matters for existing behavior. Today, the server can receive `Stop`, cancel the token, and let the receiver task exit promptly even if no more UDP packets arrive. If the receiver is parked in a raw blocking `recvmsg`, that shutdown path can wedge until another datagram arrives or the fd is otherwise interrupted.
- The spec should make the async contract explicit: use Tokio readiness integration (`readable()` + `try_io()` around a raw `recvmsg`, or `AsyncFd`) and define how `WouldBlock`, cancellation, and final-packet shutdown work.

### HIGH: the file plan does not cover all receiver code paths the spec itself identifies

- The current-state section correctly names two receiver implementations that use `recv_from()`: `src/traffic/receiver.rs` and the inline receiver in `src/bin/server.rs` (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:13-16`).
- The file plan only schedules receiver-side changes in `src/bin/server.rs` (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:120-133`).
- That leaves one of two unresolved outcomes:
  - `src/traffic/receiver.rs` stays on `recv_from()` and can never observe CE, so the repo ends up with two divergent UDP receiver behaviors.
  - The implementation quietly edits `src/traffic/receiver.rs` anyway, which means the spec's file plan is incomplete.
- Today `run_udp_receiver()` is not called anywhere (`src/traffic/receiver.rs:29`), but it is still a live code path in the crate and is explicitly part of the spec's stated current surface. Leaving it stale is exactly the kind of dead-path drift Phase 3 is supposed to catch.
- The spec should either add `src/traffic/receiver.rs` to the file plan and define how its return type carries ECN observation state, or explicitly say that the helper is intentionally out of scope / scheduled for later removal.

### MEDIUM: the new `StreamResults` fields can be backward-compatible, but only if the implementation follows the current omission semantics exactly

- On the report side, the spec is directionally correct: it proposes `Option` fields on `StreamResults` with `skip_serializing_if` (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:91-102`). That matches the current report model in `src/report/mod.rs:53-79`, where optional metrics are omitted from JSON entirely when absent.
- If implemented exactly that way, the new `StreamResults` fields themselves are backward-compatible for existing JSON consumers when ECN is off: the keys are simply absent, preserving current shape. This is also consistent with the explicit test requirement that JSON output remain unchanged without ECN (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:157-158`).
- The compatibility risk is that the spec is looser elsewhere than it is here:
  - The file plan also says "ECN mode to `StreamReport`" (`context/specs/33-ecn-marking/IMPLEMENTATION_SPEC.md:130`), but the design section never defines that field or states that it must also be optional + omitted when unset.
  - The failure modes above matter here too. If the receiver cannot observe ECN reliably and the implementation still serializes `ecn_ce_received: 0` / `ecn_ce_ratio: 0.0`, that is a semantic compatibility break even if the JSON schema is technically additive.
- The spec should state the compatibility rule more precisely: all new report fields must be optional, omitted when ECN is disabled or observation is unavailable, and zero values must mean "observed zero CE marks", not "ECN not observed".

## Reference Notes

- Linux `ip(7)` `IP_RECVTOS`: <https://man7.org/linux/man-pages/man7/ip.7.html>
- Tokio `UdpSocket` readiness / `try_io` docs: <https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html>
