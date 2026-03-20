# Decisions: 5-rust-collector

## Accepted

### Sequence number wrap-around handling
- **Source:** GEMINI (G1)
- **Severity:** MEDIUM
- **Resolution:** u32 wraps in ~5min at 10Gbps line rate with 64-byte packets. Added wrap-around logic to loss/reordering detection: gaps > u32::MAX/2 are treated as wraps. Added wrap-around unit tests to testing checklist.

### Default test duration increased to 30s
- **Source:** GEMINI (G4)
- **Severity:** LOW
- **Resolution:** Changed default `--duration` from 10s to 30s. With 5s baseline, this gives 25s loaded phase — enough for TCP to reach steady state and for BNG shaping to stabilize.

### TCP stream staggered start in RRUL
- **Source:** GEMINI (G5)
- **Severity:** LOW
- **Resolution:** Added `--rrul-ramp-up` flag (default 100ms) for delay between TCP stream starts. Avoids synchronized slow-start that could mask BNG scheduling behavior.

### Data channel port negotiation in ready message
- **Source:** GEMINI (G6)
- **Severity:** HIGH
- **Resolution:** Added stream binding model section. Server allocates ports dynamically, reports them in `ready` message. Client reports downstream listener ports in `start` message. UDP streams share a socket (demux by stream_id), TCP streams get per-port assignments.

### Control channel heartbeat keepalive
- **Source:** GEMINI (G7)
- **Severity:** HIGH
- **Resolution:** Added `heartbeat` message type to control protocol. Sent every 5s by both sides during RUNNING state. 3 missed heartbeats (15s) triggers session timeout and failure path. Prevents silent hangs during link saturation.

### Stream failure behavior defined
- **Source:** GEMINI (G8)
- **Severity:** HIGH
- **Resolution:** Added failure state table to session state machine. Stream failures continue the test in degraded mode (3/4 TCP streams in RRUL is still useful). Failed streams marked with status in results. Control channel loss triggers partial result output.

### Use jemalloc on musl
- **Source:** GEMINI (G9)
- **Severity:** MEDIUM
- **Resolution:** Added `jemallocator` as global allocator in Cargo.toml. Musl's default allocator is a bottleneck for high-rate packet processing.

### Docker layer caching for dependencies
- **Source:** GEMINI (G10)
- **Severity:** MEDIUM
- **Resolution:** Updated Dockerfile example to copy Cargo.toml/Cargo.lock and run a dummy build before copying src/, leveraging Docker's layer cache for dependency compilation.

### Configurable histogram buckets
- **Source:** GEMINI (G11)
- **Severity:** LOW
- **Resolution:** Added `--histogram-buckets` CLI flag to both server and client.

### Session state machine with failure taxonomy
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** Added full session state machine (INIT → NEGOTIATING → SYNCING → READY → RUNNING → COLLECTING → DONE) with failure states table covering control channel loss, partial stream setup, early stream exit, clock estimation failure, and SIGINT/SIGTERM. Reports include top-level `status` field.

### Concrete stream binding model and concurrency
- **Source:** CODEX (C2)
- **Severity:** HIGH
- **Resolution:** Added stream binding model section with explicit port assignment protocol. Chose tokio async runtime for concurrency. Added `stream/mod.rs` (stream registry), `traffic/receiver.rs` (client-side downstream listener), and `protocol/session.rs` to file plan. Added NAT/firewall assumption (direct L3 connectivity required).

### Report ownership — client is primary producer
- **Source:** CODEX (C3)
- **Severity:** HIGH
- **Resolution:** Added output/threshold/raw-file flags to client CLI. Client receives server metrics via `results` control message and merges with its own send-side metrics. Client is the primary report producer since it's the CI-invoked entrypoint. Both sides can still produce independent reports.

### Pin TCP_INFO required fields
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** Pinned required fields: tcpi_rtt, tcpi_rttvar, tcpi_total_retrans, tcpi_snd_cwnd. Missing fields reported as null. Added Linux-specific portability note to spec and Not In Scope.

### File plan gaps filled
- **Source:** CODEX (C5)
- **Severity:** MEDIUM
- **Resolution:** Added `.github/workflows/publish-images.yml` (modify), `.dockerignore` (create), `src/protocol/session.rs`, `src/traffic/receiver.rs`, `src/stream/mod.rs` to file plan. Added SPDX header check to testing section.

## Rejected

### Use single u64 for timestamp instead of sec+nsec
- **Source:** GEMINI (G2)
- **Severity:** MEDIUM
- **Rationale:** The sec+nsec format matches libc's `timespec` struct directly, which is what `clock_gettime` returns. Using the native layout avoids conversion at the hot path (every packet send/receive). The 4-byte savings per packet is not meaningful given the 32-byte minimum header.

### clock_gettime VDSO overhead concern
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Rationale:** `clock_gettime(CLOCK_MONOTONIC)` is a VDSO call on all modern Linux kernels — it does not enter the kernel. At the packet rates we're targeting (up to ~14.8M pps at 10Gbps with 64-byte packets), VDSO overhead is negligible. No mitigation needed.
