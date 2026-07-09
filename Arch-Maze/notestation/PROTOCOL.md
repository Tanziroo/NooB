# Notestation Protocol

`protocol_version: 0.5`

Notestation is a **middle layer**: the tight, versioned protocol that small
attestation transactions flow through. It does not own the endpoints — it owns the
formats and interfaces everyone conforms to, and keeps them tight and current.

```
   relying parties · insured partners        (above: judge trust, underwrite)
                    ▲
   ── the protocol (this document) ──         (middle: uniform records + interfaces)
                    ▼
   attestation devices · anchors · verifiers  (below: prove individual axes)
```

The tools in this repo are one conforming implementation. Anyone can build another
that speaks the same protocol; that is the point.

---

## Design rules (what "tight" means)

1. **Additive-by-default.** New fields/layers are minor versions. Consumers **must
   ignore unknown fields** and keep working.
2. **Fail-closed on unknown *required* layers.** A verifier/gate that doesn't
   understand a required clause must REFUSE, never assume-pass.
3. **Deterministic records.** The same inputs produce byte-identical canonical
   records, so any party can recompute and hash them independently.
4. **No fabricated values.** Every hash/signature is computed by code and
   independently checkable (`sha256sum`, any ed25519 verifier). Label ≤ Mechanism.
5. **Breaking changes bump the major** and are listed in the changelog below.

---

## Surface 1 — Plan record (`scribe-plan.json`)

The unit of a transaction: what was selected, and its seal.

```json
{
  "scribe_version": "0.6.0",
  "root": "/path",
  "attestation_tier": "fast-fnv | verified-sha256 | signed-ed25519",
  "event": "requested | arrived | signed | received",   // optional: lifecycle checkpoint
  "selection": { "folders": [], "extensions": [] },
  "summary": { "files": 0, "bytes": 0 },
  "manifest_fingerprint": "<fnv64 hex>",            // always
  "manifest_sha256": "<hex>",                        // verified+ tiers
  "signature": { "algo": "ed25519", "pubkey": "<hex>", "sig": "<hex>" },  // signed tier
  "files": [ { "path": "rel", "bytes": 0, "sha256": "<hex>" } ]
}
```

## Surface 2 — Canonical record + external seal

A deterministic, prose-free block a party hashes **outside** the producer. Ends with:

```
<fixed TAB-separated fields, one line per item, sorted>
anchors_digest=<anchors, whitespace-normalized, one line>
seal=[GENERATED_POST_RUN_BY_HARNESS]      # external tool runs sha256sum; the producer never fills this
```

## Surface 3 — Hash-chained audit log (`scribe-session.log`)

Append-only, tamper-evident. One entry per committed transaction:

```
seq=<n> <fields...> prev=<self of entry n-1 | 64 zeros> self=<sha256(line without " self=")>
```

Verify by recomputing each `self` and checking `prev` links. Breaks localize to the
first altered entry.

## Surface 4 — Verifier interface (how a device plugs in)

A verifier proves one axis. It reads context JSON on **stdin**, writes a verdict on
**stdout**. Any failure = not satisfied (fail-closed).

```
stdin:   { "clause": "signer_perm", "value": "L2", "artifact": "deal.txt", "artifact_sha256": "<hex>" }
stdout:  { "ok": true, "detail": "signer Ryan@desk-A, perm L3 >= L2" }
```

## Surface 5 — Layer / axis taxonomy (the collapse space)

Each clause collapses one axis of uncertainty. Registry (extensible; additive):

| clause | axis | collapses |
|--------|------|-----------|
| `content_hash` | content-integrity | the bytes are unchanged |
| `all_present` | completeness | every declared object in the set is present and its seal verifies |
| `all_parties` | multi-party-compliance | every required party contributed a valid seal (escrow-release condition) |
| `merkle_chain` | history-integrity | the log/order was not rewritten |
| `signer_trusted` | key-trust | the signer's key is pinned by the verify layer |
| `signer_perm` | authority | who signed, at what permission |
| `host_in` | location | which host/desk it came from |
| `distance` | proximity | physical/network nearness |
| `liveness_knock` | liveness | a live present party, not a replay |
| `zk_media` | media-provenance | audio/video authenticity, zero-knowledge |
| `cosign` | distributed-trust | multiple independent signers |
| `timestamp` | time | it existed at time T |

## Surface 6 — Simulation result (optional, executor → viewer)

```json
{ "dest": "...", "dest_free_bytes": 0, "fits": true, "eta_seconds": 0,
  "totals": { "copy_bytes": 0, "skip_bytes": 0, "conflicts": 0 },
  "rows": [ { "key": "...", "bytes": 0, "action": "COPY|SKIP|CONFLICT|DEDUP" } ] }
```

---

## Transaction tiers (throughput vs. assurance)

Small transactions stay cheap; assurance is opt-in per contract.

| tier | seal | per-txn cost | for |
|------|------|--------------|-----|
| `fast-fnv` | FNV fingerprint | ~free | high-volume, low-stakes |
| `verified-sha256` | SHA-256 content seal | one read pass | trust-but-verify |
| `signed-ed25519` | + signature | one sign | attributable |
| *insured* | + partner-underwritten layers | fee + premium | high-stakes (off-protocol commercial layer) |

The middle layer's job: keep every tier's record **uniform and tiny**, so millions
of small transactions interoperate and remain independently verifiable.

---

## Validation events (metered, party-requested)

A validation is a **point-in-time check, requested by either side**, that reports
whether the declared numbers/objects are true *at that instant*. It is metered: a
**small per-validation transaction fee** is charged for the act itself — **whether or
not everything is present.** We are paid to report the truth of the state, never to
make it pass. That incentive keeps the validator neutral.

Typical lifecycle checkpoints — each an independent validation event, reversible:

| event | asserts |
|-------|---------|
| `requested` | the numbers are true at the time of the validation request |
| `arrived`   | true on arrival at the endpoint |
| `signed`    | true after signing |
| `received`  | true on receipt |

Either party may request any event. Each produces a plan record + seal **or a
discrepancy report** (present vs. declared), and appends to the hash-chained log.
High volume × small fee is the model — the middle layer collects many small, honest
validations; it moves and holds nothing.

**On a false / discrepancy result, we report it and stop.** Resolving it — dispute,
remediation, re-validation, chargeback, claim — is the **other parties'** job, not
ours. We surface the fact ("declared X, present Y"); they do the work.

The validator is deliberately **thin**. Every downstream action — transport,
custody, settlement, dispute resolution, insurance — is a distinct role for another
party. **Minimal validator, maximal ecosystem:** staying small is what creates the
jobs around it.

---

## Composition patterns

The same surfaces compose into higher-order transactions. Each is just a contract
selecting which clauses it requires — the gate stays fail-closed.

- **Presence bundle ("all objects present").** The base trust: the plan record is a
  manifest of a set; `all_present` collapses *completeness* — every declared object
  is present with a verifying seal. Default "trust the set is here."
- **Bundle-at-creation.** Seal at the moment a bundle/package is assembled, so it
  carries its own manifest + seal from birth. The producer attests presence;
  downstream parties re-verify independently.
- **Multi-party compliance ("all parties comply").** A contract requiring
  `all_parties` seals only when every required party has contributed a valid seal
  (each proving its own axes). Typically an **insured tier** — a partner underwrites
  the "all complied" attestation.
- **Two-point handoff (validate at origin and endpoint; reversible).** Validate the
  bundle at the **start** (`all_present` → seal with `manifest_sha256` / signature) and
  again at the **endpoint** (recompute → `--verify` the seal matches). A match proves
  the same complete bundle arrived **intact and unchanged** across a custody transfer
  — **without Notestation transporting it.** A carrier moves the bundle between the two
  gates; we only validate at each. Runs either direction (origin→endpoint, or the
  return leg). This is the whole non-liable act: point-in-time validation, twice.
- **Escrow release-condition (we are NOT the custodian).** Notestation only emits the
  compliance attestation — "`all_parties` / `all_present` collapsed; release
  authorized." A **bank or insured custodian** holds the funds/keys/asset and performs
  any release, *consuming* our attestation as a condition. Notestation never holds,
  moves, or has title to value, and carries none of that liability. Custody, funds,
  and payout are theirs; the sealed bundle + attestation are ours.

## Changelog

- **0.5** — on a false result the validator reports the discrepancy and stops;
  resolution/dispute/remediation is the parties' job. Thin validator, ecosystem of
  downstream roles.
- **0.4** — add metered validation events (`requested` / `arrived` / `signed` /
  `received`), either-party request, small per-validation fee charged regardless of
  outcome, optional `event` field on the plan record.
- **0.3** — add the two-point handoff pattern (validate at origin + endpoint,
  reversible); reinforce that Notestation only validates — it does not transport,
  hold, or custody anything.
- **0.2** — add `all_present` (completeness) and `all_parties`
  (multi-party-compliance) clauses; document composition patterns (presence bundle,
  bundle-at-creation, multi-party compliance, escrow).
- **0.1** — initial: plan record, canonical record + external seal, hash-chained
  log, verifier interface, axis taxonomy, sim-result. Tiers fast/verified/signed.

Scope of what a seal claims (and does not) is in
[`ATTESTATION-CLAIMS.md`](ATTESTATION-CLAIMS.md).
