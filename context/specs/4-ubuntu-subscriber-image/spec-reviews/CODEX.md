# Spec Critique: Ubuntu Subscriber Image (Codex)

I do not see an obvious missing-package problem in the proposed Ubuntu Dockerfile. `isc-dhcp-client`, `ppp`, `iputils-ping`, `iproute2`, `iperf3`, `curl`, `ca-certificates`, and `netbase` match the current shared-entrypoint contract and the Debian pattern. The weak spots are in what the spec claims the tests prove, and in how the scope boundary handles a Jammy-specific mismatch if one appears.

## Findings

### MEDIUM: Debian parity is asserted more strongly than the Ubuntu tests actually prove

- The spec says Ubuntu needs no entrypoint changes because issue #3 already completed all `dhclient` compatibility work, and it repeatedly states that Ubuntu should behave identically to Debian because both use `isc-dhcp-client` (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:5`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:13`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:47`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:55`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:159`).
- The actual runtime contract still depends on the shared entrypoint copying `/etc/dhcp/dhclient.conf` if present, appending `timeout N;`, launching `dhclient -4/-6 -1 -d -cf ...`, and waiting on that PID (`images/shared/entrypoint.sh:202`, `images/shared/entrypoint.sh:208`, `images/shared/entrypoint.sh:212`, `images/shared/entrypoint.sh:224`).
- The Ubuntu testing section does not verify that Jammy actually honors that contract. Its `DHCP_TIMEOUT honored` test only checks that the generated config file contains the requested timeout value, not that `dhclient` exits on that deadline in a real no-lease scenario (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:153`). Likewise, `Container stays alive after lease` is listed as an expectation but without any Ubuntu-specific observation point beyond a happy-path lease (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:155`).
- Since issue #4 explicitly exists to validate Ubuntu's package/version behavior, the spec should require at least one Ubuntu-specific runtime parity check rather than treating Debian as proof. A concrete addition would be: verify a non-default `DHCP_TIMEOUT` causes `dhclient` to exit non-zero around the configured deadline when no server responds, and verify a successful lease leaves the `dhclient` process alive for renewals.

### MEDIUM: The cleanup test repeats a previously fixed Debian spec weakness

- The Ubuntu spec says `SIGTERM cleanup + lease release` validates `lease release via dhclient -r` (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:154`).
- In the real entrypoint, cleanup suppresses any `dhclient -r` failure and then sends `kill`/`wait` to the client PID (`images/shared/entrypoint.sh:35`, `images/shared/entrypoint.sh:40`, `images/shared/entrypoint.sh:42`). That means interface teardown alone does not prove a DHCPRELEASE was actually sent.
- Issue #3 already accepted this exact critique and tightened the Debian spec to require an observation point such as `ip addr show` confirming address removal or DHCP server logs showing `DHCPRELEASE` (`context/specs/3-debian-subscriber-image/DECISIONS.md:30`, `context/specs/3-debian-subscriber-image/DECISIONS.md:33`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:169`).
- The Ubuntu spec should carry that same requirement forward. As written, this test can claim success even if `dhclient -r` did nothing.

### MEDIUM: The Dockerfile-only scope boundary has no explicit stop condition if Jammy is not perfectly Debian-compatible

- The spec says the image is a single-phase Dockerfile-only change, that no existing files should be modified, and that entrypoint changes are out of scope (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:55`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:108`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:125`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:170`).
- The same spec also says any Ubuntu/Debian behavior difference would indicate a packaging or version discrepancy worth investigating (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:159`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:166`).
- That leaves a gap in the implementation contract. If Jammy exposes a real difference in `dhclient -cf`, `dhclient -r`, or `pppoe.so` packaging, Phase 5 is blocked but the spec does not say whether implementation should stop immediately and amend the issue, or whether the branch is allowed to take a minimal compatibility fix. The issue's motivation explicitly calls out Ubuntu-specific package-version differences, so this is not just a theoretical edge case.
- To preserve the scope boundary cleanly, the spec should add an explicit stop rule: if Ubuntu reveals an entrypoint incompatibility, stop implementation, document the mismatch, and amend the relevant issue before merging. Without that, the spec promises a one-file implementation path without defining the failure path.

### LOW: The spec advertises DHCPv6 support but never exercises the Ubuntu DHCPv6 path

- The overview and configuration sections describe the image as supporting DHCPv4 and DHCPv6 (`context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:5`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:67`, `context/specs/4-ubuntu-subscriber-image/IMPLEMENTATION_SPEC.md:73`).
- The shared entrypoint has separate `dhclient -4` and `dhclient -6` launch paths (`images/shared/entrypoint.sh:212`, `images/shared/entrypoint.sh:224`).
- The Ubuntu test matrix only exercises DHCPv4. That is acceptable if issue #4 is intentionally scoped to DHCPv4 QinQ acceptance, but then the spec should avoid implying that Ubuntu-specific DHCPv6 behavior was validated. Otherwise, add at least one DHCPv6 smoke test or explicitly say DHCPv6 is inherited but not revalidated by this issue.
