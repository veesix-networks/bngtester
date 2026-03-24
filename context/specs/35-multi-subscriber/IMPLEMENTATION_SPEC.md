# Implementation Spec: Multi-Subscriber Coordination (Concurrent Client Sessions)

## Overview

Enable bngtester-server to handle multiple concurrent client sessions, each from a different subscriber container. Each session gets its own control channel, data ports, and independently tracked metrics. The server produces a combined report with per-client breakdown. Clients are identified by a `client_id` in the hello message (defaulting to source IP + port).

## Source Issue

[#35 — Multi-subscriber coordination (concurrent client sessions)](https://github.com/veesix-networks/bngtester/issues/35)

## Current State

- The server accepts one client at a time — `handle_session()` is awaited sequentially in the accept loop.
- `handle_session()` borrows `&Cli` and `&Thresholds` — not spawn-ready.
- Each session writes its report directly to stdout or a file — concurrent writes would interleave.
- No client identification in the hello message.
- Metrics, report generation, and control protocol are all per-session.

## Design

### Server Config Ownership

Extract the needed config from `Cli` into an owned, `Clone + Send + 'static` struct:

```rust
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub output: String,
    pub file: Option<String>,
    pub raw_file: Option<String>,
    pub thresholds: Thresholds,
    pub histogram_buckets: Option<String>,
    pub combined: bool,
    pub max_clients: Option<usize>,
    pub timeout_secs: Option<u64>,
}
```

Wrap in `Arc<ServerConfig>` and clone into each spawned session task. `handle_session()` takes `Arc<ServerConfig>` instead of `&Cli` + `&Thresholds`.

### Server Concurrency Model

The accept loop spawns each session into a `JoinSet`:

```rust
let mut join_set = JoinSet::new();

loop {
    tokio::select! {
        result = listener.accept() => {
            let (stream, peer) = result?;
            let config = config.clone();
            let registry = registry.clone();
            join_set.spawn(async move {
                handle_session(stream, peer, config, registry).await
            });
        }
        Some(result) = join_set.join_next() => {
            // Handle completed/failed/panicked sessions
        }
    }
}
```

Using `JoinSet` provides supervision — panicked or failed tasks are detected via `join_next()`, preventing silent hangs in combined mode.

### Session Registry

Tracks both active and completed sessions:

```rust
pub struct SessionRegistry {
    inner: Arc<Mutex<RegistryInner>>,
}

struct RegistryInner {
    active: HashMap<String, ActiveSession>,
    completed: Vec<CompletedSession>,
}

pub struct ActiveSession {
    pub client_id: String,
    pub peer: SocketAddr,
    pub started_at: Instant,
}

pub struct CompletedSession {
    pub client_id: String,
    pub peer: SocketAddr,
    pub report: TestReport,
}
```

- Session registers as active on hello, moves to completed on finish.
- Failed/interrupted sessions push a report with `status: Interrupted` or `Partial`.
- Panicked tasks (detected via JoinSet) are recorded as failed with no metrics.

### Client Identification

Add `client_id` to `HelloMsg`:

```rust
pub struct HelloMsg {
    // ... existing fields ...
    pub client_id: Option<String>,
}
```

CLI flag: `--client-id <NAME>`. Default: `"{source_ip}:{source_port}"`. If the server detects a duplicate `client_id`, it appends a numeric suffix and logs a warning.

### Output Coordination

**Per-session mode (default, no `--combined`):**
- Each session writes its report immediately on completion.
- Stdout writes are serialized via `Arc<Mutex<()>>` writer lock to prevent interleaving.
- `--file` in per-session mode uses per-client filenames: `{base}-{client_id}.{ext}` (e.g., `report-subscriber1.json`). If no `--file`, stdout is used with the writer lock.

**Combined mode (`--combined`):**
- Server collects all completed sessions into the registry.
- Writes a single combined report after `--max-clients N` sessions complete, or after `--timeout` seconds, whichever comes first.
- If timeout is reached before N clients complete, the combined report includes whatever sessions finished plus entries for active sessions marked as `"status": "active"`.

### Combined Report Format

```json
{
  "combined": true,
  "total_clients": 2,
  "clients": [
    {
      "client_id": "subscriber-1",
      "peer": "10.255.0.2:43210",
      "report": { ... TestReport ... }
    },
    {
      "client_id": "subscriber-2",
      "peer": "10.255.1.2:43211",
      "report": { ... TestReport ... }
    }
  ]
}
```

Text:
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

JUnit: each client is a separate `<testsuite>` within `<testsuites>`.

### Resource Cleanup

Each session owns its resources (UDP socket, receiver task, heartbeat). When the session exits — whether success, error, or cancellation — all resources are dropped via Rust's ownership model. The session-scoped `CancellationToken` ensures child tasks (receiver, heartbeat) are cancelled when the session ends. One session failing does not affect others.

### Backward Compatibility

Without `--combined` or `--max-clients`, the server behaves exactly as before: accepts a session, handles it, writes its report, accepts the next. The only difference is sessions are now spawned concurrently (with the writer lock preventing stdout interleaving), so a second client connecting while the first is running will be handled simultaneously instead of queued.

## Configuration

| Flag | Where | Default | Description |
|------|-------|---------|-------------|
| `--client-id <NAME>` | Client | _(source IP:port)_ | Client identifier |
| `--combined` | Server | _(off)_ | Output combined report instead of per-session |
| `--max-clients <N>` | Server | _(unlimited)_ | Wait for N clients, then write combined report and exit |
| `--timeout <SECS>` | Server | _(none)_ | Combined mode timeout — write report with whatever completed |

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/protocol/mod.rs` | Modify | Add `client_id` to `HelloMsg` |
| `src/bin/server.rs` | Modify | `ServerConfig` struct, `Arc` ownership, `JoinSet` supervision, `SessionRegistry`, writer lock, combined mode, `--max-clients`, `--timeout` |
| `src/bin/client.rs` | Modify | Add `--client-id` CLI flag, send in hello |
| `src/report/mod.rs` | Modify | Add `CombinedReport`, `ClientReport` structs |
| `src/report/json.rs` | Modify | Add `write_combined_json()` |
| `src/report/text.rs` | Modify | Add `write_combined_text()` |
| `src/report/junit.rs` | Modify | Add `write_combined_junit()` |

## Implementation Order

1. Protocol — `client_id` in HelloMsg
2. Client — `--client-id` CLI flag
3. Server config — `ServerConfig` struct, `Arc` ownership, refactor `handle_session` signature
4. Concurrency — `JoinSet` accept loop, concurrent session spawning
5. SessionRegistry — active/completed tracking, duplicate client_id handling
6. Output coordination — writer lock for per-session stdout, per-client file naming
7. Combined mode — `--combined`, `--max-clients`, `--timeout` flags
8. Combined report formatters — `write_combined_json/text/junit()`

## Testing

- [ ] Server accepts 2 concurrent client connections
- [ ] Each client gets its own UDP data port
- [ ] Per-client metrics tracked independently (no cross-contamination)
- [ ] `client_id` in hello message and report
- [ ] Default client_id = source IP:port when not specified
- [ ] Duplicate client_id gets suffix appended
- [ ] Per-session stdout output does not interleave
- [ ] `--file` in per-session mode produces per-client files
- [ ] Combined JSON report contains both clients' results
- [ ] Combined text report shows per-client sections
- [ ] `--max-clients 2` causes server to exit after 2 sessions
- [ ] `--timeout` produces partial combined report if not all clients finish
- [ ] Failed session appears in combined report with status
- [ ] Single client without --combined works identically to before
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] 2 subscribers through BNG simultaneously

## Not In Scope

- Coordinated test start across subscribers (each starts independently)
- Cross-subscriber metrics (fairness index — future enhancement)
- Orchestration of multiple client invocations (test runner's job)
- Server-initiated client discovery
- SIGUSR1 snapshot reporting (future enhancement)
