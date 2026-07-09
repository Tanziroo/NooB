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

## We are the bundle maker — we never hold value

This boundary is absolute and by design:

- Notestation **assembles, seals, and attests bundles.** That is the entire role.
- It **never holds, moves, custodies, escrows, or has title to money or any asset,**
  and it never will. It is not a bank, a payment processor, a custodian, or an escrow
  agent, and it takes no fees or interest on anything of value.
- Holding value, moving funds, and releasing an escrow require an **insured third
  party and/or a bank** — entirely separate from and downstream of us. They may
  *consume* a Notestation attestation as a release condition, but the custody, the
  funds, the payout, and the associated liability are **theirs, not ours.**
- We attest the *condition* ("all parties complied", "all objects present"); we do
  not perform the settlement.
- We also **do not transport, carry, or move the bundle itself.** A carrier does that.
  Our single function is point-in-time **validation** that all things are present and
  ready — nothing is held, moved, or transmitted by us.
- We **do not resolve, mediate, remediate, or arbitrate discrepancies.** If a
  validation returns false, we report the discrepancy (declared vs. present) and stop.
  Handling it — dispute, remediation, re-validation, chargeback, claim — is the
  responsibility of the parties and their chosen partners, not ours.

If a transaction involves value or transport, neither the value nor the goods ever
pass through Notestation. We validate the endpoints; others move and hold.

## Who carries the liability (two tiers)

The base tool and the insured layer are deliberately separate, so the free product
never quietly takes on liability it can't back:

- **Base tier — self-serve, low-stakes / trust-but-verify.** The default. It emits
  independently-verifiable evidence and assumes **no liability**. A relying party
  weighs the evidence at its own discretion — exactly as it would any other
  trust-but-verify signal. This is what the software in this repository provides.
- **Insured tier — third-party partner, higher-stakes.** Opt-in. A partner
  (auditor / notary / insurer) verifies to a higher standard, composes additional
  attestation layers, **charges a fee, and backs the attestation with an insurance
  policy.** The liability is carried by the insuring partner under that policy's
  terms — **not** by the base tool or its authors. Bigger client → insured tier →
  more layers + underwritten assurance.

The tool's job is to produce clean, independently-checkable evidence at any tier;
whether that evidence is *insured* is a separate, opt-in commercial layer.

## No warranty

The software is provided "AS IS" under the MIT License (see `LICENSE`), without
warranty of any kind. Nothing in a seal or in this repository creates a warranty,
guarantee, or liability on the part of the authors. Money-backed assurance exists
only where an insured-tier partner has explicitly underwritten it in a separate
agreement.
