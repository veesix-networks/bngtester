# Decisions: 13-robot-framework

## Accepted

### Missing NET_ADMIN capability test case
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Added test case to 01-entrypoint-validation: run subscriber with `ENCAP=single CVLAN=100` but without `--cap-add NET_ADMIN`. Expect exit 1 with VLAN creation failure in logs.

### DHCPv6 and PPPoE dispatch test cases
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Added DHCPv6 and PPPoE test cases to 02-vlan-modes. DHCPv6 verifies dhcpcd/dhclient -6 starts (log contains "Starting DHCPv6"). PPPoE verifies pppd launches (log contains "Starting PPPoE"). These are access method dispatch tests — actual lease/session requires a server, covered by the integration tier.

### Detached container harness for 02-vlan-modes
- **Source:** GEMINI, CODEX
- **Severity:** HIGH (CODEX), MEDIUM (GEMINI)
- **Resolution:** Replaced vague "sidecar check or short delay" with concrete harness: run containers detached (`docker run -d`) with `DHCP_TIMEOUT=300`, poll `docker exec ip link show <iface>` using `Wait Until Keyword Succeeds`, fail if container exits before interface appears. Added `Run Subscriber Detached` and `Wait For Interface In Container` keywords to subscriber.robot.

### Log-based cleanup verification for 03-cleanup
- **Source:** GEMINI, CODEX
- **Severity:** HIGH (CODEX), MEDIUM (GEMINI)
- **Resolution:** Replaced interface-removal assertions with log-based verification: check entrypoint logs for cleanup messages in correct order + verify exit code. Once the container exits, Docker destroys the network namespace — `docker exec` cannot observe post-cleanup state. Documented as a known limitation with a path forward (shared-namespace observer).

### Integration test lab lifecycle and tagging
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Suite 04 tests tagged `integration` (excludable via `--exclude integration`). Documented lab ownership contract: suite uses `--reconfigure` and destroys on teardown. `sudo -E` required for image override. Users warned that running the suite will take over and destroy the existing `bngtester` lab.

### Runner preflight checks
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Added preflight checks to rf-run.sh spec: verify Docker running (`docker info`), subscriber image available (`docker image inspect`), containerlab installed (for integration), sudo passwordless access. Clear error messages on failure.

### 8021q kernel module dependency
- **Source:** GEMINI
- **Severity:** LOW
- **Resolution:** Added `8021q` kernel module to dependencies section. Host requirement note added to 02-vlan-modes test design.

### Interop documentation
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** `tests/common.robot` uses the same keyword signatures as osvbng's `common.robot` — this IS the documented interop point. Added note to "Not In Scope" section clarifying that the shared keyword interface satisfies the acceptance criterion.

## Rejected

### Redundant config validation test
- **Source:** GEMINI
- **Severity:** LOW
- **Rationale:** The entrypoint intentionally ignores `CVLAN`/`SVLAN` when `ENCAP=untagged` — they are simply unused. This is correct behavior, not a bug or ambiguity. Adding a test for this would be testing a non-requirement.

### Unique per-run lab name for integration tests
- **Source:** CODEX
- **Severity:** HIGH (part of finding #4)
- **Rationale:** The lab name is fixed to `bngtester` to match the topology file and keep container names predictable (`clab-bngtester-*`). Dynamic lab names would require topology templating and break the documented manual inspection commands. The `--reconfigure` flag and documented ownership contract are sufficient. If concurrent lab runs become a requirement, that's a separate issue.

### Remove integration test (04) from this issue
- **Source:** CODEX
- **Severity:** HIGH (part of finding #1)
- **Rationale:** The issue says "BNG-in-the-loop end-to-end tests" are out of scope, but also requires "DHCP lease acquisition" testing. The lab topology (#27) is merged and provides immediate value. Suite 04 is tagged `integration` and excludable from CI, so it doesn't burden standalone users. Removing it would leave the lab topology untested by Robot and delay the tooling that #5 needs.
