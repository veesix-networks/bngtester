# Spec Review: Continuous/Resilient Mode (#49)

Review of the implementation specification for adding continuous/resilient mode for HA failover testing.

## Findings

### CRITICAL

#### 1. Server-side Session Fragmentation
The spec states that each reconnect is a new server session and the server's `multi-subscriber` support handles it.
- **Impact:** The current `bngtester-server` implementation appends a counter to duplicate `client_id` values (e.g., `client-1`, `client-2`). This results in fragmented reports where a single long-running failover test is split into multiple independent session reports.
- **Impact:** When the TCP control channel drops, the server cancels the UDP receiver loop for that session. Any data plane traffic arriving *between* the disconnect and the subsequent reconnect will be lost by the server, even if the BNG was successfully forwarding packets.
- **Recommendation:** The server should support a "session persistence" window. If a control channel drops, the UDP receiver should remain active for a configurable period (e.g., 30s). If a client reconnects with the same `client_id` within that window, it should be associated with the existing session metrics rather than starting a new one.

#### 2. Conflict with Combined Mode (`--combined`)
The server's combined mode waits for `max_clients` to complete before generating a report.
- **Impact:** Each reconnect from a resilient client will be counted as a *new* completed client session. A single client that reconnects 5 times will "consume" 5 slots in the `max_clients` count. This can trigger premature report generation and server shutdown before other subscribers have finished their tests.
- **Recommendation:** Combined mode logic must be updated to use the `client_id` for uniqueness. Reconnects from the same `client_id` should update the existing entry rather than incrementing the total client count.

### HIGH

#### 1. Static Outage Threshold (1 Second)
The spec proposes a fixed 1-second threshold for outage detection.
- **Impact:** At low packet rates (e.g., 0.1 pps or 0.5 pps), the normal interval between packets is greater than 1 second. This will cause the tool to report constant "outage" events during healthy operation.
- **Recommendation:** The outage threshold should be rate-aware. A suggested formula is `max(1.0, 3.0 / rate_pps)`. Alternatively, allow the user to configure the threshold via `--outage-threshold`.

### MEDIUM

#### 1. Resilient UDP Send Logic
The spec suggests "log and continue" on UDP send errors.
- **Finding:** This is the correct approach for latency and failover testing. Buffering packets during an outage and "bursting" them later would skew latency and jitter metrics and doesn't represent realistic subscriber behavior (where packets are simply dropped by the client's local stack if the interface is down).
- **Recommendation:** Ensure `send_errors` are explicitly tracked in the report so users can distinguish between "packet lost in network" and "packet failed to send locally."

#### 2. Control Plane vs. Data Plane Outage Cause
The spec lists "control_disconnect" and "packet_loss" as causes.
- **Concern:** If the control channel drops, the client currently "pauses" data streams. 
- **Recommendation:** The client should have an option to *continue* sending data plane traffic even if the control channel is down (relying on the server's persistent receiver recommended in CRITICAL #1). This is vital for measuring true data plane failover time independently of control plane (TCP) reconnect time.

### LOW

#### 1. Metric Correlation for "Final Report"
The spec mentions the client writes a final report covering the entire duration.
- **Recommendation:** The client will need to implement a "Session Merger" that can take the multiple `ResultsMsg` segments received from the server and produce a coherent time-series and aggregate report.

## Summary of Suggestions
1. **Implement Server-side Session Persistence:** Allow UDP receivers to survive control-plane flaps to capture true data-plane resilience.
2. **Make Outage Threshold Rate-Aware:** Use `max(1s, 3/rate)` to avoid false positives at low rates.
3. **Update Combined Mode:** Use `client_id` to prevent reconnects from being counted as multiple distinct clients.
4. **Distinguish Local vs. Network Errors:** Track `send_errors` separately from network loss in the final report.
