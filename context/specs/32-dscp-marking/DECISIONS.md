# Decisions: 32-dscp-marking

## Accepted

### TCP SYN must carry DSCP marking
- **Source:** GEMINI (G1)
- **Severity:** HIGH
- **Resolution:** TCP sockets created via socket2 with set_tos() called before connect(). This ensures the SYN packet carries the correct DSCP, which matters for BNGs that classify on SYN.

### Server applies DSCP on reverse-path streams
- **Source:** GEMINI (G2)
- **Severity:** HIGH
- **Resolution:** Server reads DSCP config from hello message and applies it to reverse-path data stream sockets in bidirectional/RRUL modes. Generator implementation is shared — both client and server apply DSCP if configured.

### ECN bits interaction documented
- **Source:** GEMINI (G4)
- **Severity:** MEDIUM
- **Resolution:** Added note that IP_TOS zeros ECN bits. When issue #33 adds ECN, the helper must be updated to read-modify-write (preserve ECN bits via getsockopt before setting DSCP).

### Add socket2 dependency
- **Source:** GEMINI (G5)
- **Severity:** LOW
- **Resolution:** Added socket2 to Cargo.toml and file plan. Provides set_tos() and the create→set→connect pattern needed for TCP.

### Strict DSCP value validation
- **Source:** GEMINI (G6)
- **Severity:** LOW
- **Resolution:** Numeric values must be 0-63, names must match standard PHB list. Invalid inputs cause immediate error with helpful message.

### Generic helper naming for future IPv6
- **Source:** GEMINI (G7)
- **Severity:** LOW
- **Resolution:** Helper named generically. Currently asserts IPv4 and fails with clear error on IPv6. When IPV6_TCLASS is needed, the helper can be extended without renaming.

### setsockopt failure must fail-fast
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** If set_tos() fails, the test aborts immediately before any data is sent. Silent fallback to BE would produce misleading QoS results. Added negative test for unprivileged failure case.

### Scope narrowed to data streams only
- **Source:** CODEX (C2)
- **Severity:** HIGH
- **Resolution:** Overview and design explicitly scoped to data stream sockets only. Control channel TCP sockets are not marked. Updated overview text and added "Scope: Data Streams Only" section.

### IPv4-only constraint with explicit assertion
- **Source:** CODEX (C3)
- **Severity:** MEDIUM
- **Resolution:** Added "IPv4-Only Constraint" section. The helper checks socket address family and fails with clear error if IPv6 endpoint is used with --dscp. Current codebase hardcodes IPv4 binds anyway.

### JSON backward compatibility via skip_serializing_if
- **Source:** CODEX (C4)
- **Severity:** MEDIUM
- **Resolution:** New dscp and dscp_name fields use #[serde(skip_serializing_if = "Option::is_none")] so reports without DSCP don't change JSON shape. Added junit.rs to file plan for test constructor updates.

## Rejected

### Add IP_RECVTOS for received DSCP verification
- **Source:** GEMINI (G3)
- **Severity:** MEDIUM
- **Rationale:** Verifying received DSCP is a separate feature (detecting BNG re-marking or stripping). It requires recvmsg with cmsg ancillary data, which is a different code path than setsockopt. Filed as a follow-up concern — can be addressed alongside issue #33 (ECN) since both use the same IP_RECVTOS/recvmsg mechanism.
