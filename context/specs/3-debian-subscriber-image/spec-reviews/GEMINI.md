# Spec Review: Debian Subscriber Image (Gemini)

## Summary

The spec is technically sound and follows the established patterns for subscriber images in this project. The choice of Debian 12 (Bookworm) and the package mapping are correct. However, there are some inconsistencies in behavioral expectations between the Alpine and Debian implementations, primarily due to how the shared entrypoint interacts with different DHCP clients.

## Findings

| Severity | Finding |
|----------|---------|
| **HIGH** | **Inconsistent Container Lifecycle:** `dhclient -1 -d` (Debian) remains in the foreground after a successful lease, while `dhcpcd -B` (Alpine) backgrounds itself, causing the Alpine container to exit immediately. This creates a significant behavioral difference between the two images. |
| **HIGH** | **Ignored `DHCP_TIMEOUT`:** The shared entrypoint does not pass the `DHCP_TIMEOUT` variable to `dhclient`. This renders the configuration variable non-functional for the Debian image. |
| **MEDIUM** | **Missing `ca-certificates`:** `curl` is included for testing, but `ca-certificates` is missing. HTTPS requests via `curl` will fail by default. |
| **MEDIUM** | **Missing `netbase`:** Minimal `bookworm-slim` lacks `/etc/protocols` and `/etc/services`, which can cause resolution issues for some networking tools. |
| **LOW** | **PPPoE Plugin Naming:** While `pppoe.so` is available in Debian 12, `rp-pppoe.so` is the more conventional name for the kernel-mode plugin. Both are present, so this is likely a non-issue but worth noting for consistency. |

## Detailed Analysis

### 1. Foreground Behavior and Lifecycle
The shared entrypoint uses `dhclient -1 -d` for Debian and `dhcpcd -B` for Alpine.
- `dhclient -1 -d`: Stays in the foreground after getting a lease (manages renewals). The `wait $CLIENT_PID` in the entrypoint will wait indefinitely.
- `dhcpcd -B`: Backgrounds itself after getting a lease. The parent process exits, `wait $CLIENT_PID` returns, and the container exits.

**Recommendation:** The entrypoint should be modified to use consistent foreground flags (e.g., `dhcpcd -f` instead of `-B`) to ensure all subscriber images stay alive to handle renewals and allow for post-launch testing (iperf3, ping).

### 2. DHCP Timeout Support
`dhclient` does not have a command-line flag to set the lease acquisition timeout (it uses `timeout 60;` in `dhclient.conf`). The entrypoint correctly passes `DHCP_TIMEOUT` to `dhcpcd` but ignores it for `dhclient`.

**Recommendation:** To honor `DHCP_TIMEOUT` for Debian, the entrypoint or the Dockerfile should provide a basic `dhclient.conf` or the entrypoint should be amended to generate one dynamically.

### 3. Package Completeness
For a "test" image containing `curl`, `ca-certificates` is essential for modern web testing. `netbase` is also a standard requirement for networking stability in minimal Debian images.

**Recommendation:** Add `ca-certificates` and `netbase` to the `apt-get install` list in the Dockerfile.

### 4. PPPoE Plugin
Debian 12's `ppp` package (v2.4.9) provides both `/usr/lib/pppd/2.4.9/pppoe.so` and `rp-pppoe.so`. The current entrypoint uses `plugin pppoe.so`, which is valid. No changes needed here, but `rp-pppoe.so` is the "official" name for the kernel-mode plugin from the `rp-pppoe` project integrated into `pppd`.

## Focus Area Responses

- **Are the Debian package names correct and sufficient for bookworm-slim?**
  Yes, but `ca-certificates` and `netbase` should be added for completeness and reliability.
- **Does the ppp package on Debian include pppoe.so, or is a separate package needed?**
  Yes, it is included in the `ppp` package. No separate package is needed.
- **Are there dhclient-specific behaviors (lease files, config directories, foreground flags) that the existing entrypoint doesn't handle?**
  Yes:
  1. `DHCP_TIMEOUT` is ignored.
  2. Foreground behavior is different from `dhcpcd` (though `dhclient`'s behavior is actually more appropriate for a container).
  3. Lease file directory `/var/lib/dhcp/` is required (handled by package installation).
- **Is bookworm-slim missing any runtime dependencies that the packages need?**
  `ca-certificates` (for `curl`) and `netbase` (for protocol/service lookups) are the main omissions.
- **Image size considerations — are there unnecessary packages or layers?**
  The package list is lean. Using a single `RUN` layer with cache cleanup is optimal.
