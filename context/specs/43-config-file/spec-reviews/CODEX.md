# Spec Critique: Config File Support (#43)

Reusable test profiles are a good direction, but the current spec still leaves the hardest parts either wrong or underspecified: how CLI/default/config precedence actually works with the current `clap` setup, whether the YAML dependency is viable, and how strict the schema should be. Those need to be tightened in Phase 4 before implementation starts.

## Findings

### HIGH: the merge algorithm does not work against the current `clap` derive structs as written

- The spec says all config fields are `Option` and that implementation should "parse CLI first" with CLI `Some` winning over config `Some` in `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:107-153`.
- The current CLI structs do not look like that. Most client fields are concrete values with `default_value` in `src/bin/client.rs:33-115`, and server does the same in `src/bin/server.rs:35-69`.
- If implementation just follows the existing `Cli::parse()` path, clap has already materialized defaults like `size=512`, `duration=30`, `output=text`, `max_clients=1`, and `timeout=300`. A config-only invocation like `bngtester-client --config profile.yaml` would therefore keep clap defaults instead of letting the YAML supply those values.
- Plain bool flags have the same problem. `cross_host` and `combined` are `bool` today, so an absent flag is indistinguishable from the default `false` value after parsing.
- clap 4's derive API can support provenance-aware merging, but only if the spec says so explicitly: `Parser` also provides `CommandFactory` and `FromArgMatches`, and `ArgMatches::value_source()` distinguishes `CommandLine` from `DefaultValue` ([Parser docs](https://docs.rs/clap/latest/clap/trait.Parser.html), [ValueSource docs](https://docs.rs/clap/latest/clap/parser/enum.ValueSource.html)).
- Phase 4 should pick one concrete implementation shape:
  - keep derive, but merge from `ArgMatches` using `value_source()` before converting into the typed CLI struct; or
  - replace the defaulted `Cli` structs with raw overlay structs (`Option<T>` / repeatable vectors) and move built-in defaults into a post-merge resolved config layer.
- The generic vector merge note is also too loose for per-stream overrides. `thresholds` merges by threshold key, but `streams_config` needs a keyed merge by `stream id` plus field, not blind list append.
- Add explicit tests for config-only defaulted scalars and bools, not just the general "CLI overrides config" case. Example: YAML `size: 1518` with no `--size`, or YAML `combined: true` with no `--combined`.

### HIGH: the spec hard-codes `serde_yaml`, but the published `serde_yaml` crate is deprecated and the original upstream is archived

- The spec says to add `serde_yaml` in `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:17` and `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:170`.
- As of March 25, 2026, the official crate page is `serde_yaml 0.9.34+deprecated`, and the original upstream repository `dtolnay/serde-yaml` is archived with the README stating that the project is no longer maintained ([docs.rs](https://docs.rs/crate/serde_yaml/latest/source/README.md), [GitHub](https://github.com/dtolnay/serde-yaml)).
- That does not automatically make YAML the wrong format, but it does mean the spec should not treat `serde_yaml` as an uncontroversial dependency choice for a new user-facing config surface.
- Phase 4 should either switch to a maintained YAML/Serde crate explicitly or document why the project is willing to accept an unmaintained dependency here. The error-handling section should then describe the chosen library's actual behavior rather than assuming `serde_yaml`.

### MEDIUM: `streams_config` breaks the existing `--stream-*` naming pattern and is easy to confuse with both `streams` and the existing `stream_config` field

- The YAML example and config struct use `streams_config` in `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:69-80` and `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:131-132`.
- The current CLI surface is singular and per-field: `--stream-dscp`, `--stream-size`, `--stream-rate`, `--stream-pattern` in `src/bin/client.rs:93-107`.
- The protocol already uses `stream_config` as the field name for per-stream overrides in `src/protocol/mod.rs:37-40`.
- That leaves three near-miss names around the same concept: `streams`, `streams_config`, and `stream_config`. In a handwritten YAML file, that is a likely typo source.
- If the plural form is intentional, the spec should explain why. Otherwise I would rename it before implementation to something that matches the rest of the surface area, preferably `stream_config` or `stream_overrides`.

### MEDIUM: unknown config keys should be strict-rejected by default, not warn-only

- The spec currently chooses warnings for unknown fields in `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:155-160` and tests for that behavior in `context/specs/43-config-file/IMPLEMENTATION_SPEC.md:191`.
- For a test tool, silent or easy-to-miss config drift is the bigger risk than forward compatibility. A typo like `stream_config`, `max_client`, or `histogram_bucket` should stop startup, not run a different test than the operator intended.
- Strict rejection is also the simpler implementation path. Serde can enforce it on each config struct, while "warn but continue" requires custom unknown-key detection across top-level and nested mappings.
- If forward compatibility matters, make it explicit and opt-in, for example via a schema version and an `--allow-unknown-config-keys` escape hatch. The default behavior should be reject-on-unknown for both top-level and nested config objects.
