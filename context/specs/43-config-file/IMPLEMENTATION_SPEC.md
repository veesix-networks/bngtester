# Implementation Spec: Config File Support (YAML Test Profiles)

## Overview

Add `--config <PATH>` flag to both bngtester-client and bngtester-server that loads test configuration from a YAML file. Config files define complete test profiles — streams, DSCP/ECN, packet sizes, rates, patterns, thresholds, and bind settings in a single file. CLI flags override config file values. This replaces the growing CLI flag sprawl (23 client flags, 9 server flags) with reusable, version-controllable test definitions.

## Source Issue

[#43 — Config file support (YAML/TOML test profiles)](https://github.com/veesix-networks/bngtester/issues/43)

## Current State

- Client has 23 CLI flags including per-stream overrides, DSCP, ECN, bind, thresholds.
- Server has 9 CLI flags including combined mode, max-clients, timeout.
- Complex test scenarios are unreadable as CLI invocations.
- No config file support — all configuration via CLI flags only.
- `serde` and `serde_json` already in deps. Need to add `serde_yaml`.

## Design

### Format Choice: YAML

YAML over TOML because:
- More natural for nested stream definitions
- Familiar to network engineers (Ansible, containerlab, osvbng all use YAML)
- Better support for lists of complex objects (stream overrides)

### Client Config File

```yaml
# bngtester client test profile
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

# Bind
bind_iface: eth1
source_ip: 10.255.0.2
control_bind_ip: 10.255.0.2

# Identity
client_id: subscriber-1

# Output
output: json
file: results.json
raw_file: packets.jsonl

# Thresholds
thresholds:
  p99: 1000
  loss: 0.1
  bloat: 3.0
  jitter: 100

# Per-stream overrides
streams_config:
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

# Histogram
histogram_buckets: "10,50,100,500,1000,5000,10000"
```

### Config Structs

```rust
#[derive(Debug, Deserialize, Default)]
pub struct ClientConfig {
    pub mode: Option<String>,
    pub duration: Option<u32>,
    pub protocol: Option<String>,
    pub size: Option<usize>,
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
    pub streams_config: Option<Vec<StreamConfigEntry>>,
}

#[derive(Debug, Deserialize)]
pub struct StreamConfigEntry {
    pub id: u8,
    pub size: Option<u32>,
    pub rate: Option<u32>,
    pub pattern: Option<String>,
    pub dscp: Option<String>,
}
```

All fields are `Option` — only specified fields override defaults. CLI flags override config file values.

### Merge Priority

```
CLI flags > Config file > Built-in defaults
```

Implementation: parse CLI first, then load config file, then merge with CLI taking precedence. For `Option` fields, CLI `Some` wins over config `Some`. For Vec fields (thresholds, streams), CLI values append/override config values.

### Error Handling

- File not found → clear error with path
- YAML parse error → error with line number and field name
- Unknown fields → warning (not error) to allow forward compatibility
- Invalid values (e.g., dscp: "INVALID") → same validation as CLI, error at startup

### Config Module Location

`src/config.rs` — config file parsing, merge logic, `ClientConfig` and `ServerConfig` structs.

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modify | Add `serde_yaml` dependency |
| `src/config.rs` | Create | `ClientConfig`, `ServerConfig`, `StreamConfigEntry`, YAML parsing, merge logic |
| `src/lib.rs` | Modify | Add `pub mod config;` |
| `src/bin/client.rs` | Modify | Add `--config` flag, load config, merge with CLI |
| `src/bin/server.rs` | Modify | Add `--config` flag, load config, merge with CLI |

## Implementation Order

1. Add `serde_yaml` to Cargo.toml
2. `src/config.rs` — config structs, YAML parsing, merge logic, unit tests
3. Client integration — `--config` flag, load + merge
4. Server integration — `--config` flag, load + merge

## Testing

- [ ] YAML config file parsed correctly (all fields)
- [ ] Missing optional fields produce defaults
- [ ] CLI flags override config file values
- [ ] Config file values override built-in defaults
- [ ] Per-stream config from YAML works (streams_config array)
- [ ] Thresholds from YAML applied correctly
- [ ] Unknown YAML fields produce warning, not error
- [ ] Invalid YAML produces clear error with line reference
- [ ] File not found produces clear error with path
- [ ] `--config` with no other flags works (full config from file)
- [ ] `--config` with CLI overrides works (merge)
- [ ] Server config file works (listen, combined, max_clients, timeout)
- [ ] No --config = existing CLI-only behavior unchanged
- [ ] `cargo test` passes all existing + new tests
- [ ] End-to-end: client with --config profile.yaml produces correct test

## Not In Scope

- GUI config editor
- Config file hot-reloading during test
- Remote config fetch (HTTP/S3)
- Config file generation command
- TOML support (YAML only for now)
