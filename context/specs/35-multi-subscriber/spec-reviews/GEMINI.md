# Spec Review: Multi-Subscriber Coordination (Gemini)

Review of `context/specs/35-multi-subscriber/IMPLEMENTATION_SPEC.md` focusing on concurrency, resource management, and reporting logic.

## Findings

### 1. SessionRegistry and Active Session Tracking
- **Severity: MEDIUM**
- **Observation:** The `SessionRegistry` is defined as a `Vec<CompletedSession>`. While this works for gathering results after sessions finish, it does not provide visibility into *currently active* sessions.
- **Impact:** The server cannot provide a snapshot of current progress (e.g., via `SIGUSR1`) if it only tracks completed sessions. It also makes it harder to implement a "wait for all" logic if the total number of clients is unknown.
- **Recommendation:** Expand `SessionRegistry` to track active sessions (e.g., using a `HashMap<ClientId, SessionHandle>`) so the server can report on both active and completed sessions.

### 2. Resource Cleanup and Socket Leaks
- **Severity: MEDIUM**
- **Observation:** The spec correctly identifies the need for session-scoped `CancellationToken`. However, it should explicitly state that the UDP socket and any per-session tasks (receiver, heartbeats) must be dropped/closed even if the session fails or is interrupted.
- **Impact:** Failure to properly drop the UDP socket or stop the receiver task could lead to port exhaustion or "stuck" server tasks in long-running multi-client scenarios.
- **Recommendation:** Ensure the `handle_session` implementation uses `tokio::select!` or `drop` guards to guarantee that the UDP socket and receiver task are cleaned up regardless of how the session exits (success, timeout, or error).

### 3. Combined Report Timing and Termination
- **Severity: MEDIUM**
- **Observation:** The spec proposes `--max-clients <N>` as the trigger for a combined report.
- **Impact:** If `N` clients are expected but one fails to connect or crashes before sending its hello, the server will wait indefinitely for the combined report.
- **Recommendation:** Add a `--timeout <SECS>` flag for the combined report mode. If the timeout is reached, the server should produce the combined report for all sessions completed so far and exit.

### 4. Client Identification Collisions
- **Severity: LOW**
- **Observation:** `client_id` defaults to the source IP.
- **Impact:** If multiple client containers share an IP (e.g., NAT or host networking), their reports will collide in the `SessionRegistry` or combined report if they don't provide a unique `--client-id`.
- **Recommendation:** The server should append a unique suffix or the source port to the `client_id` if it detects a duplicate, or at least log a warning.

### 5. Backward Compatibility (Single-Client Use)
- **Severity: LOW**
- **Observation:** The spec states that per-session reporting is preserved for single-client use.
- **Impact:** Minimal risk, but ensure that the default output remains identical to the current version.
- **Recommendation:** Verify that without `--combined` or `--max-clients`, the server exits (or continues to the next session) exactly as it does today, producing a single report per session to stdout.

### 6. Failed Session Reporting in Combined Output
- **Severity: LOW**
- **Observation:** The spec doesn't explicitly mention how failed/interrupted sessions appear in the combined report.
- **Impact:** A combined report should show that a client *started* a test but failed, rather than just omitting it.
- **Recommendation:** Include `Interrupted` or `Failed` sessions in the `clients` array of the combined report, preserving whatever partial metrics were collected.

## Summary of Recommendations

1. **Active Tracking:** Use a registry that tracks both active and completed sessions.
2. **Robust Cleanup:** Use `Drop` or explicit guards to ensure UDP sockets are closed on session exit.
3. **Report Timeout:** Add a timeout mechanism for `--max-clients` to avoid hanging.
4. **Collision Handling:** Handle or warn about duplicate `client_id`s.
5. **Partial Results:** Ensure the combined report includes clients that failed mid-test.
