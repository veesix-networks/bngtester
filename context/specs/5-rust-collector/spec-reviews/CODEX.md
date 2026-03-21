<!-- Copyright The bngtester Authors -->
<!-- Licensed under the GNU General Public License v3.0 or later. -->
<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Spec Critique: Rust Collector -- Server and Client Binaries (Codex)

The core idea is good: dual-channel control/data separation, RRUL as a first-class mode, and Linux-native TCP telemetry are all appropriate for BNG testing. The spec is weakest where it needs an explicit lifecycle contract: failure handling, concurrent stream ownership, and report ownership are still described as happy-path behavior.

## Findings

### HIGH: the spec does not define failure behavior for control-channel loss, clock drift, or partial stream failure

- The control protocol is only specified as the happy path `hello -> ready -> clock_sync -> start -> stop -> results` (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:327-346`), even though the architecture also makes the control channel responsible for clock sync and final result exchange (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:36-38`).
- RRUL intentionally saturates the link with four TCP streams while the control socket may sit mostly idle for the duration of the loaded phase (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:98-109`). There is no heartbeat, no idle timeout, no abort contract, and no definition of whether either side must emit partial results if `stop` or `results` is lost.
- Clock offset is estimated once at test start, or skipped entirely in the default same-host path (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:84-86`). There is no handling for wrong same-host assumptions, offset drift during longer tests, or a state such as "latency estimated but not authoritative" when sync quality degrades.
- The same gap exists for stream failures: if one RRUL stream never connects, stalls, or ends early, the spec never says whether the session fails closed, continues degraded, or reports partial validity. A session state machine and failure taxonomy are missing.

### HIGH: concurrent stream management is still underspecified at the architecture level

- The design calls for multiple concurrent TCP and UDP streams, including reverse-path traffic from server to client (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:30-32`, `context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:100-113`), but the protocol never defines how streams are bound to sockets or ports. The `ready` message is described only as "server confirms ready to receive" (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:339-340`).
- UDP could be multiplexed on one socket by `stream_id`, but TCP cannot. For TCP, the spec needs a concrete mapping: per-stream ports, a small stream-identification handshake after connect, or a single accepted control-plane registry that binds each data connection to a stream.
- Reverse-path testing is also incomplete. The client CLI exposes only an outbound `<SERVER>` argument (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:308-325`), but downstream UDP/TCP requires the client to bind or accept something. There is no client-side listener/bind contract, no NAT/firewall assumption, and no file-plan entry for passive receivers or a stream/session registry.
- The file plan lists `src/traffic/generator.rs` and `src/traffic/tcp.rs`, but nothing like `session.rs`, `stream_registry.rs`, or a receiver-side coordination module (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:389-403`). The spec also never chooses a concurrency model (async runtime vs. thread-per-stream), which matters because RRUL needs a start barrier, per-second samplers, timers, signal handling, and coordinated shutdown.

### HIGH: report ownership is internally inconsistent

- The control-protocol section says result exchange allows both sides to produce complete reports (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:346`), and the configuration section says the test runner will invoke `bngtester-client` from subscriber images (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:374-376`).
- But only `bngtester-server` has `--output`, `--file`, `--raw-file`, and threshold flags (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:291-304`). Only Phase E mentions report generation (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:437-442`); Phase F does not mention merging remote results or writing artifacts on the client side (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:444-449`).
- That leaves the implementation with no authoritative answer to "where do JSON/JUnit artifacts come from?" If the server is remote and the client is the CI-invoked entrypoint, the client needs an explicit report-writing contract and matching file-plan coverage.

### MEDIUM: `TCP_INFO` is available on all three target platforms, but the spec frames the wrong portability risk

- I did not find a distro-level blocker here. Linux `tcp(7)` documents `TCP_INFO` since Linux 2.4, and both musl and glibc expose `TCP_INFO` plus `struct tcp_info` in `<netinet/tcp.h>`. That means Alpine musl, Debian, and Ubuntu are all viable targets for the specific socket option the spec wants.
- The real compatibility axis is kernel/header evolution, not Alpine vs. Debian vs. Ubuntu. The spec should explicitly pin the required fields (`tcpi_rtt`, `tcpi_rttvar`, `tcpi_total_retrans`, `tcpi_snd_cwnd`, etc.) and define behavior if the returned structure is shorter than expected or a field is not meaningful on a given kernel.
- This also deserves an explicit scope statement: `TCP_INFO` is Linux-specific. That is acceptable for the three current images, but it is not portable to the wider subscriber-platform list described elsewhere in the repo.

### MEDIUM: the file plan does not cover all of the functionality the spec says it will deliver

- The spec says the build context must move from `images/` to the repo root and explicitly calls out `publish-images.yml` as a required change (`context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:368`, `context/specs/5-rust-collector/IMPLEMENTATION_SPEC.md:451-455`), but `.github/workflows/publish-images.yml` is missing from the file plan. The current workflow still builds with `context: images/` in both build and push jobs (`.github/workflows/publish-images.yml:53-54`, `.github/workflows/publish-images.yml:90-91`).
- There is currently no `.dockerignore` in the repo root. Changing Docker context to `.` without adding one will send `.git`, `context/`, and other non-build inputs into every image build. That may not be a functional bug, but it is a real CI/cache design consequence and should either be planned explicitly or rejected explicitly.
- Issue #5 requires SPDX copyright headers on all files, and repo rules make that mandatory for new files (`CLAUDE.md:65-80`), but the spec's file plan and testing section never carry that acceptance criterion forward. With this many new Rust files, that omission is likely to become an implementation miss.

## Missing Additions

- Add an explicit session state machine with failure states for control loss, partial stream setup, early stream exit, and interrupted shutdown.
- Add an explicit clock mode contract: same-host vs. sync-estimated, how it is selected, how uncertainty is reported, and when one-way latency must be marked invalid or approximate.
- Add a concrete stream-binding model for concurrent TCP/UDP flows, including reverse-direction setup and client-side passive receive behavior.
- Add negative tests for control-channel drop during RRUL, one-stream RRUL startup failure, reverse-path stream failure, and SIGINT/SIGTERM before the final `results` exchange.

## Reference Notes

- Linux `tcp(7)`: https://man7.org/linux/man-pages/man7/tcp.7.html
- musl `<netinet/tcp.h>`: https://git.musl-libc.org/cgit/musl/tree/include/netinet/tcp.h
- glibc `<netinet/tcp.h>`: https://codebrowser.dev/glibc/glibc/sysdeps/gnu/netinet/tcp.h.html
