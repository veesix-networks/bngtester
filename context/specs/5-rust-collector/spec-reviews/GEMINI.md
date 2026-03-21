# Spec Review: Rust Collector — Server and Client Binaries

- **Spec:** [IMPLEMENTATION_SPEC.md](../IMPLEMENTATION_SPEC.md)
- **Reviewer:** Gemini
- **Date:** 2026-03-20
- **Status:** Phase 2 (Spec Refinement)

## Executive Summary

The specification for `rust-collector` is technically sound and well-aligned with the goal of validating a VPP-based BNG with sub-microsecond precision. The dual-channel architecture and RRUL design are appropriate for identifying bufferbloat and queue management issues. The use of `TCP_INFO` for TCP metrics and `CLOCK_MONOTONIC` for latency probes provides the necessary depth for high-performance network testing.

Several areas require clarification or enhancement to ensure robustness, particularly regarding control channel reliability during high-load tests and port negotiation for concurrent data streams.

## Findings

### 1. Packet Format & Clock Model
**Severity: MEDIUM**

- **Clock Synchronization (Cross-Host):** The spec proposes a "ping-pong sequence" for clock offset estimation. While sufficient for millisecond-level measurements, sub-microsecond accuracy across hosts is extremely difficult without PTP (Precision Time Protocol) or shared hardware clocks. The spec correctly identifies that this is "sufficient for lab environments," but should explicitly state that **one-way latency results in cross-host scenarios are only as accurate as the symmetry of the path delay.**
- **Redundant Timestamp Fields:** The packet header uses `u64` seconds + `u32` nanoseconds. A single `u64` for total nanoseconds since epoch/boot would simplify calculations and save 4 bytes (or keep the 4 bytes for other flags/fields). However, the current 12-byte timestamp is standard.
- **Timestamp Overhead:** To achieve sub-microsecond precision at high packet rates, the overhead of `clock_gettime` is critical. While it is usually a VDSO call, the implementation should ensure it isn't calling into the kernel unnecessarily.
- **Sequence Number Wrap-around:** A `u32` sequence number wraps after ~4.29 billion packets. At 10Gbps with 64-byte packets (~14.8M pps), this happens in ~290 seconds (under 5 minutes). For long-duration tests, the loss/reordering detection logic must handle wrap-around correctly.

### 2. RRUL Test Design & Phasing
**Severity: LOW**

- **Baseline vs. Loaded Duration:** The default test duration is 10s, with a 5s baseline. This leaves only 5s for the loaded phase, which may not be enough for TCP streams to reach steady-state (especially with slow-start or BNG shaping).
- **Recommendation:** Increase default duration to 30s or reduce default baseline to 2s.
- **TCP Stream Coordination:** The spec doesn't specify if the 4 TCP throughput streams start simultaneously or are staggered. Staggering (e.g., 100ms apart) can prevent synchronized slow-start behavior that might mask certain BNG scheduling issues.

### 3. Control Protocol Robustness
**Severity: HIGH**

- **Data Channel Port Negotiation:** The spec mentions "UDP and TCP data listeners" but does not define how ports are assigned for the multiple concurrent streams (e.g., 4 TCP + 2 UDP in RRUL mode).
- **Recommendation:** The `ready` message from the server should include a mapping of `Stream ID` to `Port Number` so the client knows where to connect/send for each stream. Alternatively, the server could listen on a single port and demux based on `Stream ID` in the packet header (for UDP) or a small handshake (for TCP).
- **Control Channel Keep-alive:** During an RRUL test, the control channel (TCP) might remain idle for 30+ seconds while the link is saturated. Some BNGs or middleboxes might drop idle TCP connections. If the control channel drops, the test may fail to stop or exchange results.
- **Recommendation:** Implement a simple heartbeat/ping-pong on the control channel every 1-5 seconds during the test.
- **Error Handling on Stream Failure:** If one of the 4 TCP streams fails to connect, does the whole test fail, or does it proceed with 3 streams? The spec should define the failure behavior.

### 4. Dockerfile & Build Approach
**Severity: MEDIUM**

- **Musl Allocator Performance:** Musl's default memory allocator can be a bottleneck for high-concurrency network applications. For a high-speed traffic generator, consider linking with `jemalloc` or `mimalloc`.
- **Build Context Impact:** Changing the build context to the repo root is necessary but will break the current `publish-images.yml` if not carefully updated. The spec correctly identifies this.
- **Multi-Stage Cache Efficiency:** Ensure `Cargo.lock` and `Cargo.toml` are copied and a dummy build is performed before copying `src/` to leverage Docker's layer caching for dependencies.

### 5. Measurement & Reporting
**Severity: LOW**

- **Goodput vs. Throughput:** The distinction is well-made. Ensure the "Raw Throughput" calculation for TCP includes an estimate of the L2/L3 overhead if possible, or clearly label it as "L4 Goodput."
- **Histogram Buckets:** The default buckets (10us up to 1ms) are excellent for VPP-based BNGs. Ensure the bucket configuration is flexible via the CLI or config.

## Suggested Changes

1. **Control Protocol:** Update the `ready` message structure to include port assignments for each requested stream.
2. **Control Protocol:** Add a `ping` / `pong` or `heartbeat` message type to keep the control channel alive during long tests.
3. **RRUL:** Add a `ramp_up` parameter (default 1s) for TCP streams to avoid "thundering herd" start-up effects.
4. **Implementation:** Use `jemalloc-ator` in the Rust crate to ensure high-performance allocation on musl.
5. **Packet Format:** Explicitly define the behavior for `u32` sequence number wrap-around in the `loss.rs` and `reordering.rs` modules.

## Conclusion

The spec is very high quality and provides a clear roadmap for implementation. Addressing the control channel port negotiation and keep-alives will ensure it survives real-world BNG testing environments where links are saturated and idle connections are aggressively reaped.
