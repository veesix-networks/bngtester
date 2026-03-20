# Implementation Spec: Rust Collector — Server and Client Binaries

## Overview

Rust crate at the repo root producing two binaries: `bngtester-server` and `bngtester-client`. Dual-channel architecture (control + data) supporting multiple concurrent streams for throughput, latency, and bufferbloat (RRUL) testing. Measures one-way latency (UDP), RTT (TCP via `TCP_INFO`), jitter, packet loss, reordering, and throughput with sub-microsecond precision. Per-second time-series, latency histograms, and per-packet export for deep analysis. Outputs JUnit XML (with configurable thresholds), JSON, JSONL, and human-readable text. Uses tokio async runtime for concurrent stream management.

## Source Issue

[#5 — Rust collector — server and client binaries for traffic generation and measurement](https://github.com/veesix-networks/bngtester/issues/5)

## Current State

- No Rust code exists. `Cargo.toml` and `src/` are planned in CLAUDE.md but not created.
- Subscriber images (Alpine, Debian, Ubuntu) exist with `iperf3` for basic throughput testing.
- SUMMARY.md notes: "bng-client will replace the shell entrypoint" — this spec builds the first piece of that.
- The Codebase State table still references a "Go collector" — this will be corrected.

## Design

### Architecture

```
[bngtester-client]                           [bngtester-server]
  (subscriber container)                       (far side of BNG)
       |                                            |
       |--- Control Channel (TCP) -----------------→|  coordination, config exchange,
       |                                            |  clock offset estimation,
       |                                            |  heartbeat, results exchange
       |                                            |
       |--- Data Stream 1 (UDP latency probes) ---→|  port from ready message
       |--- Data Stream 2 (TCP throughput) -------→|  port from ready message
       |←-- Data Stream 3 (UDP reverse path) ------|  client binds listener
       |←-- Data Stream 4 (TCP reverse path) ------|  client binds listener
       |                                            |
```

**Dual-channel design:**
- **Control channel** (TCP, always present): Client connects to server, negotiates test parameters (including port assignments for each stream), performs clock offset estimation, exchanges heartbeats during tests, and exchanges results at test end. The control channel is established first; data channels are started on command.
- **Data channels** (TCP or UDP, one per stream): Carry test traffic. Multiple concurrent streams enable RRUL testing (saturate + probe simultaneously). Each stream is bound to a specific port assigned by the server (upstream) or client (downstream).

**Concurrency model:** tokio async runtime. All streams, the control channel, heartbeats, per-second metric sampling, timers, and signal handling run as concurrent tasks within a single tokio runtime. This avoids thread-per-stream overhead and simplifies coordinated shutdown via `CancellationToken`.

### Packet Format

UDP data streams use a custom header prepended to each payload:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Magic (0x424E4754)                    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Version    |   Stream ID   |           Flags               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                       Sequence Number (u32)                   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Timestamp Seconds (u64)                      |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                  Timestamp Nanoseconds (u32)                  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                     Payload Length (u32)                       |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Padding ...                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **Magic:** `0x424E4754` ("BNGT") — identifies bngtester packets.
- **Version:** Protocol version (starts at `1`). Allows future header changes.
- **Stream ID:** Identifies which data stream this packet belongs to (0-255). Enables per-stream metrics in concurrent tests. UDP streams on a shared socket are demultiplexed by this field.
- **Flags:** Bitfield — `0x01` = last packet in stream (signals clean shutdown).
- **Sequence Number:** Monotonically increasing per stream. Used for loss and reordering detection. Wraps at `u32::MAX` — loss/reordering logic must handle wrap-around (a sequence gap > `u32::MAX / 2` is treated as a wrap, not a loss).
- **Timestamp:** `clock_gettime(CLOCK_MONOTONIC)` at send time — nanosecond precision. Uses `tv_sec` (u64) + `tv_nsec` (u32) matching libc `timespec` layout.
- **Payload Length:** Total packet size including header. Padding fills to requested size.

Header size: 32 bytes. Minimum packet size: 32 bytes (header only, no padding).

TCP data streams do not use this header — TCP metrics come from `TCP_INFO` socket option (RTT, retransmissions, congestion window) rather than embedded timestamps, since TCP's buffering and segmentation make per-packet timestamps unreliable.

### Clock Model

`CLOCK_MONOTONIC` is used for UDP one-way latency because:
1. Not affected by NTP adjustments (no jumps).
2. Containers on the same host share the kernel's monotonic clock (same epoch).
3. Sub-microsecond precision — necessary since osvbng (VPP-based) processes packets in single-digit microseconds.

**Clock offset estimation** for cross-host testing: The control channel runs a ping-pong sequence at test start (similar to NTP's algorithm) to estimate the clock offset between client and server. This offset is applied to one-way latency calculations. **One-way latency in cross-host mode is only as accurate as the symmetry of the path delay** — asymmetric paths will produce biased results. Sufficient for lab environments, not for production.

**Clock modes:**
- **`same-host`** (default): `CLOCK_MONOTONIC` timestamps used directly, no offset estimation. Latency results are authoritative.
- **`sync-estimated`**: Clock offset estimated via control channel ping-pong. Latency results are marked as "estimated" in reports. Selected automatically when `--cross-host` flag is used, or can be forced.
- If same-host mode is selected but endpoints are actually on different hosts, latency values will be meaningless. The report includes a `clock_mode` field so consumers know which mode was used.

**Same-host mode** (default): When both endpoints share a kernel (containerlab, Docker on same host), clock offset estimation is skipped and `CLOCK_MONOTONIC` timestamps are used directly. This is the primary use case.

### Test Modes

#### `throughput` — Link saturation

Single or multiple TCP/UDP streams at maximum rate. Measures goodput (application-level bytes delivered — labeled as "L4 goodput" to distinguish from L2/L3 throughput), raw throughput, and for TCP: retransmissions, RTT, and congestion window via `TCP_INFO`.

#### `latency` — Precision delay measurement

Low-rate UDP probes (default 100 pps) with embedded timestamps. Measures one-way latency, jitter (RFC 3550), packet loss, and reordering. This is the baseline measurement — run unloaded to establish reference latency.

#### `rrul` — Realtime Response Under Load (bufferbloat detection)

Runs concurrently:
1. **4 TCP throughput streams** (2 upstream, 2 downstream) to saturate the link — staggered start with configurable ramp-up delay (default 100ms between streams) to avoid synchronized TCP slow-start masking BNG scheduling behavior
2. **1 UDP latency probe stream** (upstream) at low rate (100 pps)
3. **1 UDP latency probe stream** (downstream) at low rate (100 pps)

The test runs in two phases:
1. **Baseline** (first 5s, configurable): latency probes only, no throughput streams — establishes unloaded latency.
2. **Loaded** (remaining duration): throughput streams start (staggered), latency probes continue — measures latency under load.

**Bufferbloat metric:** `loaded_p99 / baseline_p99`. A ratio near 1.0 means no bufferbloat. A ratio > 5x indicates significant bufferbloat. This directly validates BNG queue management (CoDel/FQ-CoDel, traffic shaping).

#### `bidirectional` — Asymmetric path testing

Simultaneous upstream (client→server) and downstream (server→client) streams. Catches asymmetric latency, loss, or throughput — common in BNG deployments with different upstream/downstream rate limits.

### Stream Binding Model

Concurrent streams need explicit port/socket assignments. The binding model differs by protocol:

**UDP streams:** All upstream UDP streams on the server share a single UDP socket. Packets are demultiplexed by the `stream_id` field in the packet header. Downstream UDP streams use a single socket on the client side, also demultiplexed by `stream_id`.

**TCP streams:** Each TCP stream gets its own port. The server allocates ports dynamically (binding to port 0, letting the OS assign) and reports the port assignments in the `ready` control message. For reverse-path TCP streams, the client binds listener ports and reports them in the `start` message.

**`ready` message includes:**
```json
{
  "type": "ready",
  "udp_port": 5001,
  "tcp_ports": {
    "0": 5002,
    "1": 5003
  }
}
```

**`start` message includes (for reverse-path streams):**
```json
{
  "type": "start",
  "client_udp_port": 6001,
  "client_tcp_ports": {
    "2": 6002,
    "3": 6003
  }
}
```

**NAT/firewall assumption:** Reverse-path streams require the client to be reachable from the server. This is the expected case in containerlab/lab environments where both sides have direct L3 connectivity. NAT traversal is out of scope.

### Session State Machine

The control channel follows this state machine. Both client and server track session state independently.

```
INIT ──hello──→ NEGOTIATING ──ready──→ SYNCING ──clock_sync──→ READY
                                                                 │
                                                          ──start──→ RUNNING
                                                                       │
                                                               ──stop──→ COLLECTING
                                                                           │
                                                                  ──results──→ DONE
```

**Failure states:**

| Failure | Detection | Behavior |
|---------|-----------|----------|
| Control channel drops during RUNNING | Heartbeat timeout (3 missed = 15s) | Both sides stop data streams, server writes partial results with `"status": "interrupted"`, client exits with error code |
| Stream fails to connect | TCP connect timeout (5s) | Session continues degraded — failed stream is marked `"status": "failed"` in results. Test is not aborted. RRUL with 3/4 TCP streams is still useful. |
| Stream ends early | EOF or error on data socket | Stream marked `"status": "early_exit"` in results. Other streams continue. |
| Clock offset estimation fails | Ping-pong timeout or unreasonable offset (>1s) | Fall back to `sync-estimated` with warning, or abort if `--strict-clock` is set |
| SIGINT/SIGTERM during test | Signal handler | Client sends `stop`, waits up to 5s for `results`, writes what it has. Server writes partial results on control channel close. |

**Partial results:** When a session ends abnormally, both sides write whatever metrics they collected. Reports include a top-level `"status"` field: `"complete"`, `"interrupted"`, or `"partial"`. JUnit output marks the overall test suite as having an error.

### Traffic Patterns

| Pattern | Description | Available In |
|---------|-------------|-------------|
| `fixed` | Fixed packet size (configurable) | All modes |
| `imix` | Internet Mix: 7:4:1 ratio of 64:594:1518 byte packets | throughput, rrul |
| `sweep` | Incrementing sizes from 64 to 1518 in steps | latency |

### Measurement

#### UDP Metrics (per-stream)

- **One-way latency:** `server_recv_time - client_send_time` (adjusted by clock offset if cross-host). Reported as min/avg/max/p50/p95/p99/p999. Sequence wrap-around at `u32::MAX` is handled — gaps larger than `u32::MAX / 2` are treated as wraps, not losses.
- **Jitter:** RFC 3550 inter-packet delay variation. Running exponential average.
- **Packet loss:** `(max_seq - received_count) / max_seq * 100`. Sequence gaps detected from monotonic sequence numbers with wrap-around awareness.
- **Packet reordering:** Count and percentage of out-of-order packets (sequence number less than highest seen, accounting for wrap-around). Important for VPP multi-worker setups where packets may be processed on different cores.
- **Throughput:** Bytes received / time. Reported as bits/sec and packets/sec.

#### TCP Metrics (per-stream, from `TCP_INFO`)

Uses Linux `TCP_INFO` socket option. Required fields: `tcpi_rtt`, `tcpi_rttvar`, `tcpi_total_retrans`, `tcpi_snd_cwnd`. If the kernel returns a shorter `tcp_info` struct than expected, missing fields are reported as `null` rather than failing.

- **RTT:** Smoothed RTT (`tcpi_rtt`) and RTT variance (`tcpi_rttvar`) from the kernel's TCP stack. Polled every 100ms during the test.
- **Retransmissions:** Total retransmit count (`tcpi_total_retrans`) — indicates congestion or packet loss.
- **Congestion window:** `tcpi_snd_cwnd` evolution over time — shows how TCP responds to BNG shaping/policing.
- **Goodput:** Application-level bytes delivered / time (L4 goodput — excludes retransmissions and TCP/IP headers).

**Portability note:** `TCP_INFO` is Linux-specific (available since Linux 2.4, supported by both musl and glibc). This is acceptable for the three current subscriber images (all Linux). Non-Linux platforms are out of scope.

#### Bufferbloat Metrics (RRUL mode only)

- **Baseline latency:** p50/p95/p99 from the unloaded phase.
- **Loaded latency:** p50/p95/p99 from the loaded phase.
- **Bloat ratio:** `loaded_p99 / baseline_p99` — the key bufferbloat indicator.
- **Per-second latency time-series:** Shows latency progression as throughput ramps up — reveals how quickly buffers fill and whether AQM (CoDel etc.) activates.

#### Time-Series Data

All metrics are collected **per second** in addition to aggregate stats. This produces a time-series array in the output:

```json
"time_series": [
  {"t": 0, "latency_p99_us": 45.2, "throughput_mbps": 0, "loss_pct": 0},
  {"t": 1, "latency_p99_us": 48.1, "throughput_mbps": 940, "loss_pct": 0},
  {"t": 2, "latency_p99_us": 2340.5, "throughput_mbps": 942, "loss_pct": 0.01},
  ...
]
```

This is critical for BNG analysis — it shows latency spikes, QoS policer kick-in points, and AQM behavior over time.

#### Latency Histogram

Bucketed latency distribution (default: 10us buckets up to 1ms, then 100us buckets up to 10ms, then 1ms buckets above). Configurable via `--histogram-buckets` CLI flag. Enables visualization of latency distribution shape — bimodal distributions indicate queue scheduling issues.

### Report Formats

Both `bngtester-server` and `bngtester-client` can produce reports. The server has direct access to received-packet metrics. The client receives server metrics via the `results` control message and merges them with its own send-side metrics to produce a complete report. **The client is the primary report producer** since it is the CI-invoked entrypoint in subscriber containers.

#### JSON — Full structured results

```json
{
  "status": "complete",
  "clock_mode": "same-host",
  "test": {
    "mode": "rrul",
    "duration_secs": 30,
    "client": "10.0.0.2",
    "server": "10.0.0.1:5000"
  },
  "streams": [
    {
      "id": 0,
      "type": "udp_latency",
      "direction": "upstream",
      "status": "complete",
      "results": {
        "packets_sent": 3000,
        "packets_received": 2998,
        "packets_lost": 2,
        "loss_percent": 0.067,
        "packets_reordered": 0,
        "reorder_percent": 0.0,
        "latency_us": {
          "min": 12.4, "avg": 45.2, "max": 8312.8,
          "p50": 38.1, "p95": 89.4, "p99": 201.3, "p999": 4102.1
        },
        "jitter_us": 8.7,
        "throughput_bps": 2400000,
        "throughput_pps": 100
      }
    },
    {
      "id": 1,
      "type": "tcp_throughput",
      "direction": "upstream",
      "status": "complete",
      "results": {
        "goodput_bps": 943200000,
        "rtt_us": { "min": 120, "avg": 450, "max": 12000 },
        "retransmissions": 14,
        "cwnd_max": 65536
      }
    }
  ],
  "bufferbloat": {
    "baseline_p99_us": 45.2,
    "loaded_p99_us": 201.3,
    "bloat_ratio": 4.45
  },
  "time_series": [ ... ],
  "histogram": {
    "bucket_us": [10, 20, 30, 40, 50, 100, 200, 500, 1000, 5000, 10000],
    "counts":    [12, 45, 89, 120, 98, 210, 340, 80, 12, 3, 1]
  }
}
```

#### JUnit XML — CI integration with configurable thresholds

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="bngtester.rrul" tests="5" failures="1">
    <testcase name="packet_loss" classname="bngtester.rrul.upstream">
      <system-out>loss=0.067% (threshold: &lt;1%)</system-out>
    </testcase>
    <testcase name="latency_p99" classname="bngtester.rrul.upstream">
      <system-out>p99=201.3us (threshold: &lt;1000us)</system-out>
    </testcase>
    <testcase name="bufferbloat" classname="bngtester.rrul">
      <failure message="bloat_ratio=4.45 exceeds threshold 3.0">
        Loaded p99 (201.3us) is 4.45x baseline p99 (45.2us).
        Indicates significant bufferbloat — check BNG AQM configuration.
      </failure>
    </testcase>
    <testcase name="jitter" classname="bngtester.rrul.upstream">
      <system-out>jitter=8.7us (threshold: &lt;100us)</system-out>
    </testcase>
    <testcase name="throughput" classname="bngtester.rrul.upstream">
      <system-out>goodput=943.2Mbps (threshold: &gt;900Mbps)</system-out>
    </testcase>
  </testsuite>
</testsuites>
```

Thresholds are configured via CLI flags (e.g., `--threshold loss=1 --threshold p99=1000 --threshold bloat=3.0`). When a threshold is exceeded, the JUnit test case is marked as a `<failure>`.

#### JSONL — Per-packet raw data export

One JSON object per line, per received packet:

```jsonl
{"stream":0,"seq":1,"send_ts_ns":1234567890000,"recv_ts_ns":1234567890450,"size":64,"latency_ns":450}
{"stream":0,"seq":2,"send_ts_ns":1234567900000,"recv_ts_ns":1234567900480,"size":64,"latency_ns":480}
```

This enables external analysis — load into pandas, plot with matplotlib, feed into custom dashboards. Written to a file via `--raw-file <PATH>`.

#### Text — Human-readable summary

```
bngtester RRUL test — 30s duration
═══════════════════════════════════

Status: complete | Clock: same-host

Bufferbloat: 4.45x (baseline p99: 45.2µs → loaded p99: 201.3µs)

  Stream 0 [UDP latency ↑] 100pps
    Latency:  min=12.4µs avg=45.2µs max=8312.8µs p99=201.3µs
    Jitter:   8.7µs
    Loss:     0.067% (2/2998)
    Reorder:  0.0%

  Stream 1 [TCP throughput ↑]
    Goodput:  943.2 Mbps
    RTT:      avg=450µs max=12000µs
    Retrans:  14

  Stream 2 [TCP throughput ↓]
    Goodput:  941.8 Mbps
    ...
```

### CLI Interface

#### bngtester-server

```
bngtester-server [OPTIONS]

OPTIONS:
  -l, --listen <ADDR>           Listen address [default: 0.0.0.0:5000]
  -o, --output <FORMAT>         Output format: json, junit, text [default: text]
  -f, --file <PATH>             Write report to file (default: stdout)
      --raw-file <PATH>         Write per-packet JSONL data to file
      --threshold <KEY=VAL>     JUnit pass/fail threshold (repeatable)
                                Keys: loss, p50, p95, p99, p999, jitter,
                                      throughput, bloat
      --histogram-buckets <SPEC> Latency histogram bucket specification
```

#### bngtester-client

```
bngtester-client [OPTIONS] <SERVER>

ARGS:
  <SERVER>                      Server address (host:port)

OPTIONS:
  -m, --mode <MODE>             Test mode: throughput, latency, rrul, bidirectional
                                [default: latency]
  -p, --protocol <PROTO>        Protocol for throughput: tcp, udp [default: tcp]
  -s, --size <BYTES>            Packet size in bytes [default: 512]
  -r, --rate <PPS>              Latency probe rate, packets/sec [default: 100]
  -d, --duration <SECS>         Test duration in seconds [default: 30]
  -P, --pattern <PATTERN>       Traffic pattern: fixed, imix, sweep [default: fixed]
      --rrul-baseline <SECS>    RRUL baseline phase duration [default: 5]
      --rrul-ramp-up <MS>       Delay between TCP stream starts in RRUL [default: 100]
      --streams <N>             Number of throughput streams per direction [default: 2]
      --bidir                   Run bidirectional (alias for --mode bidirectional)
      --cross-host              Use clock offset estimation (for cross-host testing)
      --strict-clock            Abort if clock sync quality is poor
  -o, --output <FORMAT>         Output format: json, junit, text [default: text]
  -f, --file <PATH>             Write report to file (default: stdout)
      --raw-file <PATH>         Write per-packet JSONL data to file
      --threshold <KEY=VAL>     JUnit pass/fail threshold (repeatable)
      --histogram-buckets <SPEC> Latency histogram bucket specification
```

### Control Protocol

The control channel uses a simple length-prefixed JSON message protocol over TCP:

```
[4 bytes: message length (u32 big-endian)][JSON payload]
```

Message types:

| Type | Direction | Purpose |
|------|-----------|---------|
| `hello` | client→server | Client sends test configuration (mode, streams, duration, patterns) |
| `ready` | server→client | Server confirms ready + port assignments for each upstream stream |
| `clock_sync` | bidirectional | Clock offset estimation ping-pong (skipped in same-host mode) |
| `start` | client→server | Begin data streams + client port assignments for downstream streams |
| `heartbeat` | bidirectional | Keepalive during test — sent every 5s, 3 missed = session timeout |
| `stop` | client→server | End test, request results |
| `results` | server→client | Server sends its metrics |
| `error` | bidirectional | Report a fatal error with reason string |

**Heartbeat:** During RUNNING state, both sides send `heartbeat` messages every 5 seconds. If 3 consecutive heartbeats are missed (15s), the control channel is considered dead and the session transitions to the failure path (see Session State Machine). This prevents silent hangs when the BNG or link saturates the control channel's path.

### Dockerfile Integration

The client binary must be available in all subscriber images. Approach: **multi-stage build** with a shared Rust builder stage.

```dockerfile
FROM rust:1.85-alpine AS builder
WORKDIR /build
# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    apk add --no-cache musl-dev && \
    cargo build --release 2>/dev/null || true
# Now build the real binary
COPY src/ src/
RUN cargo build --release --bin bngtester-client && \
    strip target/release/bngtester-client

FROM alpine:3.21
# ... existing packages ...
COPY --from=builder /build/target/release/bngtester-client /usr/local/bin/bngtester-client
```

Using `rust:1.85-alpine` + musl produces a static binary that works on all three distros (Alpine, Debian, Ubuntu). The crate uses `jemallocator` as the global allocator to avoid musl's default allocator performance issues at high packet rates.

The build context must change from `images/` to the repo root so Dockerfiles can access `Cargo.toml` and `src/`. The CI workflow (`publish-images.yml`) will need its build context updated. A `.dockerignore` at the repo root must be added to exclude `.git/`, `context/`, and other non-build inputs from the Docker build context.

**Only the client binary goes into subscriber images.** The server binary is built separately and runs on the far side of the BNG — it is not part of the subscriber containers.

## Configuration

No environment variables for the Rust crate itself. The binaries use CLI arguments only.

Subscriber images gain `bngtester-client` at `/usr/local/bin/bngtester-client` — no config needed, it's invoked by the test runner.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Create | Crate manifest with two binary targets, jemallocator dep |
| `Cargo.lock` | Create | Dependency lockfile (committed for binaries) |
| `.dockerignore` | Create | Exclude `.git/`, `context/`, `target/` from Docker build context |
| `src/lib.rs` | Create | Shared library: re-exports modules |
| `src/bin/server.rs` | Create | Server binary: control channel, listeners, orchestration |
| `src/bin/client.rs` | Create | Client binary: control channel, stream orchestration, report output |
| `src/protocol/mod.rs` | Create | Control protocol: message types, serialization |
| `src/protocol/clock.rs` | Create | Clock offset estimation (ping-pong) |
| `src/protocol/session.rs` | Create | Session state machine, failure states, heartbeat |
| `src/traffic/mod.rs` | Create | Traffic module: packet format, generators |
| `src/traffic/packet.rs` | Create | Packet header struct, serialize/deserialize, wrap-around handling |
| `src/traffic/generator.rs` | Create | UDP stream generators with rate control |
| `src/traffic/tcp.rs` | Create | TCP stream generator and `TCP_INFO` reader |
| `src/traffic/receiver.rs` | Create | Downstream stream receiver (client-side listener for reverse path) |
| `src/stream/mod.rs` | Create | Stream registry: maps stream IDs to ports, tracks state per stream |
| `src/metrics/mod.rs` | Create | Metrics module: collectors and aggregation |
| `src/metrics/latency.rs` | Create | Latency stats, histogram, percentiles |
| `src/metrics/loss.rs` | Create | Loss and reordering detection with wrap-around |
| `src/metrics/throughput.rs` | Create | Throughput and goodput calculation |
| `src/metrics/jitter.rs` | Create | RFC 3550 jitter computation |
| `src/metrics/timeseries.rs` | Create | Per-second metric bucketing |
| `src/report/mod.rs` | Create | Report module: output formatters |
| `src/report/json.rs` | Create | JSON report formatter |
| `src/report/junit.rs` | Create | JUnit XML with threshold-based pass/fail |
| `src/report/jsonl.rs` | Create | Per-packet JSONL raw data export |
| `src/report/text.rs` | Create | Human-readable text output |
| `images/alpine/Dockerfile` | Modify | Add multi-stage Rust builder, copy bngtester-client |
| `images/debian/Dockerfile` | Modify | Add multi-stage Rust builder, copy bngtester-client |
| `images/ubuntu/Dockerfile` | Modify | Add multi-stage Rust builder, copy bngtester-client |
| `.github/workflows/publish-images.yml` | Modify | Update build context from `images/` to repo root |

## Implementation Order

### Phase A: Crate scaffold, packet format, and control protocol
- `Cargo.toml` with dependencies (`tokio`, `clap`, `serde`, `serde_json`, `quick-xml`, `libc`, `jemallocator`, `tokio-util` for `CancellationToken`)
- Packet header struct with serialize/deserialize (big-endian), wrap-around handling
- Control protocol message types, serialization, session state machine
- Clock offset estimation
- Heartbeat task
- Unit tests for packet round-trip, wrap-around, control messages, state transitions

### Phase B: Metrics collection
- Latency stats with histogram and percentile computation
- RFC 3550 jitter
- Sequence-based loss and reordering detection with wrap-around
- Throughput calculation
- Per-second time-series bucketing
- Unit tests for each metric (including wrap-around edge cases)

### Phase C: Traffic generators and stream management
- Stream registry: maps stream IDs to ports, tracks per-stream state
- UDP stream generator with configurable rate, size, IMIX, sweep patterns
- TCP stream generator with `TCP_INFO` polling (pin required fields, handle shorter structs)
- Downstream receiver (client-side listener for reverse-path streams)
- Timestamp embedding via `clock_gettime(CLOCK_MONOTONIC)`

### Phase D: Report output
- JSON structured output (full results with time-series, histogram, status, clock_mode)
- JUnit XML with configurable thresholds
- JSONL per-packet raw data
- Human-readable text
- Unit tests for report formatting

### Phase E: Server binary
- CLI parsing with `clap`
- Control channel listener (TCP) with session state machine
- Port allocation and `ready` message with port assignments
- UDP and TCP data listeners with stream-ID demuxing
- Heartbeat sender/receiver
- Stream-aware metrics collection
- Partial result handling on abnormal termination
- Report generation at test end

### Phase F: Client binary
- CLI parsing with `clap` (including output/threshold flags)
- Control channel connection and test negotiation
- Downstream listener binding for reverse-path streams
- Test mode orchestration (throughput, latency, rrul, bidirectional)
- RRUL: baseline phase → staggered TCP ramp-up → loaded phase
- Result merging from server `results` message + client-side metrics
- Report writing (client is the primary report producer)
- Clean shutdown on duration expiry or SIGINT/SIGTERM with partial result output

### Phase G: Dockerfile integration
- Add `.dockerignore` at repo root
- Update all three Dockerfiles with multi-stage builder (dep caching + real build)
- Update build context from `images/` to repo root in Dockerfiles
- Update `publish-images.yml` build context path
- Verify images build successfully

## Testing

- [ ] All new files have SPDX copyright headers
- [ ] `cargo build --release` succeeds with no warnings
- [ ] `cargo test` passes all unit tests
- [ ] Packet header serialization/deserialization round-trips correctly
- [ ] Sequence number wrap-around handled correctly in loss/reorder detection
- [ ] Control protocol message exchange works (hello → ready → start → stop → results)
- [ ] Heartbeat keepalive works — 3 missed heartbeats triggers session timeout
- [ ] Clock offset estimation produces reasonable values on same host
- [ ] Session state machine transitions correctly on stream failure (degraded, not aborted)
- [ ] UDP latency stream: client sends at configured rate, server measures latency
- [ ] TCP throughput stream: goodput measured, `TCP_INFO` metrics collected
- [ ] `TCP_INFO` handles shorter-than-expected struct gracefully
- [ ] IMIX pattern produces correct 7:4:1 size distribution
- [ ] RRUL mode: baseline phase runs without throughput, TCP streams start staggered
- [ ] Bidirectional mode: server→client stream works via client-side listener
- [ ] Per-second time-series data is collected
- [ ] Latency histogram buckets are correct and configurable
- [ ] Loss detection: dropped packets are counted correctly
- [ ] Reordering detection: out-of-order packets are flagged
- [ ] JSON output is valid, contains `status` and `clock_mode` fields
- [ ] JUnit XML with threshold: exceeded threshold produces `<failure>`
- [ ] JSONL output has one valid JSON object per line per packet
- [ ] Text output is human-readable
- [ ] Client produces merged report with server-side metrics
- [ ] Partial results written on SIGINT/SIGTERM
- [ ] Client binary runs in Alpine container
- [ ] Client binary runs in Debian container
- [ ] Client binary runs in Ubuntu container
- [ ] `.dockerignore` excludes `.git/`, `context/`, `target/`
- [ ] End-to-end: client → BNG → server with metrics collected

## Not In Scope

- Kernel bypass (AF_XDP, DPDK)
- Encryption or authentication of measurement traffic
- GUI or web dashboard
- Historical data storage
- PTP clock synchronization (cross-host one-way latency uses estimated offset)
- Non-Linux platforms (`TCP_INFO` is Linux-specific)
- NAT traversal for reverse-path streams
