# Implementation Spec: Config File Support (YAML Test Profiles)

## Overview

Add `--config <PATH>` flag to both bngtester-client and bngtester-server that loads test configuration from a YAML file. Config files define complete test profiles — server address, streams, DSCP/ECN, packet sizes, rates, patterns, thresholds, bind settings, and output options in a single file. CLI flags override config file values (using clap's `value_source()` to distinguish user-provided from defaults). Strict rejection of unknown YAML fields to prevent typo-driven misconfigurations.

## Source Issue

[#43 — Config file support (YAML/TOML test profiles)](https://github.com/veesix-networks/bngtester/issues/43)

## Current State

- Client has 23+ CLI flags including per-stream overrides, DSCP, ECN, bind, thresholds.
- Server has 9+ CLI flags including combined mode, max-clients, timeout.
- All `Cli` struct fields use `default_value` — clap cannot distinguish "user typed --duration 30" from "default 30".
- `serde` and `serde_json` already in deps.
- Depends on #44 (bind-interface) being merged for bind config fields.

## Design

### Format: YAML

YAML via a maintained crate. The original `serde_yaml` (dtolnay) is deprecated/archived. Use `serde_yml` (maintained fork) instead.

### Client Config File

```yaml
# bngtester client test profile
server: 10.0.0.2:5000

mode: rrul
duration: 30
protocol: tcp
size: 512
rate: 100
pattern: fixed
cross_host: false

# RRUL settings
rrul_baseline: 5
rrul_ramp_up: 100
streams: 2

# Marking
dscp: EF
ecn: ect0

# Bind (requires #44)
bind_iface: eth1
source_ip: 10.255.0.2
control_bind_ip: 10.255.0.2

# Identity
client_id: subscriber-1

# Output
output: json
file: results.json
raw_file: packets.jsonl

# Thresholds (merged by key)
thresholds:
  p99: 1000
  loss: 0.1
  bloat: 3.0
  jitter: 100

# Per-stream overrides (merged by stream id)
stream_overrides:
  - id: 0
    size: 64
    rate: 10000
    pattern: fixed
    dscp: AF41
  - id: 1
    size: 1518
    rate: 500
    pattern: imix
    dscp: BE
```

### Server Config File

```yaml
# bngtester server config
listen: 0.0.0.0:5000
output: json
file: server-results.json
raw_file: server-packets.jsonl
data_bind_iface: eth2

# Multi-subscriber
combined: true
max_clients: 4
timeout: 120

# Thresholds
thresholds:
  p99: 1000
  loss: 0.1
```

### Merge Priority via value_source()

```
User-provided CLI flag > Config file value > Built-in default
```

Implementation uses clap's `ArgMatches::value_source()` to distinguish `ValueSource::CommandLine` from `ValueSource::DefaultValue`:

1. Parse CLI via `Cli::parse()` to get `ArgMatches`
2. Load config file (if `--config` provided)
3. For each field: if `value_source() == CommandLine`, use CLI value. Otherwise use config file value if present, else built-in default.

This avoids changing all `Cli` fields to `Option<T>` — the existing struct stays as-is, and the merge logic reads from `ArgMatches` source metadata.

### Config Structs

```rust
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ClientConfig {
    pub server: Option<String>,
    pub mode: Option<String>,
    pub duration: Option<u32>,
    pub protocol: Option<String>,
    pub size: Option<u32>,
    pub rate: Option<u32>,
    pub pattern: Option<String>,
    pub cross_host: Option<bool>,
    pub rrul_baseline: Option<u32>,
    pub rrul_ramp_up: Option<u32>,
    pub streams: Option<u32>,
    pub dscp: Option<String>,
    pub ecn: Option<String>,
    pub bind_iface: Option<String>,
    pub source_ip: Option<String>,
    pub control_bind_ip: Option<String>,
    pub client_id: Option<String>,
    pub output: Option<String>,
    pub file: Option<String>,
    pub raw_file: Option<String>,
    pub thresholds: Option<HashMap<String, f64>>,
    pub stream_overrides: Option<Vec<StreamOverrideEntry>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamOverrideEntry {
    pub id: u8,
    pub size: Option<u32>,
    pub rate: Option<u32>,
    pub pattern: Option<String>,
    pub dscp: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ServerFileConfig {
    pub listen: Option<String>,
    pub output: Option<String>,
    pub file: Option<String>,
    pub raw_file: Option<String>,
    pub data_bind_iface: Option<String>,
    pub combined: Option<bool>,
    pub max_clients: Option<u32>,
    pub timeout: Option<u64>,
    pub thresholds: Option<HashMap<String, f64>>,
}
```

### Schema Strictness

`#[serde(deny_unknown_fields)]` on all config structs. A typo like `max_client` or `stream_config` produces a clear parse error at startup — not a silent misconfiguration. No `--allow-unknown-config-keys` escape hatch in the initial implementation.

### Stream Override Merge

When both config file `stream_overrides` and CLI `--stream-*` flags are provided:
- Deep merge by stream ID: CLI field overrides config file field for the same stream
- CLI-only stream IDs are added
- Config-only stream IDs are preserved

### Thresholds Merge

When both config file `thresholds` map and CLI `--threshold` flags are provided:
- Merge by key: CLI key overrides config file key
- Config-only keys preserved

### Validation

All config values go through the same validation as CLI values (parse_mode, parse_pattern, parse_dscp, etc.) after merge. Invalid values produce clear errors with the field name and value.

### Server Address in Config

`server` is optional in the config file. If provided, the client can run with just `--config profile.yaml` (no positional arg). If both config and CLI provide server, CLI wins. If neither provides it, error at startup.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modify | Add `serde_yml` dependency |
| `src/config.rs` | Create | `ClientConfig`, `ServerFileConfig`, `StreamOverrideEntry`, YAML parsing, merge logic |
| `src/lib.rs` | Modify | Add `pub mod config;` |
| `src/bin/client.rs` | Modify | Add `--config` flag, load config, merge via value_source(), make server arg optional |
| `src/bin/server.rs` | Modify | Add `--config` flag, load config, merge via value_source() |

## Implementation Order

1. Add `serde_yml` to Cargo.toml
2. `src/config.rs` — config structs with deny_unknown_fields, YAML load, merge helpers, validation
3. Client integration — `--config` flag, value_source() merge, optional server arg
4. Server integration — `--config` flag, value_source() merge

## Testing

- [ ] YAML config file parsed correctly (all client fields)
- [ ] YAML server config parsed correctly
- [ ] Unknown YAML fields produce clear parse error (deny_unknown_fields)
- [ ] CLI flag overrides config file value (value_source = CommandLine)
- [ ] Config file value used when CLI not provided (value_source = DefaultValue)
- [ ] Built-in default used when neither CLI nor config provides value
- [ ] `server` from config file works (no positional arg needed)
- [ ] `server` from CLI overrides config file
- [ ] Missing server in both config and CLI produces clear error
- [ ] stream_overrides deep merge by ID works
- [ ] Thresholds merge by key works
- [ ] Invalid config values validated (bad mode, dscp, pattern)
- [ ] File not found produces clear error with path
- [ ] YAML syntax error produces clear error
- [ ] No --config = existing CLI-only behavior unchanged
- [ ] `cargo test` passes all existing + new tests
- [ ] End-to-end: `--config profile.yaml` runs complete test

## Not In Scope

- GUI config editor
- Config file hot-reloading during test
- Remote config fetch (HTTP/S3)
- Config file generation command
- TOML support (YAML only for now)
