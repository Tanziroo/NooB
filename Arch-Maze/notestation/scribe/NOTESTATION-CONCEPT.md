# Notestation — Concept Capture (operator's model)

Status: **captured, not built.** Integration is deliberately deferred until each
piece has proven standalone value. This note exists so the model isn't lost, and
so the signer design (FORK-01 Ring B) is built to the *right* shape when its turn
comes. Nothing here is a commitment to build now.

---

## The pieces (all exist independently today or in spec)

- **scribe** — the selector/viewer + real SHA-256 signer (v0.5, shipped).
- **dial** — the rigor dial: set once, applied automatically, resumable (v0.1).
- **attestation ladder** — composable seal layers (FORK-01 §12).

## The operator's model (three refinements to fold in)

### 1. The attestation device = family + permissions = one lattice
Identity is not a flat key. It is a partially-ordered structure:
  identity (family member / role)  ×  permission (what they may do)
A lattice, so policy can be expressed as a threshold: "signer at or above
permission P". Meets/joins let you compose roles. This replaces "a signing key"
with "a position in a permission lattice."

### 2. The signer = a two-anchor attestation "at desk"
A signature binds a PAIR of anchors, not one key:
  • HOST anchor   — *where*: the desk / host login / machine context.
  • SIGNER anchor — *who*:   the identity drawn from the family+permission lattice.
An attestation is therefore a signed statement over:
  { signer_anchor, host_anchor, permission_proof, sealed_artifact_hash, time }
"This identity, at this host, with this permission, sealed this exact artifact."
Losing either anchor invalidates it — host-bound AND identity-bound.

### 3. Contracts are writer-composed layers
The seal requirement is a menu the contract *writer* assembles, not a fixed rung:
  fingerprint · content-hash · signer-anchor sig · host-anchor sig ·
  permission-threshold · hash-chained log · external timestamp · obfuscation ·
  multi-party co-sign.
A contract declares which layers it needs; it will not seal until each declared
layer is actually produced (Label ≤ Mechanism, enforced in code).

## How this upgrades the current design

- **FORK-01 Ring B was under-specified.** It assumed a single ed25519 key. The
  correct shape is a **two-anchor signature**: sign the artifact under the SIGNER
  anchor AND bind the HOST anchor (host key / login context), plus attach a
  permission proof from the lattice. Build Ring B to that shape, not to one key.
- **The contract layer becomes a policy object,** e.g.:
  `require: { content_hash, signer_perm >= L2, host_in: [desk-A], cosign: 1 }`
  scribe/Notestation refuses to seal until every clause is satisfied by a real
  mechanism.
- **The lattice is the missing primitive.** dial dials rigor; scribe signs
  artifacts; the family+permission lattice decides WHO may seal at WHAT level. It
  is the authority layer both of the others plug into.

## Value-first sequencing (the deferral, made concrete)

Assemble only after each piece earns it. Proposed "value established" gates:

| Piece | Value gate (before integration) |
|-------|--------------------------------|
| scribe signer | one real rescue/selection sealed + `--verify` passes on real data |
| dial | one real workflow run start→seal→resume through an actual interruption |
| attestation ladder | one contract expressed as composed layers, sealed + verified |
| lattice / two-anchor signer | Ring B built + a signer-anchor and host-anchor bound in one signature |

Integration (the Notestation cockpit) starts only when the pieces above are each
independently proven — not before.

## Open questions (for when we resume)

- What concretely marks "value established" for you — a working demo, a paying
  use, a defended contract? That is the trigger to start assembling.
- Is the host anchor a key, a login/TPM binding, or both?
- Does the permission lattice live in a file, or in the attestation device itself?

---

## The epistemic core (operator's principle)

> "You can never truly be 100% sure unless you binary the choice, but each layer
> collapses vectorpoints."

This is the North Star of the whole attestation model:

1. **Certainty is asymptotic.** No stack of evidence reaches 1.0. More layers move
   you toward the decision boundary; they never arrive at it.
2. **The only 100% is the binary.** Certainty exists only at the moment you *force
   the collapse* — seal / refuse, trust / don't. It is manufactured by the
   decision, not discovered in the evidence.
3. **Uncertainty is a vector space, not a scalar.** Its axes: content-integrity,
   history-integrity, authority, location, proximity, liveness, media-provenance,
   distributed-trust, time. (Extensible.)
4. **Each layer collapses one axis.** A layer removes a degree of freedom the
   adversary could hide in. It does not empty the space — it collapses a
   vectorpoint. Compose more layers → collapse more axes → the possibility space
   shrinks toward the decision.
5. **The residual is never empty, and honesty requires naming it.** A seal must
   report which axes it collapsed AND which it did not. Claiming a collapsed axis
   you did not attest is the fabricated-seal failure (Label ≤ Mechanism).

**Operationalized in `sealgate`:** each contract clause maps to an axis
(`KNOWN_LAYERS`). The report prints the collapse profile — "collapsed axes
(6/9) … residual uncertainty (not attested) …" — and the verdict is explicitly the
binary collapse, "the only 100% is the decision, not the evidence." Verified: full
stack collapses 6/9 and seals; drop the device and it collapses 1/9 and refuses.

**Where the new pieces land** (each a collapse layer, plugged in as a verifier):
merkle chaining → history-integrity · distance layers → proximity · knock/liveness
→ liveness · zk-proof audio/video → media-provenance. The harnesses test that each
layer actually collapses its axis before a contract is allowed to rely on it.

---

## The verify-layer boundary (keystone)

> "Trusting a public key still requires pinning out of band — because the point is
> **we are the verify layer.**"

A hard separation, deliberately:

- **scribe SIGNS. It does not judge trust.** `--verify` reports "signature valid for
  key K" and stops. It never decides that K is *authorized* — a signer that certifies
  its own key's trust is circular (the fabricated-trust anti-pattern). scribe hands
  you a fact; it does not manufacture confidence in the identity.
- **The verify layer OWNS trust.** Which keys are pinned, which signer/host anchors
  are authorized, what permission the lattice grants — that judgment lives in the
  operator's attestation device, not in the tool that produced the signature.
- **The seam is the seal-gate.** Key-trust is a *contract clause*
  (`signer_trusted` -> axis `key-trust`), satisfied by the verify layer as an external
  verifier — not something scribe self-asserts. A contract composes both independently:
  - `content_hash` + `signature valid`  <- scribe proves these (mechanism it holds)
  - `signer_trusted` (key pinned) + `signer_perm` + `host_in`  <- the verify layer proves these

This is Label <= Mechanism applied to identity: scribe claims only "signed by K"
(which it can prove); the claim "K is trusted" is made only by the layer that
actually holds that authority — you.
