# Spec Review: 27-containerlab-topology (Gemini)

Review of the implementation spec for the containerlab topology with osvbng.

## Findings

### HIGH: Linux Control Plane (LCP) and Namespaces
- **Severity:** HIGH
- **Finding:** The `osvbng` configuration relies heavily on the `dataplane` plugin and VPP's Linux Control Plane (LCP) to synchronize interfaces from the VPP data plane into the Linux network stack. 
- **Rationale:** The reference implementation in the `osvbng` repository uses `lcp-netns: dataplane` in `osvbng.yaml` and explicitly creates this namespace in its entrypoint. Without this configuration, the Linux stack (where OSPF and the DHCP server reside) will not see the BNG interfaces, causing the control plane to fail.
- **Recommendation:** Explicitly include the `dataplane` plugin configuration and the `lcp-netns` setting in the `lab/config/bng1/osvbng.yaml` section of the spec.

### MEDIUM: OSPF Stability (Passive Loopbacks)
- **Severity:** MEDIUM
- **Finding:** The spec does not mention configuring loopback interfaces as passive in the OSPF configuration.
- **Rationale:** Best practices for OSPF involve setting loopback interfaces to `passive: true` to prevent the router from attempting to form adjacencies on those interfaces and to reduce unnecessary LSA flooding. This is also present in the reference implementation.
- **Recommendation:** Update the OSPF configuration plan for both the BNG and the Server node to include `passive: true` for loopback interfaces.

### MEDIUM: IPv6 Future-Proofing
- **Severity:** MEDIUM
- **Finding:** The spec explicitly omits IPv6, which is acceptable for the current scope, but does not address how the current IP plan relates to future dual-stack requirements.
- **Rationale:** `osvbng` is a dual-stack BNG. Defining the relationship between the current IPv4-only scope and future IPv6 work (e.g., in "Not In Scope") helps prevent architectural decisions that might make adding IPv6 harder later.
- **Recommendation:** Add a note in the "Not In Scope" or "Future Work" section confirming that the topology is designed to be extensible to IPv6 (DHCPv6/RA) in a follow-up issue.

### LOW: MTU Overhead for QinQ
- **Severity:** LOW
- **Finding:** Double-tagged (QinQ) encapsulation adds 8 bytes of overhead per frame.
- **Rationale:** If the underlying physical links (veth pairs created by containerlab) use the standard 1500-byte MTU, the effective MTU for the subscriber will be 1492. This can cause issues with full-sized packets or path MTU discovery.
- **Recommendation:** Add a note to the `lab/README.md` troubleshooting section recommending that users either increase the host/veth MTU to 1508 or configure the subscriber to use an MTU of 1492 to avoid fragmentation issues.

### LOW: Server Tooling for Smoke Test
- **Severity:** LOW
- **Finding:** The smoke test depends on `iperf3` on the server node.
- **Rationale:** The `frrouting/frr` base image is optimized for routing and may not include `iperf3` by default.
- **Recommendation:** Ensure the `lab/config/server/entrypoint.sh` or the `clab` file includes a step to install `iperf3` if it is not present, or verify its presence in the chosen image.

## Conclusion

The spec is high quality and closely aligns with proven patterns from the `osvbng` project. Addressing the LCP/namespace configuration is the most critical addition needed to ensure a functional data plane.
