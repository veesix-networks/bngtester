# Decisions: 43-config-file

## Accepted

### Use value_source() to distinguish CLI from defaults
- **Source:** GEMINI (G1), CODEX (C1)
- **Severity:** CRITICAL / HIGH
- **Resolution:** Use clap's `ArgMatches::value_source()` to check `ValueSource::CommandLine` vs `ValueSource::DefaultValue`. Only CLI values from CommandLine override config file. Avoids changing all Cli fields to Option<T>. Added explicit tests for config-only scalars and bools.

### Add server address to ClientConfig
- **Source:** GEMINI (G2)
- **Severity:** CRITICAL
- **Resolution:** `server: Option<String>` in ClientConfig. Client can run with just `--config profile.yaml`. Server positional arg made optional when --config provides it. Error if neither provides server.

### Deep merge by stream ID for stream_overrides
- **Source:** GEMINI (G3)
- **Severity:** HIGH
- **Resolution:** CLI --stream-* flags override matching stream ID fields in config file. CLI-only IDs added, config-only IDs preserved. Not blind list append.

### Standardize size on u32
- **Source:** GEMINI (G4)
- **Severity:** HIGH
- **Resolution:** All size fields use u32 consistently across Cli, config structs, and StreamOverrideEntry.

### Note dependency on #44 for bind fields
- **Source:** GEMINI (G5)
- **Severity:** MEDIUM
- **Resolution:** Spec notes dependency on #44 merge. Bind fields included in config struct but may not be in CLI yet if #44 isn't merged first.

### Validation applies to config values too
- **Source:** GEMINI (G6)
- **Severity:** MEDIUM
- **Resolution:** All config values go through same validation as CLI (parse_mode, parse_dscp, etc.) after merge. Invalid values produce clear errors with field name.

### Switch from deprecated serde_yaml to serde_yml
- **Source:** CODEX (C2)
- **Severity:** HIGH
- **Resolution:** Use `serde_yml` (maintained fork) instead of archived `serde_yaml` (dtolnay).

### Rename streams_config to stream_overrides
- **Source:** CODEX (C3)
- **Severity:** MEDIUM
- **Resolution:** Renamed to `stream_overrides` to match existing naming patterns and avoid confusion with `streams` (count) and protocol `stream_config`.

### Strict rejection of unknown YAML fields
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** `#[serde(deny_unknown_fields)]` on all config structs. Typos produce clear parse errors at startup. No warn-and-continue — test tool misconfiguration is the bigger risk.

## Rejected

### histogram_buckets as Vec<u64> in YAML
- **Source:** GEMINI (G8)
- **Severity:** LOW
- **Rationale:** Keep as string for simplicity and consistency with CLI. The bucket spec format may evolve (e.g., range notation). Converting later is easier than changing the format now.
