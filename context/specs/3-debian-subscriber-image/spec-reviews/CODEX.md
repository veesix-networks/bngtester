# Spec Critique: Debian Subscriber Image (Codex)

The Dockerfile plan itself is small and coherent. The weak point is the spec's repeated claim that Debian is a pure Dockerfile exercise. The current shared entrypoint does detect `dhclient`, but the Debian-specific behavior around timeout, cleanup, and acceptance coverage is not as complete as the spec claims.

## Findings

### HIGH: "No entrypoint changes" is not justified by the current `dhclient` implementation

- The spec says Debian reuses the shared entrypoint with "zero modifications" and that the `dhclient` dispatch/release paths are "already implemented and tested" (`context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:5`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:13`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:53`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:155`).
- The current entrypoint does auto-detect `dhclient`, but its Debian path is only `dhclient -4 -v -1 -d` / `dhclient -6 -v -1 -d` (`images/shared/entrypoint.sh:199`, `images/shared/entrypoint.sh:209`). `DHCP_TIMEOUT` is logged but never applied to `dhclient`, even though the Debian spec inherits `DHCP_TIMEOUT` as a supported env var (`context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:63`) and issue #1's accepted failure contract says timeout is part of both DHCP methods (`context/specs/1-alpine-subscriber-image/IMPLEMENTATION_SPEC.md:82`, `context/specs/1-alpine-subscriber-image/IMPLEMENTATION_SPEC.md:88`, `context/specs/1-alpine-subscriber-image/IMPLEMENTATION_SPEC.md:208`, `context/specs/1-alpine-subscriber-image/DECISIONS.md:32`).
- Bookworm `dhclient(8)` documents `-1`, `-d`, and `-r`, but not a `-timeout` CLI flag; Bookworm `dhclient.conf(5)` documents `timeout` as config-file state with a default of 60 seconds. So a non-default `DHCP_TIMEOUT` cannot work on Debian without extra entrypoint or Dockerfile logic.
- I did not find evidence that `dhclient -d` daemonizes away from `$CLIENT_PID`; the concrete breakage is simpler: the Debian path does not currently honor the shared timeout contract. As written, the spec should not claim "no entrypoint changes" unless it explicitly accepts Debian-specific timeout drift.

### MEDIUM: the `dhclient` cleanup/release path is assumed rather than proven

- The spec says the cleanup path already handles Debian correctly and that SIGTERM cleanup validates "dhclient lease released on stop" (`context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:53`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:151`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:159`).
- In the actual entrypoint, cleanup suppresses all `dhclient -r` failures and then sends a plain `kill` to `$CLIENT_PID` (`images/shared/entrypoint.sh:35`). Bookworm `dhclient(8)` documents `-r` as releasing the lease and stopping the running client recorded in its PID file, so this path depends on pid-file behavior and cannot be inferred from interface teardown alone.
- The testing section needs an explicit observation point for Debian lease release, such as DHCP server logs, packet capture, or an address-state check before container exit. Without that, "lease released on stop" is aspirational, not validated.

### MEDIUM: the testing section does not fully map to issue #3 acceptance criteria

- Issue #3 requires: Dockerfile builds, shared entrypoint reused, `isc-dhcp-client` gets a DHCPv4 lease through QinQ, `ppp` is installed for PPPoE, tools are installed, and SPDX headers are present.
- The spec's testing section covers build success, QinQ DHCPv4, tool presence, and SPDX (`context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:128`).
- What is missing is explicit acceptance coverage for "uses the shared entrypoint from issue #1" and "`ppp` package installed for PPPoE support." The current tests validate behavior, but they do not explicitly prove that the Dockerfile copied `shared/entrypoint.sh` rather than a forked script, or that Debian's `ppp` package provides the PPPoE plugin the shared entrypoint expects.
- Add artifact-level checks to Build Validation: verify `COPY shared/entrypoint.sh /entrypoint.sh` in the Dockerfile, and verify the PPPoE plugin/binary expected by `plugin pppoe.so` is present in the built image.

### MEDIUM: the scope boundary has no blocking path if Debian uncovers an entrypoint gap

- The spec says entrypoint changes are out of scope and should be treated as an amendment to issue #1 (`context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:164`).
- That boundary is too rigid for this issue, because issue #3's value is specifically to validate `dhclient` against the shared entrypoint. If Debian exposes a real incompatibility, the implementation is blocked by a dependency the spec labels out of scope, while the rest of the document still promises a one-file Dockerfile-only change (`context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:108`, `context/specs/3-debian-subscriber-image/IMPLEMENTATION_SPEC.md:114`).
- The spec should say one of two things explicitly: either issue #3 is blocked pending an issue #1 amendment if any `dhclient` gap is found, or minimal shared-entrypoint changes required to make Debian work are in scope for this branch because they are part of validating the Debian image.

## Reference Notes

- Bookworm `dhclient(8)`: https://manpages.debian.org/bookworm/isc-dhcp-client/dhclient.8.en.html
- Bookworm `dhclient.conf(5)`: https://manpages.debian.org/bookworm/isc-dhcp-client/dhclient.conf.5.en.html
