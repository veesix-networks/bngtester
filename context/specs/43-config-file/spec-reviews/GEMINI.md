# Spec Review: Config File Support (#43) — GEMINI

## Summary
The spec provides a solid foundation for adding YAML configuration support, which is critical for managing the increasing number of CLI flags. The choice of YAML is appropriate for the target audience and for representing nested stream configurations. However, several critical issues related to merge priority, field types, and completeness (specifically the `server` target) need to be addressed to ensure a seamless user experience.

## Findings

### CRITICAL

#### 1. CLI Defaults Shadow Config File Values
- **Problem:** In the current `src/bin/client.rs` and `src/bin/server.rs` implementations, `clap` is configured with `default_value` for most fields (e.g., `duration: 30`, `mode: latency`). If the user runs `bngtester-client --config profile.yaml`, `cli.duration` will be `30` regardless of whether the user specified `--duration` or not. If the merge logic follows "CLI flags > Config file", the config file value will NEVER be used unless the CLI default is bypassed.
- **Impact:** Config files will be unable to override any field that has a default value in the `Cli` struct.
- **Recommendation:** 
  1. Change all `Cli` fields (except `--config` and `server`) to `Option<T>` and remove `default_value` from `clap` attributes.
  2. Implement a three-way merge: `user_provided_cli_value.unwrap_or(config_file_value).unwrap_or(builtin_default)`.
  3. Alternatively, use `clap`'s `ValueSource` to distinguish between `CommandLine` and `DefaultValue`.

#### 2. Missing `server` Target in `ClientConfig`
- **Problem:** The `ClientConfig` struct in the spec does not include the `server` address (currently a required positional argument in `Cli`).
- **Impact:** Users cannot define a truly "complete" test profile in YAML. They must always provide the server address on the command line (e.g., `bngtester-client 10.0.0.1:5000 --config profile.yaml`).
- **Recommendation:** Add `server: Option<String>` to `ClientConfig`. Update `Cli` to make the `server` positional argument optional if `--config` is present.

### HIGH

#### 1. Merging Logic for `streams_config` and `thresholds`
- **Problem:** The spec says "For Vec fields (thresholds, streams), CLI values append/override config values." For `streams_config`, which is a `Vec<StreamConfigEntry>`, a simple append is insufficient.
- **Impact:** If YAML defines stream 0 and CLI defines `--stream-size 0=1500`, a simple append would create two entries for stream 0 or ignore one, instead of merging the size override into the existing stream 0 definition.
- **Recommendation:** Implement a "Deep Merge by ID" for `streams_config`. CLI overrides for a specific stream ID should update the fields of the corresponding `StreamConfigEntry` from the YAML file. Same for `thresholds` (merge by key).

#### 2. Type Mismatch: `usize` vs `u32` for `size`
- **Problem:** `Cli.size` is `usize`, but `StreamConfigEntry.size` in the spec is `u32`. `src/stream/config.rs` also uses `u32`.
- **Impact:** Inconsistent types across the codebase and config structs. `usize` is platform-dependent and less ideal for network protocol fields like packet size.
- **Recommendation:** Standardize on `u32` for all packet size fields in `Cli`, `ClientConfig`, and `StreamConfigEntry`.

### MEDIUM

#### 1. "Bind" Flags Mentioned but Missing from Code
- **Problem:** The spec mentions `bind_iface`, `source_ip`, and `control_bind_ip` as existing flags ("Client has 23 CLI flags including ... bind"). However, these flags are not present in the current `feat/config-file` branch's `src/bin/client.rs`.
- **Impact:** The spec assumes existence of features that might still be in other branches (`feat/bind-interface`).
- **Recommendation:** Clarify if these flags are being added *as part of* this PR, or if this spec depends on the merge of `feat/bind-interface`.

#### 2. Validation of Merged Config
- **Problem:** Current validation (e.g., `parse_mode`, `parse_pattern`) happens during CLI parsing in `main`. Config file values need the same validation.
- **Impact:** Invalid values in YAML (e.g., `mode: "invalid"`) might not be caught until deep in the execution or might cause panics if not handled.
- **Recommendation:** Move validation logic (mode, pattern, protocol, DSCP strings) to `src/config.rs` or `src/lib.rs` so it can be applied to both CLI and Config file values after the merge.

### LOW

#### 1. Forward Compatibility with Unknown Fields
- **Problem:** The spec says "Unknown fields → warning (not error)". 
- **Recommendation:** Ensure `serde` is configured to allow unknown fields. If using `serde_yaml`, this is generally the default unless `#[serde(deny_unknown_fields)]` is used. However, explicitly using `#[serde(default)]` on fields is recommended.

#### 2. Histogram Bucket Format
- **Problem:** `histogram_buckets` in the server is an `Option<String>` in `Cli` and `String` in the YAML example.
- **Recommendation:** Keep it as `String` in YAML for simplicity, but consider if it should be `Vec<u64>` to be more "YAML-native" than a comma-separated string.

## Conclusion
The spec is well-defined but needs adjustments to the `Cli` struct definition to allow config files to actually work (by removing `clap` defaults). The inclusion of the `server` target in the config file will greatly improve the utility of test profiles.
