# notestation / sealgate

The **assembly spine**: a contract declares the attestation layers it requires, and
`sealgate` **refuses to seal** until every one is satisfied by a *real* mechanism.
It never fakes a layer it can't verify — an unconfigured clause **fails closed**.
This is the whole thread's rule — Label ≤ Mechanism — turned into an enforcement
engine.

Your proven pieces (the family+permission lattice, the host/signer anchors) plug in
as **external verifier commands**; `sealgate` calls them and consumes their result,
the same honest-executor split scribe and dial use. It does not reimplement your
device — it *gates* on it.

## Use

```bash
sealgate init                                    # scaffold contract.json + verifiers.json
sealgate check contract.json deal.txt --verifiers verifiers.json   # dry run (no seal)
sealgate seal  contract.json deal.txt --verifiers verifiers.json   # seals iff all layers pass
sealgate verify deal.txt                         # re-check a sealed artifact hasn't changed
```

## Contract = writer-composed layers

```json
{ "level": "L2",
  "require": {
    "content_hash": true,       // built-in: binds the artifact's real SHA-256
    "signer_perm": "L2",        // external: your lattice proves permission >= L2
    "host_in": ["desk-A"],      // external: your host-anchor check
    "cosign": 1                 // external: N co-signers
  } }
```

Verifier map — clause → the command that proves it (your device):

```json
{ "signer_perm": "/path/to/your-device verify-signer",
  "host_in":     "/path/to/your-device verify-host" }
```

## The verifier contract (how your device plugs in)

`sealgate` runs each external verifier, sends the clause + artifact context as JSON
on **stdin**, and reads a JSON verdict on **stdout**:

```
stdin:   {"clause":"signer_perm","value":"L2","artifact":"deal.txt","artifact_sha256":"98c7…"}
stdout:  {"ok": true, "detail": "signer anchor Ryan@desk-A, perm L3 >= L2"}
```

Any failure — nonzero exit with no verdict, bad JSON, timeout, missing verifier —
is treated as **not satisfied**. Fail closed, always.

## What's built vs. yours

- **Built (proven here):** the gate, fail-closed evaluation, the `content_hash`
  built-in (real SHA-256), seal → attestation record, `verify` tamper check, an
  append-only `sealgate-audit.log`. Tested: refuse-when-unconfigured, seal-on-pass,
  tamper→TAMPERED.
- **Yours (plug in via verifiers):** `signer_perm` (family+permission lattice),
  `host_in` (host anchor), `cosign`, and any layer your device attests. Two-anchor
  signing (signer + host) lands here when FORK-01 Ring B is built to that shape.

## Why fail-closed matters

A contract that requires `signer_perm >= L2` but has no verifier for it must NOT
seal. If it sealed anyway, the "L2" claim would be theater — exactly the fabricated-
seal failure this whole project exists to prevent. So the default for any layer the
gate cannot prove is REFUSE. "More serious" is a gate, checked in code — not a vibe.

Status: v0.1 spine. Stdlib only, real SHA-256. Not yet wired to scribe/dial or to a
real device — that's the next weld, on your go.
