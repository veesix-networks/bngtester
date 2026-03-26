# Implementation Spec: Continuous/Resilient Mode with Reconnect and Failover Metrics

## Overview

Add continuous operating mode for long-running and HA failover testing. `--duration 0` or `--continuous` runs indefinitely until SIGINT. UDP streams are resilient to packet loss (never stop on loss). The TCP control channel reconnects on disconnect with backoff. Failover-specific metrics (time-to-recovery, outage events) are tracked in time-series data.

## Source Issue

[#49 — Continuous/resilient mode with reconnect and failover metrics](https://github.com/veesix-networks/bngtester/issues/49)

## Current State

- Generator loop checks `start.elapsed() >= config.duration` to stop. Duration 0 would stop immediately.
- UDP send errors cause `return Err(e)` — any send failure terminates the generator.
- TCP control channel has no reconnect — disconnect ends the session.
- No outage event tracking in time-series or reports.
- SIGINT triggers `CancellationToken` which cleanly stops everything.

## Design

### Continuous Mode Activation

Two ways to activate:
- `--duration 0` — duration zero means indefinite
- `--continuous` — explicit flag (alias for duration 0)

Config file: `duration: 0` or `continuous: true`.

### Generator Duration Check

Change the duration check from:
```rust
if start.elapsed() >= config.duration { break; }
```
To:
```rust
if !config.duration.is_zero() && start.elapsed() >= config.duration { break; }
```

`Duration::ZERO` means run forever — only `CancellationToken` (from SIGINT) stops it.

### UDP Resilience

Currently, send errors terminate the generator. For continuous mode, UDP send errors should be logged and counted, not fatal:

```rust
match socket.send(&pkt).await {
    Ok(n) => { packets_sent += 1; bytes_sent += n as u64; }
    Err(_) if cancel.is_cancelled() => break,
    Err(e) => {
        send_errors += 1;
        // In continuous mode: log and continue. In fixed mode: return Err.
        if config.duration.is_zero() {
            continue;
        }
        return Err(e);
    }
}
```

The server receiver loop already handles packet gaps via the loss tracker — no changes needed on the receiver side.

### TCP Control Channel Reconnect

When the control channel TCP drops during a continuous test:

1. Client detects disconnect (read returns None or error)
2. Client pauses data streams (cancel token for current generator)
3. Client attempts reconnect with exponential backoff (1s, 2s, 4s, 8s, max 30s)
4. On successful reconnect: send new Hello, receive Ready, resume data streams
5. Reconnect events logged and recorded in report

**Reconnect limit:** `--max-reconnects <N>` (default 10). After N failed reconnects, the test ends.

Server side: each reconnect is a new session. The server's multi-subscriber support already handles new connections. The client's `--client-id` ensures the server can correlate reconnected sessions.

### Outage Events

Track periods where no packets were successfully sent or received:

```rust
pub struct OutageEvent {
    pub start_secs: f64,      // seconds since test start
    pub end_secs: f64,        // when traffic resumed
    pub duration_secs: f64,   // end - start
    pub packets_lost: u64,    // estimated packets lost during outage
    pub cause: String,        // "packet_loss", "control_disconnect", "send_error"
}
```

Detection: if no successful UDP send/receive for > 1 second, an outage event begins. It ends when the next successful packet is sent/received.

### Failover Metrics

Added to the report:

```rust
pub struct FailoverMetrics {
    pub outages: Vec<OutageEvent>,
    pub total_outage_secs: f64,
    pub max_outage_secs: f64,
    pub reconnects: u32,
    pub availability_percent: f64,  // (total_time - outage_time) / total_time * 100
}
```

Only present when outages occurred (skip_serializing_if).

### Report Output

Text:
```
Availability: 99.7% (2 outages, max 1.2s)
  Outage 1: t=45.2s → t=46.4s (1.2s, 120 packets lost, control_disconnect)
  Outage 2: t=89.1s → t=89.4s (0.3s, 30 packets lost, packet_loss)
```

JSON: `failover` object with outages array.

JUnit: `availability` test case with threshold support (`--threshold availability=99.9`).

### Clean Shutdown

SIGINT in continuous mode:
1. Cancel token fires
2. Generator stops sending (sends FLAG_LAST)
3. Client sends Stop on control channel
4. Server sends Results
5. Client writes final report covering entire run duration
6. Exit 0

### Config File

```yaml
continuous: true
# or
duration: 0

max_reconnects: 10
```

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `src/traffic/generator.rs` | Modify | Duration zero = indefinite, resilient send errors in continuous mode |
| `src/bin/client.rs` | Modify | `--continuous` flag, `--max-reconnects`, reconnect loop, outage tracking |
| `src/bin/server.rs` | Modify | No changes needed (multi-subscriber already handles reconnects) |
| `src/report/mod.rs` | Modify | `OutageEvent`, `FailoverMetrics`, `availability` threshold |
| `src/report/text.rs` | Modify | Availability line, outage event list |
| `src/report/json.rs` | Modify | Failover object |
| `src/report/junit.rs` | Modify | Availability test case |
| `src/config.rs` | Modify | Add `continuous` and `max_reconnects` to ClientConfig |
| `src/protocol/mod.rs` | Modify | Add `continuous` to HelloMsg for server awareness |

## Implementation Order

1. Generator — duration zero check, resilient send errors
2. Report structs — OutageEvent, FailoverMetrics, availability threshold
3. Client — `--continuous` flag, outage detection, reconnect loop
4. Report formatters — availability in text/JSON/JUnit
5. Config — continuous and max_reconnects in YAML

## Testing

- [ ] `--duration 0` runs until SIGINT
- [ ] `--continuous` runs until SIGINT
- [ ] Duration zero in config file works
- [ ] UDP send errors logged, not fatal in continuous mode
- [ ] UDP send errors still fatal in fixed-duration mode
- [ ] Generator stops only on cancel token when duration is zero
- [ ] Outage detected when no successful send for >1s
- [ ] Outage event recorded with start/end/duration/cause
- [ ] Availability percentage calculated correctly
- [ ] FailoverMetrics omitted when no outages (skip_serializing_if)
- [ ] `--threshold availability=99.9` in JUnit
- [ ] SIGINT produces clean final report
- [ ] `cargo test` passes all existing + new tests
- [ ] [MANUAL] Run continuous mode, kill server, restart server — client reconnects

## Not In Scope

- Triggering HA failover (BNG/orchestrator's job)
- Detecting failover type (MAC move vs admin switch)
- BNG-specific health checks
- Automatic test duration based on convergence detection
