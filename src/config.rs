// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

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

pub fn load_client_config(path: &Path) -> Result<ClientConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read config file '{}': {e}", path.display()))?;
    serde_yml::from_str(&content)
        .map_err(|e| format!("failed to parse config file '{}': {e}", path.display()))
}

pub fn load_server_config(path: &Path) -> Result<ServerFileConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read config file '{}': {e}", path.display()))?;
    serde_yml::from_str(&content)
        .map_err(|e| format!("failed to parse config file '{}': {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_client_config() {
        let yaml = r#"
server: 10.0.0.2:5000
mode: rrul
duration: 30
protocol: tcp
size: 512
rate: 100
pattern: fixed
cross_host: false
rrul_baseline: 5
rrul_ramp_up: 100
streams: 2
dscp: EF
ecn: ect0
client_id: subscriber-1
output: json
file: results.json
raw_file: packets.jsonl
thresholds:
  p99: 1000.0
  loss: 0.1
stream_overrides:
  - id: 0
    size: 64
    rate: 10000
    pattern: fixed
    dscp: AF41
  - id: 1
    size: 1518
"#;
        let cfg: ClientConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.server.as_deref(), Some("10.0.0.2:5000"));
        assert_eq!(cfg.mode.as_deref(), Some("rrul"));
        assert_eq!(cfg.duration, Some(30));
        assert_eq!(cfg.protocol.as_deref(), Some("tcp"));
        assert_eq!(cfg.size, Some(512));
        assert_eq!(cfg.rate, Some(100));
        assert_eq!(cfg.pattern.as_deref(), Some("fixed"));
        assert_eq!(cfg.cross_host, Some(false));
        assert_eq!(cfg.rrul_baseline, Some(5));
        assert_eq!(cfg.rrul_ramp_up, Some(100));
        assert_eq!(cfg.streams, Some(2));
        assert_eq!(cfg.dscp.as_deref(), Some("EF"));
        assert_eq!(cfg.ecn.as_deref(), Some("ect0"));
        assert_eq!(cfg.client_id.as_deref(), Some("subscriber-1"));
        assert_eq!(cfg.output.as_deref(), Some("json"));
        assert_eq!(cfg.file.as_deref(), Some("results.json"));
        assert_eq!(cfg.raw_file.as_deref(), Some("packets.jsonl"));

        let thresholds = cfg.thresholds.unwrap();
        assert_eq!(thresholds.get("p99"), Some(&1000.0));
        assert_eq!(thresholds.get("loss"), Some(&0.1));

        let overrides = cfg.stream_overrides.unwrap();
        assert_eq!(overrides.len(), 2);
        assert_eq!(overrides[0].id, 0);
        assert_eq!(overrides[0].size, Some(64));
        assert_eq!(overrides[0].rate, Some(10000));
        assert_eq!(overrides[0].pattern.as_deref(), Some("fixed"));
        assert_eq!(overrides[0].dscp.as_deref(), Some("AF41"));
        assert_eq!(overrides[1].id, 1);
        assert_eq!(overrides[1].size, Some(1518));
        assert_eq!(overrides[1].rate, None);
    }

    #[test]
    fn parse_valid_server_config() {
        let yaml = r#"
listen: 0.0.0.0:5000
output: json
file: server-results.json
raw_file: server-packets.jsonl
combined: true
max_clients: 4
timeout: 120
thresholds:
  p99: 1000.0
  loss: 0.1
"#;
        let cfg: ServerFileConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.listen.as_deref(), Some("0.0.0.0:5000"));
        assert_eq!(cfg.output.as_deref(), Some("json"));
        assert_eq!(cfg.file.as_deref(), Some("server-results.json"));
        assert_eq!(cfg.raw_file.as_deref(), Some("server-packets.jsonl"));
        assert_eq!(cfg.combined, Some(true));
        assert_eq!(cfg.max_clients, Some(4));
        assert_eq!(cfg.timeout, Some(120));

        let thresholds = cfg.thresholds.unwrap();
        assert_eq!(thresholds.get("p99"), Some(&1000.0));
        assert_eq!(thresholds.get("loss"), Some(&0.1));
    }

    #[test]
    fn deny_unknown_fields_client() {
        let yaml = r#"
server: 10.0.0.2:5000
unknown_field: bad
"#;
        let result: Result<ClientConfig, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown_field") || err.contains("unknown"),
            "error should mention the unknown field: {err}"
        );
    }

    #[test]
    fn deny_unknown_fields_server() {
        let yaml = r#"
listen: 0.0.0.0:5000
bad_key: true
"#;
        let result: Result<ServerFileConfig, _> = serde_yml::from_str(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn missing_file_error() {
        let result = load_client_config(Path::new("/nonexistent/path/config.yaml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("/nonexistent/path/config.yaml"), "error should contain path: {err}");
    }

    #[test]
    fn missing_file_error_server() {
        let result = load_server_config(Path::new("/nonexistent/path/server.yaml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("/nonexistent/path/server.yaml"), "error should contain path: {err}");
    }

    #[test]
    fn parse_minimal_config() {
        let yaml = "server: 10.0.0.1:5000\n";
        let cfg: ClientConfig = serde_yml::from_str(yaml).unwrap();
        assert_eq!(cfg.server.as_deref(), Some("10.0.0.1:5000"));
        assert!(cfg.mode.is_none());
        assert!(cfg.duration.is_none());
        assert!(cfg.thresholds.is_none());
        assert!(cfg.stream_overrides.is_none());
    }

    #[test]
    fn parse_empty_config() {
        let yaml = "{}\n";
        let cfg: ClientConfig = serde_yml::from_str(yaml).unwrap();
        assert!(cfg.server.is_none());
    }
}
