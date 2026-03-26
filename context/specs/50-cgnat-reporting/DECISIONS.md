# Decisions: 50-cgnat-reporting

## Accepted

### IP-only comparison for peer vs subscriber (strip port)
- **Source:** GEMINI (G2), CODEX (C3)
- **Severity:** LOW / MEDIUM
- **Resolution:** Parse peer as SocketAddr, compare peer.ip() to subscriber_ip parsed as IpAddr. String comparison would fail because peer includes port. On parse failure, fall back to dual display.

### subscriber_ip on both TestConfig and ClientReport
- **Source:** GEMINI (G3)
- **Severity:** LOW
- **Resolution:** Keep on both for usability. TestConfig is the primary home, ClientReport provides top-level convenience for combined report consumers.

### No fallback to control socket local address
- **Source:** CODEX (C1)
- **Severity:** HIGH
- **Resolution:** Critical fix. Control socket local address may be management IP, not subscriber data path IP. subscriber_ip only set from --source-ip. If not set, field omitted entirely — not populated with a wrong address.

### subscriber_ip uses skip_serializing_if, additive schema change
- **Source:** CODEX (C2)
- **Severity:** HIGH
- **Resolution:** #[serde(skip_serializing_if = "Option::is_none")] on subscriber_ip. Field omitted when None. Explicitly documented as additive schema change — strict consumers must tolerate new optional field.

## Rejected

### Add --hide-local-ip flag for privacy
- **Source:** GEMINI (G1)
- **Severity:** MEDIUM
- **Rationale:** This is a private BNG test tool, not a public service. The operator owns the network being tested. subscriber_ip is only sent when --source-ip is explicitly set (no automatic local IP leak). Documenting the behavior is sufficient.
