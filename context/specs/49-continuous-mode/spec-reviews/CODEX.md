# Spec Critique: Continuous/Resilient Mode with Reconnect and Failover Metrics (#49)

The feature direction is useful, but the current spec treats reconnect as if it were a light interruption inside one long-lived session. In the current codebase, socket creation, `Ready.udp_port`, server metrics collection, and report production are all session-scoped. That makes the generator error-handling and report-shape decisions more structural than the spec currently acknowledges.

## Findings

### HIGH: the "UDP resilience" section conflates packet loss with socket/send failures, and blindly continuing on `send()` error is likely the wrong recovery model

- The overview and UDP section frame this as "UDP streams are resilient to packet loss" and then implement that by continuing after `socket.send()` errors at `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:5`, `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:42-61`.
- Those are different problems:
  - ordinary UDP packet loss is already non-fatal at the sender because the sender usually never sees an error for dropped packets;
  - a real `send()` error means the local socket/path state is bad enough that the kernel refused the send.
- In the current generator, the UDP socket is created once at `src/traffic/generator.rs:64-99` and then reused for the whole run at `src/traffic/generator.rs:107-169`. There is no in-generator socket repair path.
- So if continuous mode just does `send_errors += 1; continue`, it keeps hammering the same broken socket. On the unlimited-rate path at `src/traffic/generator.rs:107-133`, that also creates a tight retry loop with no backoff at all.
- Phase 4 should treat `send()` failure as a transport failure event, not as proof of packet loss. The spec should require the error to surface to the reconnect/orchestration layer so the client can tear down and recreate the generator socket. If the design really wants to continue on some errors, it should enumerate which errors are considered transient and why.

### HIGH: reconnect implies a new `Hello`/`Ready` handshake and a new server UDP port, so the generator cannot simply "pause" and "resume"

- The reconnect flow says "pause data streams" and later "resume data streams" after a new `Hello` / `Ready` exchange at `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:63-75`.
- In the current client, `Ready.udp_port` is used to construct the data target at `src/bin/client.rs:593-647`, and `run_udp_generator()` is then started with that one target at `src/bin/client.rs:663-677`.
- In the current server, every session allocates a fresh UDP socket and fresh `udp_port` before sending `Ready` at `src/bin/server.rs:570-608`.
- That means a reconnect is not "resume the old generator." It is:
  - cancel and discard the old generator;
  - reconnect control;
  - send a new `Hello`;
  - receive a new `Ready` with a new UDP port;
  - construct a new generator/socket for that new target.
- The current generator API has no way to retarget an existing connected UDP socket. It only supports creating a fresh socket inside `run_udp_generator()` at `src/traffic/generator.rs:64-99`.
- There is also a server-correlation wrinkle. The spec says `--client-id` lets the server correlate reconnects at `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:75`, but the current server intentionally suffixes duplicate IDs (`foo`, `foo-1`, ...) at `src/bin/server.rs:129-139` and `src/bin/server.rs:554-559`. So reconnect aggregation is a client-side responsibility unless the spec adds explicit server correlation behavior.
- Phase 4 should make the session boundary explicit and require tests for "old generator canceled, new UDP port learned, new generator created" instead of describing reconnect as a generic pause/resume.

### HIGH: `FailoverMetrics` likely needs a separate top-level continuous-mode report, not just another field on `TestReport`

- The spec adds `FailoverMetrics` to "the report" at `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:93-120`, but it does not define whether that means `TestReport` or a new wrapper type.
- Today `TestReport` is the core single-session report shape consumed by all formatters at `src/report/mod.rs:17-29`. It carries one `status`, one `clock_mode`, one `histogram`, one `time_series`, and one set of stream results.
- The server also produces one `TestReport` per accepted session at `src/bin/server.rs:889-899`.
- Continuous failover metrics span reconnect boundaries, which means they can cover:
  - multiple `Hello` / `Ready` / `Results` exchanges,
  - multiple server UDP ports,
  - multiple per-session histograms and time-series segments,
  - outage intervals where there is no active session at all.
- The repo already solved a similar shape mismatch by introducing a separate `CombinedReport` wrapper for multi-client output instead of overloading `TestReport` at `src/report/mod.rs:124-140`.
- Continuous mode looks closer to that pattern than to a plain `TestReport` extension. A dedicated top-level type such as `ContinuousReport` can own the cross-session `failover` data and either embed per-session reports or define explicit aggregation rules.
- If Phase 4 keeps `FailoverMetrics` inside `TestReport`, it needs to specify what happens to `status`, `clock_mode`, `histogram`, `time_series`, and stream counters across reconnects. Without that, the report shape becomes ambiguous.

### MEDIUM: `--max-reconnects` defaulting to 10 is too low for multi-hour HA testing unless the spec turns it into a time-budgeted or unlimited retry policy

- The spec sets exponential backoff with a 30s cap and a default `--max-reconnects 10` at `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:69-73` and `context/specs/49-continuous-mode/IMPLEMENTATION_SPEC.md:139`.
- With the stated backoff, 10 consecutive failed reconnects only buys roughly 181 seconds of retry time (`1 + 2 + 4 + 8 + 16 + 30 + 30 + 30 + 30 + 30`).
- That is a short recovery budget for the exact use case this feature targets: long-running HA/failover exercises, maintenance windows, orchestrated restarts, or repeated control-plane flaps over hours.
- The spec also does not say whether the reconnect counter resets after a successful reconnect. If it is lifetime-total, 10 is even less defensible for multi-hour tests.
- Phase 4 should either:
  - make the default unlimited in continuous mode,
  - use `0` / `None` as "retry forever until SIGINT",
  - or set a much higher default and define whether the counter is consecutive-failures-only or lifetime-total.
