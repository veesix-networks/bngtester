# bngtester

BNG subscriber testing framework. Validates real subscriber connectivity across multiple platforms — Linux distributions, VyOS, OPNsense, pfSense, and other real-world subscriber devices.

**This project is fully LLM-driven.** All code, specs, and documentation are produced by AI agents following the workflow defined in [context/PROCESS.md](context/PROCESS.md). Human contributors are welcome — follow the same workflow process.

## Repository Layout

```
bngtester/
├── CLAUDE.md                    # This file (repo root — agent instructions)
├── README.md                    # Human-facing project overview
├── LICENSE                      # GPL-3.0-or-later
├── context/                     # AI workflow (agents read this, humans rarely need to)
│   ├── PROCESS.md               # Workflow phases, formats, rules
│   ├── SUMMARY.md               # Project state, dependency graph, key decisions
│   └── specs/                   # Implementation specs (one per issue)
│       └── <issue-number>-<slug>/
│           ├── README.md        # Status tracker (created Phase 1, updated every phase)
│           ├── IMPLEMENTATION_SPEC.md
│           ├── DECISIONS.md
│           ├── spec-reviews/    # Phases 2-3 output
│           │   ├── GEMINI.md
│           │   └── CODEX.md
│           └── code-reviews/    # Phase 6 output
│               ├── CLAUDE.md
│               ├── GEMINI.md
│               └── CODEX.md
├── Cargo.toml                   # Rust crate — traffic generation + metrics collection
├── src/
│   ├── lib.rs                   # Shared library code
│   ├── bin/
│   │   ├── server.rs            # Server binary (far side of BNG, receives + measures)
│   │   └── client.rs            # Client binary (runs in subscriber containers)
│   ├── traffic/                 # TCP/UDP/IMIX generators, packet structures
│   ├── metrics/                 # Timestamps, jitter, latency, loss calculation
│   └── report/                  # JUnit XML, JSON output
├── images/                      # Subscriber container images (custom-built)
│   ├── alpine/
│   │   └── Dockerfile
│   ├── debian/
│   │   └── Dockerfile
│   └── ubuntu/
│       └── Dockerfile
└── .github/
    ├── ISSUE_TEMPLATE/          # Issue templates (feature, bug, enhancement, testing)
    └── workflows/               # CI pipelines
```

## Starting Work

All work starts from a GitHub issue with the `approved` label. Read the issue via `gh issue view <number>`, then follow [context/PROCESS.md](context/PROCESS.md).

### Issue Types

| Template | Triggers Full Spec Workflow | Commit Type |
|----------|---------------------------|-------------|
| **Feature** | Yes | `feat` |
| **Enhancement** | Yes (unless trivial) | `feat` or `refactor` |
| **Bug** | No (unless complex — see issue fields) | `fix` |
| **Testing** | Yes | `test` |

## Contribution Rules

### Copyright Headers

Every new file must have the SPDX header.

**Go:**
```go
// Copyright The bngtester Authors
// Licensed under the GNU General Public License v3.0 or later.
// SPDX-License-Identifier: GPL-3.0-or-later
```

**Shell / YAML / Dockerfile:**
```
# Copyright The bngtester Authors
# Licensed under the GNU General Public License v3.0 or later.
# SPDX-License-Identifier: GPL-3.0-or-later
```

### Commit Messages

Conventional Commits format:

```
<type>[optional scope]: <description>
```

- Types: `feat`, `fix`, `docs`, `refactor`, `test`, `ci`, `chore`, `build`
- Scopes: match the area of the codebase being changed (e.g., `alpine`, `debian`, `collector`, `ci`, `spec`)
- Description: imperative mood, no capital first letter, no period

### Branch Naming

Branch prefix matches the commit type:

```
<type>/<scope>-<description>
```

Examples: `feat/alpine-subscriber-image`, `fix/entrypoint-vlan-race`, `test/dhcp-lease-validation`

### Code Style

- No unnecessary comments. Only comment when logic is genuinely non-obvious.
- Follow existing patterns. Read neighboring files before writing new ones.
- Keep Dockerfiles minimal — no unnecessary layers, no dev tools in production images.

### Scope

- **One feature per PR. One PR per issue.** No bundling unrelated changes.
- If implementation reveals a needed change outside the issue's scope, file a new issue for it.

### Workflow Output

**Every response that completes work must end with:**

```
Branch: <type>/<short-description>
Commit: <conventional commit message>
Files:
  - path/to/file1
  - path/to/file2
```
