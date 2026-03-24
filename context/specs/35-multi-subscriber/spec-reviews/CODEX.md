# Spec Critique: Multi-Subscriber Coordination (#35)

The concurrency direction is right, but the spec still leaves several compile-time and liveness details implicit. The main risk is that "spawn each session" sounds mechanical in the design, while the current server/report code is still shaped around one synchronous session owning the output path.

## Findings

### HIGH: spawning `handle_session()` needs a real signature change, not just an `Arc` in the call-site sketch

- The design swaps `handle_session(stream, peer, &cli, &thresholds).await` for `tokio::spawn(async move { ... })` in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:21-34`, and implementation order mentions "share CLI/thresholds via Arc" in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:152-159`.
- In the current code, `handle_session()` still borrows `&Cli` and `&Thresholds` and returns `Result<(), Box<dyn std::error::Error>>` in [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L93) through [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L98).
- That is not spawn-ready as written:
  - the spawned future cannot hold non-`'static` borrows of `cli` / `thresholds` from `main()`;
  - `Cli` is not `Clone`, so the spec cannot hand-wave ownership unless it defines an owned config shape;
  - `Box<dyn std::error::Error>` is also the wrong default task output for spawned supervision, because the task boundary needs a sendable result or the task has to log/store the failure internally.
- Phase 4 should make the ownership model explicit: either pass `Arc<ServerConfig>` / `Arc<Thresholds>` (or smaller owned clones of just the needed fields), and either make the task result `Send` or keep errors inside the task and record them in the registry.

### HIGH: combined mode can hang forever or silently omit a client if one session task panics or wedges

- The spec says `SessionRegistry` is just an `Arc<Mutex<Vec<CompletedSession>>>` and that sessions append when complete in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:55-71`. Combined mode then waits for "all sessions" or `--max-clients N` in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:111-119` and `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:136-138`.
- That is not enough to define failure behavior. If a spawned session panics or hangs before it pushes a `CompletedSession`, the completed-count never reaches `N`, so the combined report waiter has no way to know whether it should keep waiting or emit a partial/failure result.
- The current single-session code already drops one join failure on the floor: it ignores `recv_handle.await` in [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L413), then errors out only if no metrics were stored in [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L417). That pattern becomes worse once session tasks themselves are also detached.
- Heartbeat timeout only covers the remote control channel path in [src/protocol/session.rs](/home/brandon/osvbng-dev/bngtester/src/protocol/session.rs), not an internally wedged or panicked task.
- Phase 4 should require supervisory state, not just a completed-results vector: at minimum a `JoinSet` / tracked `JoinHandle`s plus explicit failed/interrupted session entries, and a defined partial-report behavior for `--combined`.

### HIGH: preserving per-session output as the default becomes unsafe once sessions run concurrently

- The lifecycle section says per-session output remains the default in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:113-119`.
- Today each session writes its report directly inside `handle_session()` by creating the output sink and calling `write_json` / `write_junit` / `write_text` in [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L530) through [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L541).
- With concurrent tasks, that creates two concrete failure modes the spec does not mention:
  - `--file <PATH>` becomes last-writer-wins because every task does `File::create(path)`;
  - stdout/stderr reports can interleave across sessions, producing corrupted human-readable output.
- This is not just polish. The current "current behavior, preserved" statement stops being true under concurrency unless the spec adds a writer coordinator, a mutex around shared output, append semantics, or a restriction that per-session `--file` output is unsupported in multi-client mode.

### MEDIUM: `SIGUSR1` snapshots are implementable without blocking `accept()`, but the spec is internally inconsistent and the file plan omits the work

- The lifecycle section says `SIGUSR1` writes a snapshot in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:111-119`, but the very next sentence says the initial implementation only adds option 2 (`--combined` + `--max-clients`) in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:119`.
- The file plan and testing section also omit any signal-handling work in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:140-173`.
- This is implementable without blocking the accept loop, but only if it runs in a separate task that listens for the Unix signal and snapshots the registry. It should not be done inline in the accept loop.
- Phase 4 should pick one of two paths:
  - remove `SIGUSR1` from the initial spec entirely, or
  - keep it and add explicit server-side signal handling, snapshot semantics, and tests to the file plan.

### MEDIUM: the file plan for `CombinedReport` is incomplete at the report API boundary

- The file plan adds `CombinedReport` / `ClientReport` to `src/report/mod.rs` and updates the formatter modules in `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md:140-150`.
- The current formatter APIs are still hard-typed to `TestReport`:
  - JSON: [src/report/json.rs](/home/brandon/osvbng-dev/bngtester/src/report/json.rs#L9) through [src/report/json.rs](/home/brandon/osvbng-dev/bngtester/src/report/json.rs#L17)
  - Text: [src/report/text.rs](/home/brandon/osvbng-dev/bngtester/src/report/text.rs#L10) through [src/report/text.rs](/home/brandon/osvbng-dev/bngtester/src/report/text.rs#L17)
  - JUnit: [src/report/junit.rs](/home/brandon/osvbng-dev/bngtester/src/report/junit.rs#L18) through [src/report/junit.rs](/home/brandon/osvbng-dev/bngtester/src/report/junit.rs#L29)
- Both bins call those `TestReport`-only functions directly in [src/bin/client.rs](/home/brandon/osvbng-dev/bngtester/src/bin/client.rs#L532) through [src/bin/client.rs](/home/brandon/osvbng-dev/bngtester/src/bin/client.rs#L534) and [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L538) through [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L541).
- The missing decision is the important one: does the repo gain separate `write_combined_*` functions, a report enum/wrapper, or server-only branching around the formatters? Until that is spelled out, the file plan does not fully describe how `CombinedReport` actually fits into the current report API.

## UDP Port Note

Two live sessions should not accidentally share a UDP port under the current strategy.

- The server binds each session socket with `UdpSocket::bind("0.0.0.0:0")` in [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L124) through [src/bin/server.rs](/home/brandon/osvbng-dev/bngtester/src/bin/server.rs#L127).
- That already delegates active port uniqueness to the kernel, so the spec does not need a separate user-space port allocator for concurrent sessions.
- The subtler risk is after a crashed session drops its socket: the port can be reused later while a stale client is still sending. That is a task-failure cleanup problem, not an active-session conflict problem, and it reinforces the need for explicit session supervision.
