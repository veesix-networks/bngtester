# Decisions: 34-per-stream-config

## Accepted

### Consolidate stream_dscp into StreamConfigOverride
- **Source:** GEMINI (G1)
- **Severity:** HIGH
- **Resolution:** Merged `StreamDscpConfig` into `StreamConfigOverride` which now carries size, rate, pattern, and dscp fields. Single vector in HelloMsg instead of two separate ones. `--stream-dscp` CLI preserved — populates dscp field within the unified struct.

### RRUL stream ID mapping documented
- **Source:** GEMINI (G2)
- **Severity:** MEDIUM
- **Resolution:** Added stream ID mapping reference (0-1: UDP latency, 2-5: TCP throughput) for future RRUL implementation. Not enforced since RRUL multi-stream is not yet implemented.

### Last match wins for repeated CLI flags
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Resolution:** Resolution uses last match wins — `--stream-size 0=64 --stream-size 0=128` gives 128. Consistent with standard CLI behavior.

### Validate size and rate values
- **Source:** GEMINI (G4)
- **Severity:** LOW
- **Resolution:** Size must be >= HEADER_SIZE (32). Rate 0 is valid (unlimited). Invalid values rejected at parse time.

### Show all resolved values in report
- **Source:** GEMINI (G5)
- **Severity:** LOW
- **Resolution:** Report shows resolved config (size, rate, pattern) when overrides active. All values shown, not just overridden ones.

### Reject sub-header packet sizes at parse time
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** `parse_stream_size()` rejects values below `HEADER_SIZE` (32 bytes) with clear error. No silent clamping by `build_packet()`. Added negative test case.

### Rate 0 = unlimited, documented and rendered
- **Source:** CODEX (C2)
- **Severity:** MEDIUM
- **Resolution:** Rate 0 is unlimited (existing sentinel). Text report renders as "unlimited". JSON keeps `0`. Added test case.

### Parsing helpers in src/stream/config.rs, not dscp.rs
- **Source:** CODEX (C3)
- **Severity:** MEDIUM
- **Resolution:** Created `src/stream/config.rs` for all per-stream config parsing and resolution. `dscp.rs` remains focused on DSCP/ECN/TOS. Updated file plan.

### Scope narrowed to current UDP path
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** Explicitly scoped to current UDP stream path. TCP generator config extension is future work when RRUL multi-stream is implemented. Added to Not In Scope.
