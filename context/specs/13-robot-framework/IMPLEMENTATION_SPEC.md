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
| `Check Interface Removed` | `docker exec ip link show <iface>` returns non-zero |

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

#### 02-vlan-modes (Standalone)

Tests VLAN interface creation. Uses `docker run` with `--cap-add NET_ADMIN` and a dedicated Docker network (veth provides the physical interface).

| Test Case | Env Vars | Expected |
|-----------|----------|----------|
| Untagged mode | `ENCAP=untagged` | `eth0` used directly as target (no sub-interfaces) |
| Single VLAN | `ENCAP=single CVLAN=100` | `eth0.100` interface created |
| QinQ | `ENCAP=qinq SVLAN=100 CVLAN=10` | `eth0.100` (802.1ad) and `eth0.100.10` interfaces created |

Note: These tests verify interface creation only. DHCP will timeout (no server), so the container will exit with a DHCP error — tests check for interface existence before that happens using a sidecar check or by inspecting the container state after a short delay.

#### 03-cleanup (Standalone)

Tests cleanup behavior on signal and failure. Uses `docker run` with a dedicated network.

| Test Case | Scenario | Expected |
|-----------|----------|----------|
| SIGTERM cleanup (QinQ) | Start with QinQ, send SIGTERM | Container exits, VLAN interfaces removed from network namespace |
| SIGTERM cleanup (single) | Start with single VLAN, send SIGTERM | Container exits, VLAN interface removed |
| DHCP timeout exit | Start with no DHCP server, wait for timeout | Container exits after DHCP_TIMEOUT, interfaces cleaned up |

#### 04-ipoe-bng (Integration)

Tests IPoE subscriber through the osvbng BNG using the `lab/` topology. This is the Robot Framework equivalent of `lab/smoke-test.sh` — structured, with proper reporting.

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

Suite Setup deploys the lab topology (with `BNGTESTER_IMAGE` variable for image matrix). Suite Teardown destroys it.

### Image Matrix

All test suites accept a `${SUBSCRIBER_IMAGE}` variable (default: `veesixnetworks/bngtester:alpine-latest`). Run with different images via:

```bash
./tests/rf-run.sh tests/02-vlan-modes/ --variable SUBSCRIBER_IMAGE:veesixnetworks/bngtester:debian-latest
```

For integration tests, the variable maps to `BNGTESTER_IMAGE` in the containerlab topology.

### Test Runner

`tests/rf-run.sh` — adapted from osvbng's `tests/rf-run.sh`:

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
- containerlab (integration tests only)
- osvbng image (integration tests only)

## File Plan

### New Files

| File | Purpose |
|------|---------|
| `tests/common.robot` | Shared keywords: containerlab, docker exec, osvbng health, VPP/FRR commands |
| `tests/subscriber.robot` | Subscriber keywords: interface checks, IP checks, ping, signal, container lifecycle |
| `tests/rf-run.sh` | Test runner script (venv, robot CLI, output management) |
| `tests/.gitignore` | Ignore `.venv/` and `out/` |
| `tests/01-entrypoint-validation/01-entrypoint-validation.robot` | Entrypoint env var validation tests (8 cases) |
| `tests/02-vlan-modes/02-vlan-modes.robot` | VLAN interface creation tests: untagged, single, QinQ (3 cases) |
| `tests/03-cleanup/03-cleanup.robot` | SIGTERM and timeout cleanup tests (3 cases) |
| `tests/04-ipoe-bng/04-ipoe-bng.robot` | IPoE through BNG integration tests (8 cases) |
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

- **osvbng-side Robot integration** — osvbng has its own test suites; interop is an osvbng issue
- **Rust collector integration** — depends on #5; collector-specific test cases will be added then
- **Performance/load testing** — single subscriber validation only
- **CI workflow** — the test suite is runnable locally and in CI, but the GitHub Actions workflow is a separate issue (requires self-hosted runner for integration tests)
- **PPPoE through BNG** — the standalone tests validate PPPoE entrypoint validation, but PPPoE termination through osvbng is a follow-up (osvbng PPPoE config is more complex)
- **DHCPv6 through BNG** — the lab topology is IPv4-only per #27
