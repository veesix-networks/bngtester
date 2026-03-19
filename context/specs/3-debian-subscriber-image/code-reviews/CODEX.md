# Code Review: Debian Subscriber Image (Codex)

I built `images/debian/Dockerfile` locally and verified the static/image-level acceptance criteria that can be checked without a BNG test network:

- `docker build -f images/debian/Dockerfile images/` succeeds
- image entrypoint is `/entrypoint.sh`
- expected tools are present: `dhclient`, `pppd`, `ping`, `ip`, `iperf3`, `curl`
- `ca-certificates`, `netbase`, and `pppoe.so` are present in the built image
- the Dockerfile copies `shared/entrypoint.sh`

I did not have a DHCP/PPPoE test environment for QinQ runtime validation, so the functional acceptance criteria remain unverified in this review.

## Findings

### MEDIUM: the `DHCP_TIMEOUT` fix replaces Debian's packaged `dhclient.conf`, so the container is no longer exercising the stock Debian client profile

- The finalized spec explicitly chose `dhclient -cf /tmp/dhclient-bngtester.conf` as the timeout fix, and the implementation matches that design in [`images/shared/entrypoint.sh`](../../../images/shared/entrypoint.sh) at lines 198-223.
- The generated config contains only `timeout N;`. Because `-cf` replaces the default config path rather than extending it, this bypasses Debian's packaged `/etc/dhcp/dhclient.conf`.
- In the built Debian image, `/etc/dhcp/dhclient.conf` includes Debian-specific defaults such as `send host-name = gethostname();` and an expanded `request ...` list covering `domain-search`, `dhcp6.*`, `interface-mtu`, `rfc3442-classless-static-routes`, and `ntp-servers`.
- Issue #3's goal was to validate Debian `isc-dhcp-client` behavior and option parsing against the BNG. With the current implementation, we are validating a custom reduced `dhclient` profile instead of the packaged Debian defaults.
- Recommendation: preserve the packaged config and layer the timeout override on top of it. For example, generate a temp file by copying `/etc/dhcp/dhclient.conf` and appending `timeout $DHCP_TIMEOUT;`, then pass that file via `-cf`.

## Notes

- I did not find a Dockerfile compliance gap. [`images/debian/Dockerfile`](../../../images/debian/Dockerfile) matches the finalized package list and image structure from the spec.
- The accepted lifecycle-consistency concern does not appear to be a remaining code issue here: Alpine still uses `dhcpcd -B`, and in `dhcpcd` that is the `--nobackground` flag, so the current code path is consistent with keeping the container alive after lease acquisition.
- Unverified in this review: DHCPv4 lease acquisition through QinQ, PPPoE launch against a real access network, SIGTERM lease release behavior against a real DHCP server, and end-to-end confirmation that a non-default `DHCP_TIMEOUT` behaves as intended in a live Debian `dhclient` environment.
