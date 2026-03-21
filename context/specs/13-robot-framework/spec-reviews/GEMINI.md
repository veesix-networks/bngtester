# Spec Review: Robot Framework Test Runner (Gemini)

## Overview
This review covers the integration of Robot Framework as the test runner for `bngtester`. The design effectively leverages patterns from the `osvbng` project while addressing the specific needs of subscriber image validation through both standalone and integration tests.

## Findings

| Severity | Title | Description |
|----------|-------|-------------|
| **MEDIUM** | Race condition in standalone VLAN checks | `02-vlan-modes` tests might fail if the container exits due to DHCP timeout before Robot can `docker exec` into it to verify interface existence. |
| **MEDIUM** | Cleanup verification strategy | Verifying that `entrypoint.sh` successfully removes VLAN interfaces after container exit (Suite 03) is difficult if the network namespace is destroyed by Docker immediately. |
| **LOW** | Missing 8021q dependency in spec | The spec does not explicitly mention that the host running standalone tests must have the `8021q` and `802.1ad` kernel modules loaded. |
| **LOW** | Redundant config validation | `entrypoint.sh` allows `CVLAN` to be set when `ENCAP=untagged` without warning or error, which might lead to confusing test results. |

## Detailed Analysis

### 1. Standalone Test Designs (01-03)

The standalone tests are well-structured and cover the primary logic of the shared entrypoint.

*   **01-entrypoint-validation**: Complete and matches the validation logic in `entrypoint.sh`. 
*   **02-vlan-modes**: Correctly identifies the three modes. To avoid the race condition mentioned in the findings, it is recommended to set a high `DHCP_TIMEOUT` (e.g., 60s) via environment variables for these tests, ensuring the container stays alive long enough for Robot to perform its inspections.
*   **03-cleanup**: Verifying cleanup *after* exit is the challenge. If the test uses a dedicated Docker network, Docker will clean up the namespace anyway. To truly test that `entrypoint.sh`'s `cleanup()` function works, the test should probably run a "monitor" container sharing the same network namespace (`--net container:target`) that persists after the target exits, allowing Robot to check the interface state.

### 2. Integration Test (04) vs. `smoke-test.sh`

The integration test properly replicates and improves upon `lab/smoke-test.sh`. The inclusion of the BNG API check (`/api/show/subscriber/sessions`) is a significant improvement as it validates the control plane state, not just the data plane path.

### 3. Image Matrix Approach

The approach of using a `${SUBSCRIBER_IMAGE}` variable is practical and fits well with the project's goal of supporting multiple distros. Since `entrypoint.sh` is shared and its dependencies are satisfied in all three Dockerfiles (Alpine uses `dhcpcd`, Debian/Ubuntu use `isc-dhcp-client`), the matrix should be stable.

### 4. Shared Keywords

The keywords adapted from `osvbng` are sufficient for the integration tier. For the standalone tier, the following enhancements are suggested:

*   **`Run Container In Background`**: To allow Robot to continue execution while the subscriber is "waiting" for DHCP.
*   **`Wait For Interface In Container`**: A polling keyword to check `ip link` until the VLAN appears.
*   **`Check Interface Removed`**: As noted in Findings, this needs a "sidecar" or "shared namespace" strategy to be meaningful.

### 5. Directory Structure and Naming

The `tests/` directory and numbered suite naming convention (`01-`, `02-`, etc.) are appropriate and maintain consistency with the `osvbng` repository, which is beneficial for developers working on both projects.

## Recommendations

1.  **Add Host Prerequisites**: Update the "Dependencies" section of the spec to include the requirement for `8021q` and `802.1ad` kernel modules on the Docker host.
2.  **Explicit Timeout for Standalone**: In `02-vlan-modes`, explicitly set `DHCP_TIMEOUT=60` to stabilize the `docker exec` checks.
3.  **Sidecar Pattern for Cleanup**: Consider adding a "sidecar" keyword or pattern to the `03-cleanup` design to verify that interfaces are removed *within* the namespace before the namespace itself is destroyed.
4.  **Negative Validation**: Add a test case to `01-entrypoint-validation` that ensures `ENCAP=untagged` ignores or warns about `CVLAN/SVLAN` if provided (or strictly forbids them).
