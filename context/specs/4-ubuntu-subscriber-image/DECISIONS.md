# Decisions: 4-ubuntu-subscriber-image

## Accepted

### dhclient config conflict check
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Added build validation test to verify Ubuntu's default `/etc/dhcp/dhclient.conf` does not contain conflicting `timeout` or `retry` settings that would override the entrypoint-generated config.

### DEBIAN_FRONTEND=noninteractive
- **Source:** GEMINI
- **Severity:** LOW
- **Resolution:** Added `DEBIAN_FRONTEND=noninteractive` inline in the Dockerfile RUN command to prevent potential interactive prompts during package installation.

### Ubuntu-specific runtime parity checks
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Strengthened the testing section to require Ubuntu-specific runtime validation rather than treating Debian results as proof. Added a runtime timeout test (dhclient exits non-zero around configured deadline when no server responds) and tightened the container-stays-alive test to verify the dhclient process remains alive for renewals. Added note about dhclient version difference (Ubuntu 4.4.1 vs Debian 4.4.3).

### Cleanup test observation point
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Tightened the SIGTERM cleanup test to require verification via `ip addr show` confirming no address on the target interface or DHCP server log showing DHCPRELEASE — matching the same requirement accepted for the Debian spec in issue #3.

### Explicit stop rule for entrypoint incompatibilities
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Added an explicit stop rule to the design section: if Ubuntu reveals a behavioral difference in dhclient, pppd, or any entrypoint code path, stop implementation, document the mismatch, file or amend the relevant issue, and do not merge until resolved.

### DHCPv6 coverage clarification
- **Source:** CODEX
- **Severity:** LOW
- **Resolution:** Added a "DHCPv6 Coverage" subsection to the testing section explicitly stating that DHCPv6 is inherited from the shared entrypoint but not revalidated by this issue's test matrix. If Ubuntu-specific DHCPv6 testing is needed, it should be a separate testing issue.

## Rejected

None.
