# Decisions: 27-containerlab-topology

## Accepted

### LCP and dataplane namespace must be explicit in osvbng.yaml
- **Source:** GEMINI
- **Severity:** HIGH
- **Resolution:** Added `dataplane` section with `lcp-netns: dataplane` to the required osvbng.yaml configuration table. Without this, VPP cannot sync interfaces into the Linux control plane namespace where FRR and DHCP operate.

### osvbng.yaml contract must enumerate all required sections
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Replaced the bullet-list description of osvbng configuration with a detailed table naming all required sections (interfaces, subscriber-groups, ipv4-profiles, dhcp, aaa, plugins, protocols.ospf, dataplane, logging) with their key settings. This eliminates ambiguity for implementation.

### Smoke test needs concrete timeouts and failure diagnostics
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Replaced the generic smoke test description with a stage-by-stage table specifying timeouts (120s for osvbng, 90s for DHCP, 60s for OSPF) and diagnostic dumps on failure (container logs, interface state, route tables, OSPF neighbor state).

### OSPF adjacency must be validated in smoke test
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Added Stage 4 to the smoke test: verify OSPF adjacency is established (server sees bng1 as Full neighbor) within 60s. The static route remains as a convergence fallback but the smoke test independently validates OSPF, so a broken OSPF config will not pass silently.

### OSPF passive loopbacks
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Updated the osvbng OSPF configuration to include `loop0` and `loop100` as passive interfaces. Updated server FRR config description to include loopback `10.254.0.2/32` as passive.

### Server node must provision iperf3
- **Source:** GEMINI, CODEX
- **Severity:** MEDIUM (CODEX), LOW (GEMINI)
- **Resolution:** Added iperf3 installation (`apk add`) to the server entrypoint.sh spec. The FRR image is Alpine-based, so iperf3 is available via apk. The entrypoint installs it and starts `iperf3 -s -D` so it is ready without manual intervention. The iperf3 smoke test stage is informational (non-fatal).

### Rust collector current state description
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Reworded the "Current State" section to clarify that the Rust collector spec is finalized but binaries do not exist on `main` yet, avoiding reference to files only present on an unmerged branch.

### QinQ MTU overhead documentation
- **Source:** GEMINI
- **Severity:** LOW
- **Resolution:** Added a note to the `lab/README.md` file plan entry to include QinQ MTU overhead (8 bytes) in the troubleshooting section. Containerlab veth pairs default to 1500 MTU, so effective subscriber MTU is 1492.

## Rejected

### IPv6 future-proofing note
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Rationale:** The "Not In Scope" section already states "IPv6 subscriber testing — IPoE DHCPv4 only in this issue." The topology is inherently extensible to IPv6 by adding ipv6-profiles to osvbng.yaml — no architectural decisions in this spec constrain dual-stack. Adding a future-proofing note would be speculative and is an anti-pattern per the workflow (generic recommendations).
