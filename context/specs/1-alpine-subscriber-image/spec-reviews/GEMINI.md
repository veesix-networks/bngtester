# Spec Review: 1-alpine-subscriber-image (Gemini)

## Overview

This review evaluates the Alpine Linux subscriber image specification for Docker best practices, networking correctness, and extensibility.

## Findings

### 1. Dockerfile Best Practices

| Finding | Severity | Recommendation |
|---------|----------|----------------|
| **Package pinning** | LOW | Use pinned versions for packages (e.g., `dhcpcd~=10.0`) or at least `apk add --no-cache` to ensure reproducible builds and smaller image size. |
| **Layer ordering** | LOW | The current order is fine, but ensure `COPY` of the entrypoint happens after package installation to maximize layer caching. |

### 2. Networking Correctness (VLAN/QinQ)

| Finding | Severity | Recommendation |
|---------|----------|----------------|
| **QinQ Protocol** | MEDIUM | The spec uses `proto 802.1ad` for the S-VLAN (outer), which is correct for 802.1ad. Ensure the C-VLAN (inner) uses the default `802.1Q`. The spec currently says `type vlan id $CVLAN`, which defaults to 802.1Q. This is correct. |
| **Interface Up sequence** | MEDIUM | Ensure the parent interface (physical) is `up` before creating VLAN sub-interfaces. The spec implies this but doesn't explicitly state the physical interface state management. |

### 3. dhcpcd and pppd Flag Usage

| Finding | Severity | Recommendation |
|---------|----------|----------------|
| **pppd `nodetach`** | **HIGH** | The spec mentions `pppd` runs in the foreground but doesn't explicitly list the `nodetach` flag. Without `nodetach`, `pppd` will fork, and the Docker container will immediately exit. |
| **pppd standard flags** | **HIGH** | For a functional subscriber connection, `pppd` typically requires `noauth` (most ISPs), `defaultroute`, and `usepeerdns`. These should be added to the implementation. |
| **pppd authentication** | MEDIUM | Ensure the entrypoint passes `user "$PPPOE_USER" password "$PPPOE_PASSWORD"` to the `pppd` command. |
| **dhcpcd release on exit** | MEDIUM | Consider using the `--persistent` (or `-p`) flag if you want the IP to stay assigned during a restart, or ensure the `trap` handles `dhcpcd -k` (kill/release) if you want a clean release on BNG. |
| **dhcpcd timeout** | LOW | Use `-t 0` (timeout 0) to wait indefinitely for a lease if the BNG is slow to respond, preventing the container from exiting on initial failure. |

### 4. POSIX sh Portability & Signal Handling

| Finding | Severity | Recommendation |
|---------|----------|----------------|
| **Signal Proxying** | **HIGH** | If the entrypoint script is the parent (PID 1) and launches `dhcpcd` or `pppd` as children, it MUST either `exec` the final process or explicitly proxy signals. If it uses `trap` and stays alive, it must ensure it doesn't block the signals (e.g., by using `wait`). |
| **Cleanup Logic** | MEDIUM | The `trap` should explicitly kill the background `dhcpcd` if not using `exec`. |

### 5. Environment Variable Validation

| Finding | Severity | Recommendation |
|---------|----------|----------------|
| **Validation completeness** | MEDIUM | Add validation for `PPPOE_USER` and `PPPOE_PASSWORD` when `ACCESS_METHOD=pppoe`. |
| **Invalid VLAN IDs** | LOW | Add validation that `SVLAN` and `CVLAN` are between 1 and 4094. |

### 6. Extensibility (Debian/Ubuntu)

| Finding | Severity | Recommendation |
|---------|----------|----------------|
| **DHCP Client Dispatch** | MEDIUM | The shared entrypoint design should use a helper function or variable (e.g., `DHCP_CLIENT`) that defaults to `dhcpcd` in Alpine but can be set to `dhclient` in Debian. This avoids hardcoding `dhcpcd` throughout the script. |

## Suggested Changes

### Entrypoint Script Enhancements

```sh
# Helper for DHCP client selection
get_dhcp_client() {
    if command -v dhcpcd >/dev/null 2>&1; then
        echo "dhcpcd"
    elif command -v dhclient >/dev/null 2>&1; then
        echo "dhclient"
    else
        echo "error"
    fi
}

# Example PPPoE launch with correct flags
pppd plugin pppoe.so "$TARGET_IFACE" \
    user "$PPPOE_USER" \
    password "$PPPOE_PASSWORD" \
    nodetach \
    noauth \
    defaultroute \
    usepeerdns \
    persist
```

### Dockerfile Optimization

```dockerfile
RUN apk add --no-cache \
    dhcpcd \
    ppp \
    ppp-pppoe \
    iputils \
    iproute2 \
    iperf3
```

## Conclusion

The spec is solid and establishes a good foundation. Addressing the `pppd` flags and ensuring signal handling is robust in the entrypoint script will prevent common Docker networking pitfalls.
