# Implementation Spec: Multi-Subscriber Coordination (Concurrent Client Sessions)

## Overview

Enable bngtester-server to handle multiple concurrent client sessions, each from a different subscriber container. Each session gets its own control channel, data ports, and independently tracked metrics. The server produces a combined report with per-client breakdown. Clients are identified by a `client_id` in the hello message (defaulting to source IP).

## Source Issue

[#35 — Multi-subscriber coordination (concurrent client sessions)](https://github.com/veesix-networks/bngtester/issues/35)

## Current State

- The server accepts one client at a time — `handle_session()` is awaited sequentially in the accept loop (line 74-90 of `src/bin/server.rs`).
- Each session gets its own UDP socket bound to an ephemeral port.
- Metrics, report generation, and control protocol are all per-session.
- The server report only contains one client's results.
- No client identification in the hello message.

## Design

### Server Concurrency Model

The accept loop spawns each session as a concurrent tokio task:

```rust
// Current: sequential
let result = handle_session(stream, peer, &cli, &thresholds).await;

// New: concurrent
tokio::spawn(async move {
    let result = handle_session(stream, peer, cli_arc, thresholds_arc, sessions.clone()).await;
    // ...
});
```

Each spawned session:
- Gets its own UDP data socket (separate ephemeral port)
- Tracks its own metrics independently (latency, loss, jitter, throughput, ECN)
- Runs its own heartbeat
- Produces its own session-level results

### Client Identification

Add `client_id` to `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub client_id: Option<String>,
}
```

CLI flag on the client: `--client-id <NAME>`. Defaults to source IP if not provided. Used in the combined report to identify which subscriber each result set belongs to.

### Session Registry

A shared `SessionRegistry` tracks active sessions:

```rust
pub struct SessionRegistry {
    sessions: Arc<Mutex<Vec<CompletedSession>>>,
}

pub struct CompletedSession {
    pub client_id: String,
    pub peer: SocketAddr,
    pub report: TestReport,
}
```

When a session completes, it pushes its `TestReport` into the registry. The server can output a combined report after all sessions finish (or on-demand via a signal).

### Combined Report

The combined report wraps multiple per-client reports:

```json
{
  "combined": true,
  "clients": [
    {
      "client_id": "subscriber-1",
      "peer": "10.255.0.2:43210",
      "report": { ... per-client TestReport ... }
    },
    {
      "client_id": "subscriber-2",
      "peer": "10.255.1.2:43211",
      "report": { ... per-client TestReport ... }
    }
  ]
}
```

Text output:
```
bngtester COMBINED — 2 clients
══════════════════════════════════════════════════

--- subscriber-1 (10.255.0.2:43210) ---
  Stream 0 [udp_latency ↑ DSCP=EF] 100pps
    Latency: ...

--- subscriber-2 (10.255.1.2:43211) ---
  Stream 0 [udp_latency ↑ DSCP=AF41] 100pps
    Latency: ...
```

JUnit output: each client becomes a separate `<testsuite>`.

### Server Lifecycle

The server runs indefinitely, accepting clients. Report output options:

1. **Per-session** (default): Each session writes its own report to stdout/file as it completes. This is the current behavior, preserved for single-client use.
2. **Combined** (`--combined`): Server waits for all sessions to complete (or a `--max-clients N` threshold), then writes a single combined report.
3. **Signal-triggered**: `SIGUSR1` causes the server to write a snapshot report of all completed sessions so far.

For the initial implementation, option 1 (per-session) is the default and option 2 (combined with `--max-clients`) is added.

### Resource Isolation

Each session is fully isolated:
- Separate UDP socket (different port)
- Separate metrics collectors
- Separate ECN counters
- Separate heartbeat tracker
- Session-scoped CancellationToken (one session failing doesn't kill others)

The only shared state is the `SessionRegistry` (append-only, behind `Arc<Mutex>`).

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--client-id <NAME>` | _(source IP)_ | Client identifier (on client CLI) |
| `--max-clients <N>` | _(unlimited)_ | Server: wait for N clients to complete, then write combined report |
| `--combined` | _(off)_ | Server: output combined report instead of per-session |

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/protocol/mod.rs` | Modify | Add `client_id` to `HelloMsg` |
| `src/bin/server.rs` | Modify | Spawn concurrent sessions, SessionRegistry, combined report output |
| `src/bin/client.rs` | Modify | Add `--client-id` CLI flag, send in hello |
| `src/report/mod.rs` | Modify | Add `CombinedReport`, `ClientReport` structs |
| `src/report/json.rs` | Modify | Combined JSON output |
| `src/report/text.rs` | Modify | Combined text output |
| `src/report/junit.rs` | Modify | Combined JUnit with per-client testsuites |

## Implementation Order

1. Protocol — `client_id` in HelloMsg
2. Client CLI — `--client-id` flag
3. Server concurrency — spawn sessions as tasks, share CLI/thresholds via Arc
4. SessionRegistry — collect completed session reports
5. Combined report — `--combined` and `--max-clients` flags, combined output
6. Report formatters — combined JSON, text, JUnit

## Testing

- [ ] Server accepts 2+ concurrent client connections
- [ ] Each client gets its own UDP data port
- [ ] Per-client metrics tracked independently (no cross-contamination)
- [ ] `client_id` in hello message and report
- [ ] Default client_id = source IP when not specified
- [ ] Combined JSON report contains both clients' results
- [ ] Combined text report shows per-client sections
- [ ] `--max-clients 2` causes server to exit after 2 sessions complete
- [ ] Single client still works (backward compatible)
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] End-to-end: 2 subscribers through BNG simultaneously

## Not In Scope

- Coordinated test start across subscribers (each starts independently)
- Cross-subscriber metrics (fairness index — future enhancement)
- Orchestration of multiple client invocations (test runner's job)
- Server-initiated client discovery
