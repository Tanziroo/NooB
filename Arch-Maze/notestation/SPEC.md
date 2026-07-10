# Notestation — Specification

**Version 1.0** · consolidates the incremental protocol drafts (0.1–0.7) into one
clean, authoritative document. This is the canonical spec; the reference tools
(`scribe`, `dial`, `sealgate`) are one conforming implementation.

---

## 1. Summary

Notestation is a **trusted neutral middle layer**: an open, versioned protocol for
**point-in-time validation** of bundles. It answers one question, honestly, at a
moment: *"are the declared objects present, in the declared state, right now?"* —
and records that answer as independently-verifiable evidence.

It is the **bundle maker and validator**. It does not transport, hold, custody,
settle, or resolve anything. Those are separate roles for separate parties.

## 2. Principle

- **Label ≤ Mechanism.** A claim may never exceed what its mechanism can prove.
  Hashes and signatures are computed in code, never asserted; every value is
  independently checkable.
- **Certainty is asymptotic.** No stack of evidence reaches 100%. Each attestation
  layer collapses one axis of uncertainty; the seal is the binary decision that
  forces the collapse. The residual is never zero, and honesty requires naming it.
- **Evidence, not guarantees.** A seal is one line of evidence a relying party
  weighs alongside others; it does not ask anyone to take our word for it.

## 3. Scope of claims (liability)

**What a seal claims:** *"The described artifact was present, in the described
byte-for-byte state, at the recorded time, as computed by the named mechanism."*
Nothing more.

**What it does NOT claim:** authenticity, truth, accuracy, ownership, lawfulness, or
fitness; trust in any signing key (that is pinned out-of-band by the verify layer);
time beyond the recorded value (absent an external timestamp authority). Absence of a
seal is not evidence of anything.

Not legal advice. Any liability-bearing use is codified in a separate agreement. The
software is MIT, "AS IS", without warranty. Full text: `ATTESTATION-CLAIMS.md`.

## 4. Roles & boundaries

```
   relying parties · insured partners        judge trust · underwrite
                    ▲
   ── Notestation: validate + record ──      the trusted neutral middle
                    ▼
   attestation devices · anchors · verifiers  prove individual axes
```

| We do | Deliberately NOT us → another party's role |
|-------|--------------------------------------------|
| validate presence/state at a moment | resolve discrepancies → parties / dispute handlers |
| emit a seal or a discrepancy report | transport the bundle → carriers |
| append the honest record | hold / move / release value → banks, insured custodians |
| charge a small per-validation fee | underwrite outcomes → insurers |

If a transaction involves value or goods, **neither ever passes through
Notestation.** We validate the endpoints; others move, hold, and settle.

## 5. Protocol surfaces

### 5.1 Plan record (`scribe-plan.json`) — the unit of a transaction
```json
{
  "scribe_version": "0.6.0",
  "protocol_version": "1.0",
  "root": "/path",
  "attestation_tier": "fast-fnv | verified-sha256 | signed-ed25519",
  "event": "requested | arrived | signed | received",     // optional lifecycle checkpoint
  "selection": { "folders": [], "extensions": [] },
  "summary": { "files": 0, "bytes": 0 },
  "manifest_fingerprint": "<fnv64 hex>",                    // always
  "manifest_sha256": "<hex>",                               // verified+ tiers
  "signature": { "algo": "ed25519", "pubkey": "<hex>", "sig": "<hex>" },  // signed tier
  "files": [ { "path": "rel", "bytes": 0, "sha256": "<hex>" } ]
}
```

### 5.2 Canonical record + external seal
A deterministic, prose-free block, hashed **outside** the producer. Ends with:
```
<fixed TAB-separated fields, one line per item, sorted>
anchors_digest=<anchors, whitespace-normalized, one line>
seal=[GENERATED_POST_RUN_BY_HARNESS]   # external tool runs sha256sum; the producer never fills this
```

### 5.3 Hash-chained audit log (`scribe-session.log`)
Append-only, tamper-evident. One entry per committed transaction:
```
seq=<n> <fields...> prev=<self of entry n-1 | 64 zeros> self=<sha256(line without " self=")>
```
Verify by recomputing each `self` and checking `prev` links; a break localizes to
the first altered entry.

### 5.4 Verifier interface (how a device plugs in)
A verifier proves one axis. Reads context JSON on **stdin**, writes a verdict on
**stdout**. Any failure = not satisfied (fail-closed).
```
stdin:   { "clause": "signer_perm", "value": "L2", "artifact": "deal.txt", "artifact_sha256": "<hex>" }
stdout:  { "ok": true, "detail": "signer Ryan@desk-A, perm L3 >= L2" }
```

### 5.5 Layer / axis taxonomy (the collapse space)
Each clause collapses one axis of uncertainty. Extensible; additive.

| clause | axis | collapses |
|--------|------|-----------|
| `content_hash` | content-integrity | the bytes are unchanged |
| `all_present` | completeness | every declared object is present and its seal verifies |
| `all_parties` | multi-party-compliance | every required party contributed a valid seal |
| `merkle_chain` | history-integrity | the log/order was not rewritten |
| `signer_trusted` | key-trust | the signer's key is pinned by the verify layer |
| `signer_perm` | authority | who signed, at what permission |
| `host_in` | location | which host/desk it came from |
| `distance` | proximity | physical/network nearness |
| `liveness_knock` | liveness | a live present party, not a replay |
| `zk_media` | media-provenance | audio/video authenticity, zero-knowledge |
| `cosign` | distributed-trust | multiple independent signers |
| `timestamp` | time | it existed at time T |

### 5.6 Simulation result (optional; executor → viewer)
```json
{ "dest": "...", "dest_free_bytes": 0, "fits": true, "eta_seconds": 0,
  "totals": { "copy_bytes": 0, "skip_bytes": 0, "conflicts": 0 },
  "rows": [ { "key": "...", "bytes": 0, "action": "COPY|SKIP|CONFLICT|DEDUP" } ] }
```

## 6. Attestation tiers

| tier | seal | per-txn cost | for |
|------|------|--------------|-----|
| `fast-fnv` | FNV fingerprint | ~free | high-volume, low-stakes |
| `verified-sha256` | SHA-256 content seal | one read pass | trust-but-verify |
| `signed-ed25519` | + ed25519 signature | one sign | attributable |
| *insured* | + partner-underwritten layers | fee + premium | high-stakes (off-protocol commercial layer) |

Records stay uniform and tiny at every tier, so high volumes interoperate and remain
independently verifiable.

## 7. Validation events & fee model

A validation is a **point-in-time check, requested by either party**, reporting
whether the declared numbers/objects are true at that instant. It is **metered**: a
small per-validation fee for the act itself — **charged whether or not everything is
present.** We are paid to report the truth, never to make it pass; that keeps the
validator neutral.

Lifecycle checkpoints (each an independent event; reversible):
`requested` (true at request) · `arrived` (at endpoint) · `signed` (after signing) ·
`received` (on receipt).

Each event yields a plan record + seal **or a discrepancy report** (declared vs.
present) and appends to the chained log. **On a false result we report it and stop**
— dispute, remediation, re-validation, chargeback, and claims are the parties' job.

## 8. Composition patterns

Each is just a contract selecting clauses; the gate stays fail-closed.

- **Presence bundle** — `all_present`: the declared set is here, each seal verifies.
- **Bundle-at-creation** — seal at assembly, so the bundle carries its manifest from
  birth.
- **Multi-party compliance** — `all_parties`: seals only when every required party
  contributed a valid seal (typically insured).
- **Two-point handoff** — validate at origin (seal) and endpoint (`--verify` matches);
  a match proves the same complete bundle arrived intact across a transfer we never
  touched. Reversible.
- **Escrow release-condition** — we emit "release authorized"; a bank/insured
  custodian holds the asset and releases on our attestation. We never custody it.

## 9. Design rules & conformance

A conforming implementation MUST:
1. **Be additive-tolerant** — ignore unknown fields; keep working.
2. **Fail closed** — on any unknown *required* layer or any verifier failure, REFUSE;
   never assume-pass.
3. **Produce deterministic records** — identical inputs yield byte-identical
   canonical records, recomputable and hashable by any party.
4. **Never fabricate values** — every hash/signature computed in code, independently
   checkable; seals left to external tooling where specified.
5. **Version breaking changes** — additive changes are minor; breaking changes bump
   the major and are listed in the changelog.

## 10. Governance & stewardship — the trusted neutral middle

Open, permissionless standard (MIT). Others in the stack are wanted, on purpose.

- **Competitors welcome.** Conformance is by spec, not permission. A rival that
  speaks this protocol grows the shared network; adopting it makes an "usurper" a
  participant.
- **The steward keeps the spec tight — it does not gatekeep.** Value is neutrality
  and network, not exclusivity.
- **No lock-in.** Every record is checkable with standard tools; nobody needs our
  software to verify a seal. Trust is earned through neutrality and verifiability,
  not captivity.
- **The steady hand.** Stability and neutrality are the commitment: additive, slow,
  breaking changes rare and loud. We stay the constant while the other layers find
  their ground, and let the free market decide who fills each role.

## 11. Reference implementation

- `scribe/` — Rust TUI + signer: scan → select → `--hash` (SHA-256) → `--sign`
  (ed25519) → `--verify` / `--verify-log`.
- `dial/` — set the validation rigor once; applied automatically; resumable.
- `sealgate/` — fail-closed contract gate over the verifier interface; reports the
  collapse profile.

## 12. Changelog

- **1.0** — consolidated authoritative spec. Supersedes protocol drafts 0.1–0.7
  (plan/canonical/chain/verifier/taxonomy/sim surfaces; tiers; metered validation
  events; composition patterns incl. two-point handoff and escrow; openness,
  governance, and the steady-hand stewardship commitment).
