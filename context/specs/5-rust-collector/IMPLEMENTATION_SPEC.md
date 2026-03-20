# Implementation Spec: Rust Collector — Server and Client Binaries

## Overview

Rust crate at the repo root producing two binaries: `bngtester-server` and `bngtester-client`. Dual-channel architecture (control + data) supporting multiple concurrent streams for throughput, latency, and bufferbloat (RRUL) testing. Measures one-way latency (UDP), RTT (TCP via `TCP_INFO`), jitter, packet loss, reordering, and throughput with sub-microsecond precision. Per-second time-series, latency histograms, and per-packet export for deep analysis. Outputs JUnit XML (with configurable thresholds), JSON, JSONL, and human-readable text.

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
       |                                            |  results exchange
       |                                            |
       |--- Data Stream 1 (UDP latency probes) ---→|
       |--- Data Stream 2 (TCP throughput) -------→|  concurrent streams
       |←-- Data Stream 3 (UDP reverse path) ------|  for RRUL/bidirectional
       |                                            |
```

**Dual-channel design:**
- **Control channel** (TCP, always present): Client connects to server, negotiates test parameters, performs clock offset estimation, and exchanges results at test end. The control channel is established first; data channels are started on command.
- **Data channels** (TCP or UDP, one per stream): Carry test traffic. Multiple concurrent streams enable RRUL testing (saturate + probe simultaneously).

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
- **Stream ID:** Identifies which data stream this packet belongs to (0-255). Enables per-stream metrics in concurrent tests.
- **Flags:** Bitfield — `0x01` = last packet in stream (signals clean shutdown).
- **Sequence Number:** Monotonically increasing per stream. Used for loss and reordering detection.
- **Timestamp:** `clock_gettime(CLOCK_MONOTONIC)` at send time — nanosecond precision.
- **Payload Length:** Total packet size including header. Padding fills to requested size.

Header size: 32 bytes. Minimum packet size: 32 bytes (header only, no padding).

TCP data streams do not use this header — TCP metrics come from `TCP_INFO` socket option (RTT, retransmissions, congestion window) rather than embedded timestamps, since TCP's buffering and segmentation make per-packet timestamps unreliable.

### Clock Model

`CLOCK_MONOTONIC` is used for UDP one-way latency because:
1. Not affected by NTP adjustments (no jumps).
2. Containers on the same host share the kernel's monotonic clock (same epoch).
3. Sub-microsecond precision — necessary since osvbng (VPP-based) processes packets in single-digit microseconds.

**Clock offset estimation** for cross-host testing: The control channel runs a simple ping-pong sequence at test start (similar to NTP's algorithm) to estimate the clock offset between client and server. This offset is applied to one-way latency calculations. Accuracy depends on symmetric path delay — sufficient for lab environments, not for production.

**Same-host mode** (default): When both endpoints share a kernel (containerlab, Docker on same host), clock offset estimation is skipped and `CLOCK_MONOTONIC` timestamps are used directly. This is the primary use case.

### Test Modes

#### `throughput` — Link saturation

Single or multiple TCP/UDP streams at maximum rate. Measures goodput (application-level bytes delivered), raw throughput, and for TCP: retransmissions, RTT, and congestion window via `TCP_INFO`.

#### `latency` — Precision delay measurement

Low-rate UDP probes (default 100 pps) with embedded timestamps. Measures one-way latency, jitter (RFC 3550), packet loss, and reordering. This is the baseline measurement — run unloaded to establish reference latency.

#### `rrul` — Realtime Response Under Load (bufferbloat detection)

Runs concurrently:
1. **4 TCP throughput streams** (2 upstream, 2 downstream) to saturate the link
2. **1 UDP latency probe stream** (upstream) at low rate (100 pps)
3. **1 UDP latency probe stream** (downstream) at low rate (100 pps)

The test runs in two phases:
1. **Baseline** (first 5s): latency probes only, no throughput streams — establishes unloaded latency.
2. **Loaded** (remaining duration): throughput streams start, latency probes continue — measures latency under load.

**Bufferbloat metric:** `loaded_p99 / baseline_p99`. A ratio near 1.0 means no bufferbloat. A ratio > 5x indicates significant bufferbloat. This directly validates BNG queue management (CoDel/FQ-CoDel, traffic shaping).

#### `bidirectional` — Asymmetric path testing

Simultaneous upstream (client→server) and downstream (server→client) streams. Catches asymmetric latency, loss, or throughput — common in BNG deployments with different upstream/downstream rate limits.

### Traffic Patterns

| Pattern | Description | Available In |
|---------|-------------|-------------|
| `fixed` | Fixed packet size (configurable) | All modes |
| `imix` | Internet Mix: 7:4:1 ratio of 64:594:1518 byte packets | throughput, rrul |
| `sweep` | Incrementing sizes from 64 to 1518 in steps | latency |

### Measurement

#### UDP Metrics (per-stream)

- **One-way latency:** `server_recv_time - client_send_time` (adjusted by clock offset if cross-host). Reported as min/avg/max/p50/p95/p99/p999.
- **Jitter:** RFC 3550 inter-packet delay variation. Running exponential average.
- **Packet loss:** `(max_seq - received_count) / max_seq * 100`. Sequence gaps detected from monotonic sequence numbers.
- **Packet reordering:** Count and percentage of out-of-order packets (sequence number less than highest seen). Important for VPP multi-worker setups where packets may be processed on different cores.
- **Throughput:** Bytes received / time. Reported as bits/sec and packets/sec.

#### TCP Metrics (per-stream, from `TCP_INFO`)

- **RTT:** Smoothed RTT and RTT variance from the kernel's TCP stack.
- **Retransmissions:** Total retransmit count — indicates congestion or packet loss.
- **Congestion window:** `cwnd` evolution over time — shows how TCP responds to BNG shaping/policing.
- **Goodput:** Application-level bytes delivered / time (excludes retransmissions and headers).

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

Bucketed latency distribution with configurable bucket widths (default: 10us buckets up to 1ms, then 100us buckets up to 10ms, then 1ms buckets above). Enables visualization of latency distribution shape — bimodal distributions indicate queue scheduling issues.

### Report Formats

#### JSON — Full structured results

```json
{
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
  -d, --duration <SECS>         Test duration in seconds [default: 10]
  -P, --pattern <PATTERN>       Traffic pattern: fixed, imix, sweep [default: fixed]
      --rrul-baseline <SECS>    RRUL baseline phase duration [default: 5]
      --streams <N>             Number of throughput streams [default: 2]
      --bidir                   Run bidirectional (alias for --mode bidirectional)
```

### Control Protocol

The control channel uses a simple length-prefixed JSON message protocol over TCP:

```
[4 bytes: message length (u32 big-endian)][JSON payload]
```

Message types:

| Type | Direction | Purpose |
|------|-----------|---------|
| `hello` | client→server | Client sends test configuration |
| `ready` | server→client | Server confirms ready to receive |
| `clock_sync` | bidirectional | Clock offset estimation (ping-pong) |
| `start` | client→server | Begin data streams |
| `stop` | client→server | End test, request results |
| `results` | server→client | Server sends its metrics |

This allows the server to know what to expect (which streams, how many, which protocols) before data arrives, and enables exchanging results so both sides can produce complete reports.

### Dockerfile Integration

The client binary must be available in all subscriber images. Approach: **multi-stage build** with a shared Rust builder stage.

```dockerfile
FROM rust:1.85-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN apk add --no-cache musl-dev && \
    cargo build --release --bin bngtester-client && \
    strip target/release/bngtester-client

FROM alpine:3.21
# ... existing packages ...
COPY --from=builder /build/target/release/bngtester-client /usr/local/bin/bngtester-client
```

Using `rust:1.85-alpine` + musl produces a static binary that works on all three distros (Alpine, Debian, Ubuntu).

The build context must change from `images/` to the repo root so Dockerfiles can access `Cargo.toml` and `src/`. The CI workflow (`publish-images.yml`) will need its build context updated.

**Only the client binary goes into subscriber images.** The server binary is built separately and runs on the far side of the BNG — it is not part of the subscriber containers.

## Configuration

No environment variables for the Rust crate itself. The binaries use CLI arguments only.

Subscriber images gain `bngtester-client` at `/usr/local/bin/bngtester-client` — no config needed, it's invoked by the test runner.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Create | Crate manifest with two binary targets |
| `Cargo.lock` | Create | Dependency lockfile (committed for binaries) |
| `src/lib.rs` | Create | Shared library: re-exports modules |
| `src/bin/server.rs` | Create | Server binary: control channel, listeners, orchestration |
| `src/bin/client.rs` | Create | Client binary: control channel, stream orchestration |
| `src/protocol/mod.rs` | Create | Control protocol: message types, serialization |
| `src/protocol/clock.rs` | Create | Clock offset estimation (ping-pong) |
| `src/traffic/mod.rs` | Create | Traffic module: packet format, generators |
| `src/traffic/packet.rs` | Create | Packet header struct, serialize/deserialize |
| `src/traffic/generator.rs` | Create | UDP stream generators with rate control |
| `src/traffic/tcp.rs` | Create | TCP stream generator and `TCP_INFO` reader |
| `src/metrics/mod.rs` | Create | Metrics module: collectors and aggregation |
| `src/metrics/latency.rs` | Create | Latency stats, histogram, percentiles |
| `src/metrics/loss.rs` | Create | Loss and reordering detection from sequence numbers |
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

## Implementation Order

### Phase A: Crate scaffold, packet format, and control protocol
- `Cargo.toml` with dependencies (`clap`, `serde`, `serde_json`, `quick-xml`, `libc` for `clock_gettime`)
- Packet header struct with serialize/deserialize (big-endian)
- Control protocol message types and serialization
- Clock offset estimation
- Unit tests for packet round-trip and control messages

### Phase B: Metrics collection
- Latency stats with histogram and percentile computation
- RFC 3550 jitter
- Sequence-based loss and reordering detection
- Throughput calculation
- Per-second time-series bucketing
- Unit tests for each metric

### Phase C: Traffic generators
- UDP stream generator with configurable rate, size, IMIX, sweep patterns
- TCP stream generator with `TCP_INFO` polling
- Timestamp embedding via `clock_gettime(CLOCK_MONOTONIC)`

### Phase D: Report output
- JSON structured output (full results with time-series and histogram)
- JUnit XML with configurable thresholds
- JSONL per-packet raw data
- Human-readable text
- Unit tests for report formatting

### Phase E: Server binary
- CLI parsing with `clap`
- Control channel listener (TCP)
- UDP and TCP data listeners
- Stream-aware metrics collection
- Report generation at test end

### Phase F: Client binary
- CLI parsing with `clap`
- Control channel connection and test negotiation
- Test mode orchestration (throughput, latency, rrul, bidirectional)
- RRUL: baseline phase → loaded phase with concurrent streams
- Clean shutdown on duration expiry or SIGINT/SIGTERM

### Phase G: Dockerfile integration
- Update all three Dockerfiles with multi-stage builder
- Update build context from `images/` to repo root
- Update `publish-images.yml` build context path
- Verify images build successfully

## Testing

- [ ] `cargo build --release` succeeds with no warnings
- [ ] `cargo test` passes all unit tests
- [ ] Packet header serialization/deserialization round-trips correctly
- [ ] Control protocol message exchange works (hello → ready → start → stop → results)
- [ ] Clock offset estimation produces reasonable values on same host
- [ ] UDP latency stream: client sends at configured rate, server measures latency
- [ ] TCP throughput stream: goodput measured, `TCP_INFO` metrics collected
- [ ] IMIX pattern produces correct 7:4:1 size distribution
- [ ] RRUL mode: baseline phase runs without throughput, loaded phase starts throughput streams
- [ ] Bidirectional mode: server→client stream works
- [ ] Per-second time-series data is collected
- [ ] Latency histogram buckets are correct
- [ ] Loss detection: dropped packets are counted correctly
- [ ] Reordering detection: out-of-order packets are flagged
- [ ] JSON output is valid and contains all expected fields
- [ ] JUnit XML with threshold: exceeded threshold produces `<failure>`
- [ ] JSONL output has one valid JSON object per line per packet
- [ ] Text output is human-readable
- [ ] Client binary runs in Alpine container
- [ ] Client binary runs in Debian container
- [ ] Client binary runs in Ubuntu container
- [ ] End-to-end: client → BNG → server with metrics collected

## Not In Scope

- Kernel bypass (AF_XDP, DPDK)
- Encryption or authentication of measurement traffic
- GUI or web dashboard
- Historical data storage
- PTP clock synchronization (cross-host one-way latency uses estimated offset)
