# Attestation Claims & Scope

Notestation produces **evidence, not guarantees.** This document states, in plain
language, exactly what a seal claims and — just as important — what it does not.

> Not legal advice. This describes the *technical* scope of what the tools attest.
> Any liability-bearing use should be reviewed by qualified counsel and written into
> the relevant agreement.

## The claim

Each seal makes one narrow, mechanically-checkable statement:

> **"The described artifact was present, in the described byte-for-byte state, at the
> recorded time, as computed by the stated mechanism."**

That is the whole claim: a record of **presence and integrity at a moment**. The
mechanism is named on every seal (`fast-fnv` / `verified-sha256` / `signed-ed25519`,
plus any gate layers), so the claim is only ever as strong as the mechanism that
produced it.

## What it does NOT guarantee

- It does **not** guarantee the artifact is authentic, true, accurate, complete,
  lawful, owned by any party, or fit for any purpose.
- It does **not** vouch for the identity behind a signing key. A valid signature
  proves "signed by whoever holds key K." Trust in K is established **out of band**
  by the verify layer — not by us and not by the tool (see the verify-layer boundary
  in `scribe/NOTESTATION-CONCEPT.md`).
- It does **not** certify time beyond the recorded value unless an external
  timestamp authority is composed into the contract.
- **Absence** of a seal is not evidence of anything.

## Independently verifiable — we are not the sole authority

Every artifact is checkable by third parties with standard tools, **no Notestation
software required**:

- content:   `sha256sum -c scribe-manifest.sha256`
- signature: any ed25519 verifier, against the published `pubkey`
- history:   recompute the `self` / `prev` hash chain in `scribe-session.log`

Independent verification through other means is expected and encouraged. A seal is
**one line of evidence** among whatever others a relying party chooses to weigh; it
does not ask anyone to take our word for it.

## Assurance scales with the tier (and the engagement)

The strength of a claim equals exactly the layers the contract required — no more.

- **Default:** the light claim above (present + integrity + time).
- **Higher assurance:** the contract composes more layers — signer/host anchors, key
  pinning, liveness, distance, media-provenance, co-signing, external timestamp /
  notary anchoring — each collapsing another axis of uncertainty.
- **Larger engagements tighten both the mechanism and the terms.** More layers are
  required to seal, and the assurances may be backed by separate contractual
  commitments. Bigger client → tighter contract → more collapsed axes.

Certainty is never claimed as absolute: *you can never be 100% sure; each layer
collapses a vectorpoint; the seal is the binary that forces the decision.* The seal
records that decision and the evidence behind it — it does not pretend to be proof
of truth.

## No warranty

The software is provided "AS IS" under the MIT License (see `LICENSE`), without
warranty of any kind. Nothing in a seal or in this repository creates a warranty,
guarantee, or liability on the part of the authors.
