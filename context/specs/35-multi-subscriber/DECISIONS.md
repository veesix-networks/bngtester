# Decisions: 35-multi-subscriber

## Accepted

### Track active + completed sessions in registry
- **Source:** GEMINI (G1)
- **Severity:** MEDIUM
- **Resolution:** SessionRegistry uses HashMap for active sessions and Vec for completed. Active sessions visible for timeout/snapshot scenarios. Failed sessions move to completed with partial metrics.

### Explicit resource cleanup on session exit
- **Source:** GEMINI (G2)
- **Severity:** MEDIUM
- **Resolution:** Session owns all resources (socket, receiver task, heartbeat) via Rust ownership. Session-scoped CancellationToken ensures child tasks are cancelled. Resources dropped when session exits regardless of exit path.

### Add --timeout for combined mode
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Resolution:** `--timeout <SECS>` produces combined report with whatever completed when time expires. Active sessions reported with status "active". Prevents hanging when a client fails to connect.

### Handle duplicate client_id
- **Source:** GEMINI (G4)
- **Severity:** LOW
- **Resolution:** Server appends numeric suffix to duplicate client_id and logs warning. Prevents report collisions.

### Single-client backward compatibility preserved
- **Source:** GEMINI (G5)
- **Severity:** LOW
- **Resolution:** Without --combined, server writes per-session reports as before. Sessions are spawned concurrently but writer lock prevents stdout interleaving.

### Failed sessions in combined report
- **Source:** GEMINI (G6)
- **Severity:** LOW
- **Resolution:** Failed/interrupted sessions push to completed with status Interrupted/Partial and whatever metrics were collected. Combined report includes them.

### Owned ServerConfig via Arc, not borrowed Cli
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** Extract needed config into owned `ServerConfig` struct (Clone + Send + 'static). Wrap in Arc, clone into each spawned task. handle_session takes Arc<ServerConfig> instead of &Cli + &Thresholds.

### JoinSet supervision for panicked/wedged sessions
- **Source:** CODEX (C2)
- **Severity:** HIGH
- **Resolution:** Use `JoinSet` instead of bare `tokio::spawn`. join_next() in the accept loop detects panicked/failed tasks. Panicked tasks recorded as failed sessions with no metrics. Combined with --timeout prevents indefinite hanging.

### Writer coordination for concurrent output
- **Source:** CODEX (C3)
- **Severity:** HIGH
- **Resolution:** Per-session stdout serialized via Arc<Mutex<()>> writer lock. --file produces per-client files ({base}-{client_id}.{ext}). Combined mode writes single report after all sessions complete.

### Remove SIGUSR1 from initial spec
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** SIGUSR1 snapshot removed from initial implementation scope. Listed in Not In Scope as future enhancement. Avoids signal-handling complexity in initial delivery.

### Separate write_combined_* report functions
- **Source:** CODEX (C5)
- **Severity:** MEDIUM
- **Resolution:** Added `write_combined_json()`, `write_combined_text()`, `write_combined_junit()` to file plan. Keeps existing TestReport-based functions unchanged. CombinedReport is a separate struct with a Vec of ClientReport entries.
