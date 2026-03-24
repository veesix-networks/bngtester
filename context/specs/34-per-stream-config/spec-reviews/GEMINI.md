# Spec Review: Per-Stream Configuration (#34)

## Overview
The specification provides a solid foundation for per-stream overrides, which is critical for supporting mixed-traffic scenarios like RRUL. The choice of `ID=VALUE` CLI flags is consistent with existing patterns in the project.

## Findings

### CRITICAL: None

### HIGH: Per-Stream Configuration Fragmentation
The spec adds a new `stream_config` vector to `HelloMsg` while keeping the existing `stream_dscp` vector from #32. 
- **Problem:** Having two separate vectors for per-stream overrides is redundant and inconsistent. It makes the protocol more complex and harder to maintain.
- **Recommendation:** Consolidate `stream_dscp` into the new `StreamConfigOverride` struct. Since #32 is part of the current development cycle, it's better to unify them now.
- **Proposed Struct:**
  ```rust
  pub struct StreamConfigOverride {
      pub stream_id: u8,
      pub size: Option<u32>,
      pub rate_pps: Option<u32>,
      pub pattern: Option<TrafficPattern>,
      pub dscp: Option<u8>,
  }
  ```

### MEDIUM: RRUL Stream ID Mapping
The spec mentions that per-stream config prepares for RRUL mode, but does not define the stream ID mapping for RRUL.
- **Problem:** Users won't know which stream ID corresponds to which traffic (e.g., upstream latency vs. downstream throughput) when using RRUL.
- **Recommendation:** Define a stable mapping of stream IDs for the multi-stream modes (RRUL, Bidirectional) in the spec so users can reliably apply overrides.

### MEDIUM: Resolution Strategy Ambiguity
The current implementation of `resolve_stream_dscp` (which the spec uses as a template) uses "first match wins" (`iter().find()`).
- **Problem:** CLI users typically expect "last match wins" if a flag is repeated (e.g., `--stream-size 0=64 --stream-size 0=128`).
- **Recommendation:** Explicitly state the resolution strategy for multiple overrides of the same ID. "Last match wins" is recommended for CLI consistency.

### LOW: Validation of Overrides
The spec doesn't explicitly mention validation for override values.
- **Recommendation:** Add validation in the parsing helpers (`parse_stream_size`, etc.) to ensure packet sizes are within UDP/IP limits (e.g., 64 to 65507 bytes) and rates are positive.

### LOW: Report Clarity for Partial Overrides
The spec shows example output where all fields are overridden.
- **Question:** How will the report handle a stream that only overrides *one* field?
- **Recommendation:** The report should clearly distinguish between overridden values and global defaults. Using a marker (like an asterisk) or only showing overridden fields in the bracketed header could help.

## Backward Compatibility
The use of `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for the new `HelloMsg` fields correctly handles backward compatibility with older servers that don't recognize the new fields.

## RRUL Interaction
In RRUL mode, the server creates reverse-path streams. The spec should explicitly state that the server **MUST** use the overrides received in `HelloMsg` for these reverse streams (e.g., if stream 0 has a size override, the server's reverse stream 0 should respect it).
