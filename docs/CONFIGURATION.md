# Configuration Reference

Complete reference for bngtester-client and bngtester-server parameters. All parameters can be set via CLI flags or YAML config file. CLI flags override config file values.

## Config File

Both binaries accept `--config <PATH>` to load a YAML config file. CLI flags override config values.

```bash
bngtester-client --config examples/latency-test.yaml
bngtester-client --config examples/rrul-bufferbloat.yaml --duration 60  # override duration
bngtester-server --config examples/multi-subscriber-server.yaml
```

See `examples/` for sample config files.

---

## Client Reference

### Usage

```
bngtester-client [OPTIONS] [SERVER]
```

`SERVER` is optional if provided in the config file.

### Test Configuration

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `[SERVER]` | `server` | string | _(required)_ | Server address (host:port) |
| `--config` | — | path | _(none)_ | YAML config file path |
| `-m, --mode` | `mode` | string | `latency` | Test mode: `throughput`, `latency`, `rrul`, `bidirectional` |
| `-p, --protocol` | `protocol` | string | `tcp` | Protocol for throughput streams: `tcp`, `udp` |
| `-s, --size` | `size` | u32 | `512` | Packet size in bytes (min 32) |
| `-r, --rate` | `rate` | u32 | `100` | Latency probe rate in packets/sec (0 = unlimited) |
| `-d, --duration` | `duration` | u32 | `30` | Test duration in seconds |
| `-P, --pattern` | `pattern` | string | `fixed` | Traffic pattern: `fixed`, `imix`, `sweep` |
| `--streams` | `streams` | u32 | `2` | Number of throughput streams per direction |
| `--cross-host` | `cross_host` | bool | `false` | Use clock offset estimation for cross-host testing |

### RRUL Settings

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--rrul-baseline` | `rrul_baseline` | u32 | `5` | Baseline phase duration in seconds (latency probes only) |
| `--rrul-ramp-up` | `rrul_ramp_up` | u32 | `100` | Delay between TCP stream starts in ms |

### DSCP / ECN Marking

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--dscp` | `dscp` | string | _(none)_ | DSCP codepoint: `BE`, `CS0`-`CS7`, `AF11`-`AF43`, `EF`, or `0`-`63` |
| `--ecn` | `ecn` | string | _(none)_ | ECN mode: `ect0` or `ect1` |

DSCP and ECN are combined into the TOS byte. Example: `--dscp EF --ecn ect0` = TOS 0xBA.

### Per-Stream Overrides

| Flag | Config Key | Type | Description |
|------|-----------|------|-------------|
| `--stream-size ID=BYTES` | `stream_overrides[].size` | u32 | Packet size for stream ID (min 32) |
| `--stream-rate ID=PPS` | `stream_overrides[].rate` | u32 | Rate for stream ID (0 = unlimited) |
| `--stream-pattern ID=PAT` | `stream_overrides[].pattern` | string | Pattern for stream ID |
| `--stream-dscp ID=DSCP` | `stream_overrides[].dscp` | string | DSCP for stream ID |

CLI flags use `ID=VALUE` format. Config file uses a list:

```yaml
stream_overrides:
  - id: 0
    size: 64
    rate: 10000
    pattern: fixed
    dscp: EF
  - id: 1
    size: 1518
    rate: 500
    pattern: imix
    dscp: AF41
```

Resolution: per-stream override > global default. Last match wins for repeated IDs.

### Network Binding

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--bind-iface` | `bind_iface` | string | _(none)_ | Bind data sockets to interface via SO_BINDTODEVICE |
| `--source-ip` | `source_ip` | IP | _(any)_ | Bind data sockets to source IP |
| `--control-bind-ip` | `control_bind_ip` | IP | _(any)_ | Bind control channel TCP to source IP |

For bare metal testing where the client needs to send traffic via a specific interface.

### Identity

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--client-id` | `client_id` | string | _(source IP:port)_ | Client identifier for multi-subscriber coordination |

### Output

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `-o, --output` | `output` | string | `text` | Report format: `json`, `junit`, `text` |
| `-f, --file` | `file` | path | _(stdout)_ | Write report to file |
| `--raw-file` | `raw_file` | path | _(none)_ | Write per-packet JSONL data to file |

### Thresholds

| Flag | Config Key | Type | Description |
|------|-----------|------|-------------|
| `--threshold KEY=VAL` | `thresholds` | map | JUnit pass/fail thresholds |

**Threshold keys:**

| Key | Unit | Description |
|-----|------|-------------|
| `loss` | % | Max acceptable packet loss percentage |
| `p50` | us | Max acceptable p50 latency in microseconds |
| `p95` | us | Max acceptable p95 latency |
| `p99` | us | Max acceptable p99 latency |
| `p999` | us | Max acceptable p999 latency |
| `jitter` | us | Max acceptable jitter |
| `throughput` | Mbps | Min acceptable throughput |
| `bloat` | ratio | Max acceptable bufferbloat ratio (loaded_p99 / baseline_p99) |

CLI: `--threshold p99=1000 --threshold loss=0.1`

Config:
```yaml
thresholds:
  p99: 1000
  loss: 0.1
  bloat: 3.0
```

---

## Server Reference

### Usage

```
bngtester-server [OPTIONS]
```

### Configuration

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--config` | — | path | _(none)_ | YAML config file path |
| `-l, --listen` | `listen` | addr | `0.0.0.0:5000` | Listen address for control channel |
| `-o, --output` | `output` | string | `text` | Report format: `json`, `junit`, `text` |
| `-f, --file` | `file` | path | _(stdout)_ | Write report to file |
| `--raw-file` | `raw_file` | path | _(none)_ | Write per-packet JSONL data to file |
| `--threshold KEY=VAL` | `thresholds` | map | _(none)_ | JUnit pass/fail thresholds (same keys as client) |
| `--histogram-buckets` | `histogram_buckets` | string | _(default)_ | Latency histogram bucket specification |

### Multi-Subscriber

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--combined` | `combined` | bool | `false` | Collect results from multiple clients into one report |
| `--max-clients` | `max_clients` | u32 | `1` | Number of client sessions to wait for (combined mode) |
| `--timeout` | `timeout` | u64 | `300` | Timeout in seconds waiting for all clients |

### Network Binding

| Flag | Config Key | Type | Default | Description |
|------|-----------|------|---------|-------------|
| `--data-bind-iface` | `data_bind_iface` | string | _(none)_ | Bind receiver UDP socket to specific interface |

---

## Report Formats

### JSON

Full structured output with all metrics, time-series, and histogram data. Fields are omitted when not applicable (e.g., no ECN fields when ECN is off).

```bash
bngtester-client 10.0.0.2:5000 -o json -f results.json
```

Key fields:
- `status`: `"complete"`, `"interrupted"`, or `"partial"`
- `clock_mode`: `"same-host"` or `"sync-estimated"`
- `streams[]`: per-stream results with latency, loss, jitter, throughput, ECN counters
- `bufferbloat`: bloat ratio (RRUL mode only)
- `time_series[]`: per-second metrics
- `histogram`: latency distribution buckets

### JUnit XML

CI integration format. Each metric becomes a test case. Thresholds determine pass/fail.

```bash
bngtester-client 10.0.0.2:5000 -o junit --threshold p99=1000 --threshold loss=0.1
```

Test cases: `packet_loss`, `latency_p50/p95/p99/p999`, `jitter`, `throughput`, `bufferbloat`.

### Text

Human-readable summary with Unicode formatting.

```
bngtester LATENCY test — 10s duration
══════════════════════════════════════════════════

Status: Complete | Clock: same-host

  Stream 0 [udp_latency ↑ DSCP=EF ECN=ECT0 64B@10000pps fixed] 10001pps
    Latency:  min=4.5µs avg=45.2µs max=312.8µs p99=201.3µs
    Jitter:   8.7µs
    Loss:     0.067% (2/2998)
    ECN:      CE=0 (0.0%) ECT0=2998 ECT1=0 Not-ECT=0
    Throughput: 5.1 Mbps
```

### JSONL (Per-Packet)

One JSON object per line per received packet. For external analysis (pandas, matplotlib).

```bash
bngtester-client 10.0.0.2:5000 --raw-file packets.jsonl
```

```jsonl
{"stream":0,"seq":1,"send_ts_ns":1234567890000,"recv_ts_ns":1234567890450,"size":64,"latency_ns":450}
```

### Combined Report (Multi-Subscriber)

When using `--combined` on the server, the report wraps multiple client results:

```bash
bngtester-server --combined --max-clients 2 -o json -f combined.json
```

JSON adds `"combined": true` with a `"clients"` array. Text shows per-client sections. JUnit creates per-client test suites.

---

## Common Test Scenarios

### Basic Latency Test

```bash
bngtester-server -l 0.0.0.0:5000 &
bngtester-client 10.0.0.2:5000 -d 10 -r 100
```

Or with config: `bngtester-client --config examples/latency-test.yaml`

### RRUL Bufferbloat Test

```bash
bngtester-server -l 0.0.0.0:5000 &
bngtester-client 10.0.0.2:5000 -m rrul -d 30 --rrul-baseline 5 \
  --threshold bloat=3.0 --threshold p99=1000 -o junit
```

Or with config: `bngtester-client --config examples/rrul-bufferbloat.yaml`

### Multi-DSCP QoS Validation

```bash
bngtester-server -l 0.0.0.0:5000 &
bngtester-client 10.0.0.2:5000 -d 10 --dscp EF --ecn ect0 \
  --stream-size 0=64 --stream-rate 0=10000 --stream-dscp 0=EF \
  --stream-size 1=1518 --stream-rate 1=500 --stream-dscp 1=AF41
```

Or with config: `bngtester-client --config examples/multi-dscp-qos.yaml`

### Multi-Subscriber Fairness Test

Server:
```bash
bngtester-server --combined --max-clients 2 --timeout 60 -o json -f results.json
```

Subscriber 1:
```bash
bngtester-client 10.0.0.2:5000 --client-id sub1 --dscp EF -d 10
```

Subscriber 2:
```bash
bngtester-client 10.0.0.2:5000 --client-id sub2 --dscp AF41 -d 10
```

Or with config: `bngtester-server --config examples/multi-subscriber-server.yaml`

### Bare Metal Loopback

Prerequisites:
```bash
sysctl -w net.ipv4.conf.all.rp_filter=0
sysctl -w net.ipv4.conf.eth1.rp_filter=0
```

Server (network side):
```bash
bngtester-server -l 10.0.0.2:5000 --data-bind-iface eth2
```

Client (access side):
```bash
bngtester-client 10.0.0.2:5000 --bind-iface eth1 --source-ip 10.255.0.2 \
  --control-bind-ip 10.255.0.2 --dscp EF
```

Or with config: `bngtester-client --config examples/bare-metal-loopback.yaml`

---

## Traffic Patterns

| Pattern | Description |
|---------|-------------|
| `fixed` | All packets use the configured `--size` |
| `imix` | Internet Mix: 7:4:1 ratio of 64:594:1518 byte packets |
| `sweep` | Incrementing sizes: 64, 128, 256, 512, 768, 1024, 1280, 1518 bytes |

## DSCP Codepoints

| Name | Value | Description |
|------|-------|-------------|
| `BE` / `CS0` | 0 | Best Effort |
| `CS1`-`CS7` | 8,16,24,32,40,48,56 | Class Selector |
| `AF11`-`AF43` | 10,12,14,18,20,22,26,28,30,34,36,38 | Assured Forwarding |
| `EF` | 46 | Expedited Forwarding |

Numeric values 0-63 also accepted.

## ECN Modes

| Mode | ECN Bits | Description |
|------|----------|-------------|
| _(off)_ | 00 | Not ECN-capable (default) |
| `ect0` | 10 | ECN-capable transport, codepoint 0 |
| `ect1` | 01 | ECN-capable transport, codepoint 1 |

The receiver detects all four ECN states: Not-ECT (00), ECT(1) (01), ECT(0) (10), CE (11). CE marks indicate AQM congestion signaling. Not-ECT received when ECT was sent indicates ECN stripping by the BNG.
