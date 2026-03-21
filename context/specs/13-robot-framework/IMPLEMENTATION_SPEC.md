# Implementation Spec: Robot Framework Test Runner

## Overview

Integrate Robot Framework as the test runner for bngtester subscriber images. Two test tiers: standalone tests (entrypoint validation, VLAN creation, cleanup — run with `docker run`, no BNG needed) and integration tests (IPoE through osvbng BNG — use the `lab/` containerlab topology from #27). Shared keyword libraries follow the osvbng `tests/common.robot` pattern. Image-matrix testing via Robot variable override. Structured XML/HTML output for CI consumption.

## Source Issue

[#13 — Add Robot Framework as test runner for subscriber integration tests](https://github.com/veesix-networks/bngtester/issues/13)

## Current State

- Subscriber images (Alpine, Debian, Ubuntu) exist with the shared entrypoint (`images/shared/entrypoint.sh`) supporting all access methods and encapsulation types.
- The containerlab topology (`lab/bngtester.clab.yml`) deploys osvbng with a bngtester subscriber and FRR server. `lab/smoke-test.sh` validates the path but uses shell scripts, not Robot Framework.
- osvbng uses Robot Framework extensively (`tests/common.robot`, `tests/rf-run.sh`, 18 test suites). Test 18 (`18-ipoe-linux-client`) uses bngtester subscriber images with the same entrypoint and env vars — this is the direct integration pattern.
- No Robot Framework infrastructure exists in the bngtester repo.

## Design

### Test Tiers

| Tier | Needs | Tests | CI-Friendly |
|------|-------|-------|-------------|
| **Standalone** | Docker only | Entrypoint validation, VLAN creation modes, SIGTERM cleanup | Yes — no containerlab/osvbng required |
| **Integration** | Docker + containerlab + osvbng image | DHCP lease through BNG, OSPF adjacency, gateway/server ping, BNG session API, iperf3 | Requires self-hosted runner |

Standalone tests can run in any CI environment with Docker. Integration tests require containerlab and the osvbng image — these run on self-hosted runners or locally.

### Directory Structure

```
tests/
├── common.robot                     # Shared keywords: containerlab, docker exec, health checks
├── subscriber.robot                 # Subscriber-specific keywords: VLAN checks, IP checks, ping
├── rf-run.sh                        # Test runner (venv setup, robot CLI)
├── 01-entrypoint-validation/        # Standalone: env var validation
│   └── 01-entrypoint-validation.robot
├── 02-vlan-modes/                   # Standalone: untagged, single, QinQ interface creation
│   └── 02-vlan-modes.robot
├── 03-cleanup/                      # Standalone: SIGTERM cleanup, partial failure
│   └── 03-cleanup.robot
├── 04-ipoe-bng/                     # Integration: IPoE through osvbng BNG
│   └── 04-ipoe-bng.robot
└── out/                             # Test output (git-ignored)
```

Numbered suite convention matches osvbng. Suites 01-03 are standalone, 04+ are integration.

### Shared Keywords

**`common.robot`** — adapted from osvbng's `tests/common.robot`:

| Keyword | Purpose | Source |
|---------|---------|--------|
| `Deploy Topology` | `sudo containerlab deploy -t <file> --reconfigure` | osvbng |
| `Destroy Topology` | Capture logs then `sudo containerlab destroy --cleanup` | osvbng |
| `Capture Container Logs` | Extract last 200 lines from each container via `docker logs` | osvbng |
| `Get Container IPv4` | `docker inspect` for container management IP | osvbng |
| `Wait For osvbng Healthy` | Retry loop for `"osvbng started successfully"` log marker | osvbng |
| `Execute VPP Command` | `docker exec vppctl -s /run/osvbng/cli.sock <cmd>` | osvbng |
| `Execute Vtysh On BNG` | `docker exec ip netns exec dataplane vtysh -c <cmd>` | osvbng |
| `Execute Vtysh On Router` | `docker exec vtysh -c <cmd>` | osvbng |
| `Get osvbng API Response` | `curl` to osvbng REST API | osvbng |
| `Verify OSPF Adjacency On Router` | Check `show ip ospf neighbor` for Full state | osvbng |
| `Run Container` | `docker run` with capabilities, env vars, network — for standalone tests | new |
| `Remove Container` | `docker rm -f` — cleanup for standalone tests | new |

**`subscriber.robot`** — bngtester-specific keywords:

| Keyword | Purpose |
|---------|---------|
| `Check Interface Exists` | `docker exec ip link show <iface>` |
| `Check Interface Has IPv4` | `docker exec ip -4 addr show <iface>`, assert `inet` present, exclude `169.254` |
| `Check Container Exited With Error` | `docker inspect` exit code > 0, log contains expected error |
| `Check Container Log Contains` | `docker logs` grep for expected message |
| `Ping From Container` | `docker exec ping -c 3 -W 2 <target>` |
| `Send Signal To Container` | `docker kill --signal <sig> <container>` |
| `Wait For Container Exit` | `docker wait` with timeout |
| `Run Subscriber Detached` | `docker run -d` with image, capabilities, env vars, network — returns container ID |
| `Wait For Interface In Container` | Poll `docker exec ip link show <iface>` with `Wait Until Keyword Succeeds`, fail if container exits first |

### Test Case Design

#### 01-entrypoint-validation (Standalone)

Tests that the entrypoint rejects invalid configuration with correct error messages. Uses `docker run` with `--network none` — no network connectivity needed.

| Test Case | Env Vars | Expected |
|-----------|----------|----------|
| Invalid ACCESS_METHOD | `ACCESS_METHOD=invalid` | Exit 1, error contains "Invalid ACCESS_METHOD" |
| Invalid ENCAP | `ENCAP=invalid` | Exit 1, error contains "Invalid ENCAP" |
| Missing CVLAN for single | `ENCAP=single` (no CVLAN) | Exit 1, error contains "CVLAN is required" |
| Missing SVLAN for QinQ | `ENCAP=qinq CVLAN=10` (no SVLAN) | Exit 1, error contains "SVLAN is required" |
| Missing PPPOE_USER | `ACCESS_METHOD=pppoe` (no user) | Exit 1, error contains "PPPOE_USER is required" |
| Missing PPPOE_PASSWORD | `ACCESS_METHOD=pppoe PPPOE_USER=test` | Exit 1, error contains "PPPOE_PASSWORD is required" |
| VLAN ID out of range | `ENCAP=single CVLAN=5000` | Exit 1, error contains "must be between 1 and 4094" |
| Interface name too long | `PHYSICAL_IFACE=longifacename ENCAP=qinq SVLAN=100 CVLAN=10` | Exit 1, error contains "max 15" |
| Missing NET_ADMIN capability | `ENCAP=single CVLAN=100` (no `--cap-add NET_ADMIN`) | Exit 1, log contains VLAN creation failure message |

#### 02-vlan-modes (Standalone)

Tests VLAN interface creation and access method dispatch. Uses `docker run -d` (detached) with `--cap-add NET_ADMIN`, a dedicated Docker network, and `DHCP_TIMEOUT=300` so the container stays alive while Robot polls for interface state.

**Harness:** Run the subscriber container detached. Use `Wait Until Keyword Succeeds` to poll `docker exec ip link show <iface>` while the container is running. Fail immediately if the container exits before the target interface appears. Stop and remove the container in test teardown.

| Test Case | Env Vars | Expected |
|-----------|----------|----------|
| Untagged mode | `ENCAP=untagged DHCP_TIMEOUT=300` | Entrypoint logs show target interface is `eth0` (no sub-interfaces created) |
| Single VLAN | `ENCAP=single CVLAN=100 DHCP_TIMEOUT=300` | `eth0.100` interface created, visible via `ip link show` |
| QinQ | `ENCAP=qinq SVLAN=100 CVLAN=10 DHCP_TIMEOUT=300` | `eth0.100` (802.1ad) and `eth0.100.10` interfaces created |
| DHCPv6 dispatch | `ACCESS_METHOD=dhcpv6 ENCAP=untagged DHCP_TIMEOUT=300` | Container logs show "Starting DHCPv6" — dhcpcd -6 or dhclient -6 launched |
| PPPoE dispatch | `ACCESS_METHOD=pppoe ENCAP=untagged PPPOE_USER=test PPPOE_PASSWORD=test` | Container logs show "Starting PPPoE" — pppd process launched |

**Host requirement:** The Docker host must have `8021q` kernel module loaded for VLAN tests. QinQ additionally requires `8021ad` (typically built into the `8021q` module on modern kernels).

#### 03-cleanup (Standalone)

Tests cleanup behavior on signal and failure. Uses `docker run -d` with a dedicated network. Verification is **log-based** — once the container exits, its network namespace is destroyed by Docker, so we cannot inspect interface state post-exit. Instead, we verify: (1) the entrypoint logs cleanup messages in the correct order, (2) the container exits with the expected code.

| Test Case | Scenario | Expected |
|-----------|----------|----------|
| SIGTERM cleanup (QinQ) | Start detached with QinQ + `DHCP_TIMEOUT=300`, send SIGTERM | Exit code 143, logs contain cleanup in reverse order (C-VLAN removed, then S-VLAN) |
| SIGTERM cleanup (single) | Start detached with single VLAN + `DHCP_TIMEOUT=300`, send SIGTERM | Exit code 143, logs show VLAN interface removal |
| DHCP timeout exit | Start detached with `DHCP_TIMEOUT=5` (very short, no DHCP server) | Container exits after ~5s, logs show DHCP timeout, cleanup messages present |

**Limitation:** This verifies cleanup *attempted* via log evidence, not cleanup *succeeded* via namespace inspection. A future enhancement could use a shared-namespace observer container to verify interface removal directly. This is documented as a known limitation.

#### 04-ipoe-bng (Integration)

Tests IPoE subscriber through the osvbng BNG using the `lab/` topology. This is the Robot Framework equivalent of `lab/smoke-test.sh` — structured, with proper reporting. **All test cases tagged `integration`** so they can be excluded from CI runs: `--exclude integration`.

| Test Case | Check |
|-----------|-------|
| BNG Is Healthy | `Wait For osvbng Healthy` |
| OSPF Adjacency Established | Server sees bng1 as Full neighbor |
| Subscriber QinQ Interface Created | `eth1.100.10` exists on subscriber |
| Subscriber Got IPv4 Via DHCP | Non-link-local IPv4 on `eth1.100.10` |
| Session In BNG API | `/api/show/subscriber/sessions` shows >= 1 session |
| Subscriber Can Ping Gateway | Ping `10.255.0.1` from subscriber |
| Subscriber Can Ping Server Through BNG | Ping `10.0.0.2` from subscriber |
| Iperf3 Throughput | iperf3 from subscriber to server (informational) |

**Suite Setup:** Sets `BNGTESTER_IMAGE` env var from `${SUBSCRIBER_IMAGE}`, then calls `Deploy Topology` with `lab/bngtester.clab.yml`. The `--reconfigure` flag means it will take over an existing lab with the same name. `sudo -E` is used to preserve the image override env vars.

**Suite Teardown:** Calls `Destroy Topology` which captures container logs then runs `clab destroy --cleanup`.

**Lab ownership:** This suite owns the `bngtester` lab for the duration of its run. If the lab is already deployed (e.g., for debugging), `--reconfigure` will redeploy it. Users should be aware the suite destroys the lab on teardown.

### Image Matrix

All test suites accept a `${SUBSCRIBER_IMAGE}` variable (default: `veesixnetworks/bngtester:alpine-latest`). Run with different images via:

```bash
./tests/rf-run.sh tests/02-vlan-modes/ --variable SUBSCRIBER_IMAGE:veesixnetworks/bngtester:debian-latest
```

For integration tests, the variable maps to `BNGTESTER_IMAGE` in the containerlab topology.

### Test Runner

`tests/rf-run.sh` — adapted from osvbng's `tests/rf-run.sh`:

- **Preflight checks:** Verifies `docker info` succeeds (Docker running), checks `${SUBSCRIBER_IMAGE}` is available (`docker image inspect`), and for integration tests checks `command -v containerlab` and `sudo -n true`. Exits with a clear error message if any check fails.
- Creates Python venv at `tests/.venv/` with `robotframework>=7.0`
- Runs `robot` with output to `tests/out/`
- Log naming: `{suite-dir}-log.html`, `{suite-dir}-out.xml`
- Accepts extra robot args (e.g., `--variable`, `--include`, `--exclude`)
- Usage: `./tests/rf-run.sh tests/01-entrypoint-validation/`

## Configuration

### Robot Framework Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `${SUBSCRIBER_IMAGE}` | `veesixnetworks/bngtester:alpine-latest` | Subscriber image for all tests |
| `${CLAB_BIN}` | `sudo containerlab` | Containerlab binary (integration tests) |
| `${OSVBNG_API_PORT}` | `8080` | osvbng REST API port (integration tests) |
| `${VPPCTL_SOCK}` | `/run/osvbng/cli.sock` | VPP CLI socket (integration tests) |

### Dependencies

- Python >= 3.10
- `robotframework >= 7.0`
- Docker (all tests)
- `8021q` kernel module loaded on Docker host (VLAN tests)
- containerlab (integration tests only)
- osvbng image (integration tests only)
- Hugepages configured on host (integration tests only — osvbng/VPP requirement)

## File Plan

### New Files

| File | Purpose |
|------|---------|
| `tests/common.robot` | Shared keywords: containerlab, docker exec, osvbng health, VPP/FRR commands |
| `tests/subscriber.robot` | Subscriber keywords: interface checks, IP checks, ping, signal, container lifecycle |
| `tests/rf-run.sh` | Test runner script (venv, robot CLI, output management) |
| `tests/.gitignore` | Ignore `.venv/` and `out/` |
| `tests/01-entrypoint-validation/01-entrypoint-validation.robot` | Entrypoint env var validation tests (9 cases incl. missing capability) |
| `tests/02-vlan-modes/02-vlan-modes.robot` | VLAN creation + access method dispatch tests (5 cases: untagged, single, QinQ, DHCPv6, PPPoE) |
| `tests/03-cleanup/03-cleanup.robot` | SIGTERM and timeout cleanup tests (3 cases, log-based verification) |
| `tests/04-ipoe-bng/04-ipoe-bng.robot` | IPoE through BNG integration tests (8 cases, tagged `integration`) |
| `context/specs/13-robot-framework/IMPLEMENTATION_SPEC.md` | This spec |
| `context/specs/13-robot-framework/README.md` | Status tracker |

### Modified Files

None.

## Implementation Order

### Phase A: Infrastructure

Create the test runner and shared keyword libraries:

1. `tests/rf-run.sh` — adapted from osvbng
2. `tests/common.robot` — containerlab and osvbng keywords from osvbng's common.robot + new standalone container keywords
3. `tests/subscriber.robot` — bngtester-specific keywords
4. `tests/.gitignore` — ignore `.venv/` and `out/`

**Testable:** `./tests/rf-run.sh --help` or `robot --version` works after venv setup.

### Phase B: Standalone Tests (01-03)

Create the three standalone test suites:

1. `tests/01-entrypoint-validation/01-entrypoint-validation.robot`
2. `tests/02-vlan-modes/02-vlan-modes.robot`
3. `tests/03-cleanup/03-cleanup.robot`

**Testable:** `./tests/rf-run.sh tests/01-entrypoint-validation/` passes with Docker only.

### Phase C: Integration Test (04)

Create the BNG integration test suite:

1. `tests/04-ipoe-bng/04-ipoe-bng.robot`

**Testable:** `./tests/rf-run.sh tests/04-ipoe-bng/` passes with containerlab + osvbng.

### Phase D: Image Matrix Validation

Run all suites with each subscriber image to verify matrix support:

- Alpine: `./tests/rf-run.sh tests/01-entrypoint-validation/`
- Debian: `./tests/rf-run.sh tests/01-entrypoint-validation/ --variable SUBSCRIBER_IMAGE:veesixnetworks/bngtester:debian-latest`
- Ubuntu: `./tests/rf-run.sh tests/01-entrypoint-validation/ --variable SUBSCRIBER_IMAGE:veesixnetworks/bngtester:ubuntu-latest`

**Testable:** All three images produce passing results for suites 01-03.

## Testing

### Standalone Tests (CI-compatible)

```bash
# Run all standalone suites
./tests/rf-run.sh tests/01-entrypoint-validation/
./tests/rf-run.sh tests/02-vlan-modes/
./tests/rf-run.sh tests/03-cleanup/

# Image matrix
for img in alpine debian ubuntu; do
    ./tests/rf-run.sh tests/01-entrypoint-validation/ \
        --variable SUBSCRIBER_IMAGE:veesixnetworks/bngtester:${img}-latest
done
```

### Integration Tests (requires containerlab + osvbng)

```bash
# Uses lab/ topology, deploys/destroys automatically
OSVBNG_IMAGE=veesixnetworks/osvbng:local ./tests/rf-run.sh tests/04-ipoe-bng/
```

### Output

- `tests/out/{suite}-log.html` — human-readable HTML report
- `tests/out/{suite}-out.xml` — Robot XML for CI parsing

## Not In Scope

- **osvbng-side Robot integration** — osvbng has its own test suites; interop is an osvbng issue. `tests/common.robot` documents the shared keyword interface that osvbng tests can import (same keyword signatures as osvbng's `common.robot`) — this satisfies the acceptance criterion for a documented interop point.
- **Rust collector integration** — depends on #5; collector-specific test cases will be added then
- **Performance/load testing** — single subscriber validation only
- **CI workflow** — the test suite is runnable locally and in CI, but the GitHub Actions workflow is a separate issue (requires self-hosted runner for integration tests)
- **PPPoE through BNG** — the standalone tests validate PPPoE entrypoint validation, but PPPoE termination through osvbng is a follow-up (osvbng PPPoE config is more complex)
- **DHCPv6 through BNG** — the lab topology is IPv4-only per #27
