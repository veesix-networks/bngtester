# Spec Critique: Per-Stream Configuration (#34)

The feature direction is reasonable, but the spec still leaves a few behavior-defining edges implicit. The biggest risks are silent config drift on packet size and ambiguous semantics for `rate_pps = 0`.

## Findings

### HIGH: `--stream-size` needs an explicit lower bound or normalization rule; otherwise the configured value and emitted packet size can diverge

- The spec introduces raw numeric size overrides in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:41-53` and `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:57-73`, but the testing section only covers the happy path at 64 bytes in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:125-138`.
- The current sender does not preserve sub-header sizes. `build_packet()` silently clamps anything below `HEADER_SIZE` to 32 bytes in `src/traffic/packet.rs:87-93` and `src/traffic/packet.rs:180-190`, and `run_udp_generator()` just passes the resolved size through in `src/traffic/generator.rs:98-109` and `src/traffic/generator.rs:132-143`.
- That means `--stream-size 0=16` would be accepted, the config/report path would likely still say `16`, but the generator would actually send 32-byte packets. This is silent config drift.
- Phase 4 should require one explicit contract:
  - reject sizes below `traffic::packet::HEADER_SIZE` during parsing/resolution, or
  - normalize before hello/report serialization so every surfaced value is the effective 32-byte size rather than the raw input.
- The testing section should add a negative case for sub-header sizes, not just `0=64`.

### MEDIUM: `--stream-rate 0=0` inherits today's "unlimited" sentinel, but the spec/report contract never says that

- The design models rate as a plain `u32` in both `StreamConfig` and `StreamConfigReport` in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:43-50` and `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:79-89`, and the text example renders `@<n>pps` in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:92-98`.
- Today `rate_pps == 0` is not "0 pps"; `run_udp_generator()` treats it as unlimited send-as-fast-as-possible mode in `src/traffic/generator.rs:92-119`.
- If the spec leaves this implicit, an implementation can faithfully pass the override into the generator and still produce a misleading report like `0pps`.
- Phase 4 should either reject zero in per-stream overrides or explicitly preserve the existing unlimited sentinel and define how it is represented in text/JSON. Add tests for that case either way.

### MEDIUM: `src/dscp.rs` is the wrong home for generic stream-config parsing and resolution

- The file plan places `parse_stream_size()`, `parse_stream_rate()`, `parse_stream_pattern()`, `StreamConfig`, and `resolve_stream_config()` in `src/dscp.rs` in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:41-53` and `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:104-115`.
- `src/dscp.rs` already covers DSCP/ECN parsing and TOS socket helpers in `src/dscp.rs:181-310`. Adding size/rate/pattern parsing there makes it a general "stream options" junk drawer instead of a DSCP/ECN module.
- The crate already has a generic stream module with `StreamInfo`, `StreamDirection`, `StreamType`, and `StreamRegistry` in `src/stream/mod.rs:7-105`. That is a more natural home for per-stream override parsing/resolution, either by extending `src/stream/mod.rs` or adding a `src/stream/config.rs`.
- If Phase 4 agrees, the file plan should be updated to name the `src/stream/` changes instead of `src/dscp.rs`.

### MEDIUM: the file plan is only complete for the current UDP constructor, but the spec text reads broader than that

- For generator construction, today's code only instantiates `UdpGeneratorConfig`, and it does so in one place: `src/bin/client.rs:304-313`. The file plan covers that live path via `src/bin/client.rs` in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:104-115`.
- There are no current `TcpGeneratorConfig` construction sites. However, the type already exists and cannot carry size/rate/pattern at all in `src/traffic/tcp.rs:84-91`, while the spec talks about per-stream config for "each data stream" and future reverse-path/RRUL streams in `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:5`, `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:73`, and `context/specs/34-per-stream-config/IMPLEMENTATION_SPEC.md:100-103`.
- That mismatch is the real gap: the concrete plan is "current UDP stream 0 path", but the prose reads like full cross-mode support.
- Phase 4 should make the scope explicit:
  - either narrow the spec to the currently implemented UDP path and say TCP/RRUL wiring remains future work, or
  - broaden the plan later when there are actual TCP generator call sites to change.
- If helpers move out of `dscp.rs`, the file plan also needs the corresponding `src/stream/` module entries.

## Positioning Note

I would keep `StreamConfigReport` on `StreamReport`, not `StreamResults`.

- `StreamReport` already holds per-stream identity/config metadata (`dscp`, `dscp_name`, `ecn_mode`), while `StreamResults` is the measurement bucket in `src/report/mod.rs:39-93`.
- The text formatter currently pulls `throughput_pps` from `StreamResults` for the stream header in `src/report/text.rs:55-77`, but that field is observed throughput, not configured rate. Putting configured size/rate/pattern under `StreamResults` would blur input vs measurement even further.
- `StreamReport.config` keeps the model clean: stream metadata/config at the report level, observed results under `results`.
