# Implementation Spec: Configuration Reference Documentation

## Overview

Create comprehensive configuration reference documentation for bngtester-client and bngtester-server covering all CLI flags, YAML config file parameters, threshold keys, report formats, and common test scenario examples.

## Source Issue

[#45 — Configuration reference documentation](https://github.com/veesix-networks/bngtester/issues/45)

## Current State

- 27 client CLI flags and 11 server CLI flags.
- YAML config file support with all parameters.
- 5 example config files in `examples/`.
- No unified reference documentation — users must rely on `--help` output.

## Design

A single `docs/CONFIGURATION.md` file at the repo root `docs/` directory containing:

1. Client CLI reference
2. Server CLI reference
3. Config file reference (YAML)
4. Threshold keys reference
5. Report format documentation (JSON schema, JUnit structure, text)
6. Common test scenario cookbook

## File Plan

| File | Action | Purpose |
|------|--------|---------|
| `docs/CONFIGURATION.md` | Create | Complete configuration reference |

## Not In Scope

- API documentation
- Architecture documentation (covered in specs)
- Tutorial/getting-started guide
