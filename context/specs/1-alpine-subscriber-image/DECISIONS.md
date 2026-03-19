# Decisions: 1-alpine-subscriber-image

## Amendments

### DHCP client auto-detection replaces Alpine-only dispatch
- **Source:** AMENDMENT (human intervention)
- **Phase:** Phase 4
- **Resolution:** Changed dispatch functions from Alpine-only (dhcpcd) to auto-detecting at runtime (`command -v dhcpcd` / `command -v dhclient`). Both client implementations are built into the shared entrypoint. Eliminates the need for per-image overrides or a separate abstraction layer.

### bng-client noted as long-term entrypoint replacement
- **Source:** AMENDMENT (human intervention)
- **Phase:** Phase 4
- **Resolution:** Added design note that the planned Rust `bng-client` binary will eventually replace the shell entrypoint entirely. The current entrypoint is the minimum viable approach before the Rust tooling exists.

## Accepted

### pppd must run with `nodetach` and standard session flags
- **Source:** GEMINI (findings 5, 6, 8)
- **Severity:** HIGH
- **Resolution:** Added `nodetach`, `noauth`, `defaultroute`, `usepeerdns`, and explicit `user`/`password` passthrough to the pppd command specification in the Design section.

### Entrypoint must handle PID 1 signal semantics correctly
- **Source:** GEMINI (findings 7, 11)
- **Severity:** HIGH
- **Resolution:** Specified that PPPoE uses `exec pppd` (replaces shell, pppd becomes PID 1). DHCP methods use `trap` + `wait` pattern with explicit `dhcpcd -k` / `kill` in the cleanup path.

### Define DHCP client dispatch interface for cross-image reuse
- **Source:** CODEX (finding 1), GEMINI (finding 14)
- **Severity:** HIGH
- **Resolution:** Added a dispatch mechanism using shell functions (`start_dhcpv4`, `start_dhcpv6`, `start_pppoe`, `stop_client`) in the entrypoint. Alpine implementations call `dhcpcd`/`pppd` directly. Future images override these functions or set `DHCP_CLIENT` to dispatch to `dhclient`. This establishes the shared contract without building unused code paths.

### Define access-acquisition failure and readiness contract
- **Source:** CODEX (finding 2), GEMINI (finding 10)
- **Severity:** HIGH
- **Resolution:** Added a "Failure and Readiness Contract" subsection to Design. Each access method defines: what counts as success, timeout behavior, and exit code semantics. dhcpcd runs in foreground mode (`-B` removed, use `-f` for foreground) with `-t` for lease timeout. PPPoE uses `persist` + `maxfail`. Non-zero exit from the client = non-zero container exit.

### Document runtime network-attachment model
- **Source:** CODEX (finding 3)
- **Severity:** HIGH
- **Resolution:** Added "Runtime Network Model" subsection to Design. Documented that subscribers require a dedicated interface into the BNG-facing network (`--network none` + injected veth/macvlan, or a dedicated Docker/podman network). Removed misleading "basic docker run should work" claim from Phase B. Updated examples to show `--network` usage.

### Ensure physical interface is UP and check operstate before VLAN creation
- **Source:** GEMINI (finding 4), CODEX (finding 4)
- **Severity:** MEDIUM
- **Resolution:** Updated entrypoint flow: after interface presence check, explicitly `ip link set $PHYSICAL_IFACE up` and poll for `operstate` = `up` or `unknown` (some drivers report `unknown` when carrier is present) before proceeding to VLAN creation.

### Validate interface name length for derived VLAN names
- **Source:** CODEX (finding 5)
- **Severity:** MEDIUM
- **Resolution:** Added validation step: compute derived interface name, check it is <= 15 bytes. Exit with a clear error if exceeded. No short-name scheme — reject early with actionable message.

### Idempotent cleanup on all exit paths, not just signals
- **Source:** CODEX (finding 6)
- **Severity:** MEDIUM
- **Resolution:** Single `cleanup()` function registered via `trap ... EXIT`. Runs on signal, on error, and on normal exit. Removes VLAN interfaces in reverse creation order, kills client processes, releases DHCP leases. Idempotent — safe to call multiple times.

### Handle VLAN capability and kernel module failures
- **Source:** CODEX (finding 7)
- **Severity:** MEDIUM
- **Resolution:** After each `ip link add`, check exit code. On failure, emit a diagnostic message pointing at missing 8021q/8021ad kernel module or missing NET_ADMIN capability. Exit non-zero. Added negative test case.

### Clean DHCP lease release on exit
- **Source:** GEMINI (finding 9)
- **Severity:** MEDIUM
- **Resolution:** Cleanup function runs `dhcpcd -k $TARGET_IFACE` (or equivalent for dhclient) before removing interfaces. Merged into the idempotent cleanup path.

### Validate PPPOE_USER and PPPOE_PASSWORD when ACCESS_METHOD=pppoe
- **Source:** GEMINI (finding 12)
- **Severity:** MEDIUM
- **Resolution:** Added to validation step: if `ACCESS_METHOD=pppoe`, require `PPPOE_USER` and `PPPOE_PASSWORD` to be non-empty. Exit with clear error if missing.

### Validate VLAN ID range (1-4094)
- **Source:** GEMINI (finding 13)
- **Severity:** LOW
- **Resolution:** Added numeric range validation for SVLAN and CVLAN (1-4094). Exit with error if out of range.

### Use `apk add --no-cache` in Dockerfile
- **Source:** GEMINI (finding 1)
- **Severity:** LOW
- **Resolution:** Specified `--no-cache` flag in the Dockerfile package install step.

## Rejected

### Pin Alpine package versions
- **Source:** GEMINI (finding 1, partial)
- **Severity:** LOW
- **Rationale:** Alpine's package repositories do not retain old versions. Pinning breaks builds when the pinned version is removed from the repo. `--no-cache` is sufficient for reproducibility at the layer level.

### QinQ protocol verification
- **Source:** GEMINI (finding 3)
- **Severity:** MEDIUM
- **Rationale:** Informational finding — confirms the spec is already correct. S-VLAN uses `protocol 802.1ad`, C-VLAN defaults to `802.1Q`. No change needed.

### Layer ordering recommendation
- **Source:** GEMINI (finding 2)
- **Severity:** LOW
- **Rationale:** Already the planned order in the spec (install packages before COPY entrypoint). No change needed.

### dhcpcd `-t 0` for indefinite wait
- **Source:** GEMINI (finding 10)
- **Severity:** LOW
- **Rationale:** Superseded by the failure contract (C2 accepted). Indefinite wait is the wrong default — a bounded timeout with non-zero exit is needed for CI observability. The failure contract defines explicit timeout behavior per access method.
