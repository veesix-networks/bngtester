# Decisions: 3-debian-subscriber-image

## Accepted

### DHCP_TIMEOUT not passed to dhclient
- **Source:** GEMINI + CODEX
- **Severity:** HIGH
- **Resolution:** Entrypoint will generate a minimal `/tmp/dhclient-bngtester.conf` with `timeout $DHCP_TIMEOUT;` and pass it via `dhclient -cf`. This honors the timeout contract on both Alpine (dhcpcd -t) and Debian (dhclient.conf). Added to File Plan and Implementation Order Phase A.

### Container lifecycle inconsistency (dhcpcd vs dhclient)
- **Source:** GEMINI
- **Severity:** HIGH
- **Resolution:** Accept finding. During implementation, verify dhcpcd -B behavior on Alpine 3.21. If dhcpcd exits after lease (unlike dhclient -1 -d which stays alive), change to -f for foreground persistence. Both images should keep containers alive for post-lease testing.

### "No entrypoint changes" claim not justified
- **Source:** CODEX
- **Severity:** HIGH
- **Resolution:** Removed the "zero modifications" claim from the spec. Overview, Design, File Plan, and Implementation Order now explicitly describe the two entrypoint fixes (DHCP_TIMEOUT config generation and lifecycle verification/fix). Scope amendment posted on issue #3.

### Missing ca-certificates
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Added `ca-certificates` to the Debian package list. Required for curl HTTPS support on bookworm-slim. Added HTTPS verification to Build Validation tests.

### Missing netbase
- **Source:** GEMINI
- **Severity:** MEDIUM
- **Resolution:** Added `netbase` to the Debian package list. Provides `/etc/protocols` and `/etc/services` missing in bookworm-slim.

### dhclient cleanup/release path assumed, not proven
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Testing section updated with explicit observation points for lease release: verify via address-state check (`ip addr show` confirms no address) or DHCP server logs showing DHCPRELEASE. "Lease released on stop" is now a testable assertion, not an assumption.

### Testing section doesn't fully map to acceptance criteria
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Added artifact-level checks to Build Validation: verify Dockerfile uses `COPY shared/entrypoint.sh` (not a fork), verify `pppoe.so` plugin is present in the built image under `/usr/lib/pppd/`.

### Scope boundary too rigid for entrypoint gaps
- **Source:** CODEX
- **Severity:** MEDIUM
- **Resolution:** Relaxed scope boundary. Minimal shared entrypoint fixes required for dhclient compatibility are in scope for this issue. "Not In Scope" now says only unrelated entrypoint changes require a separate issue. Scope amendment documented on issue #3.

## Rejected

### PPPoE plugin naming (pppoe.so vs rp-pppoe.so)
- **Source:** GEMINI
- **Severity:** LOW
- **Rationale:** `pppoe.so` works on Debian 12 — pppd resolves it from its plugin directory. Both names point to the same plugin. Changing to `rp-pppoe.so` would break Alpine where only `pppoe.so` is available. No change needed.
