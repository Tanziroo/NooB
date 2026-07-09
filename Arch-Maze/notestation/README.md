# Notestation Tools

[![CI](https://github.com/KHET-1/notestation-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/KHET-1/notestation-tools/actions/workflows/ci.yml)
![license](https://img.shields.io/badge/license-MIT-blue)
![rust](https://img.shields.io/badge/rust-stable-orange)
![python](https://img.shields.io/badge/python-3.12-blue)

**Deliberate selection, honest signing, gated sealing.**

Three small tools for high-stakes work where you must decide under uncertainty and
*prove* what you decided. They share one principle:

> Certainty is asymptotic — you can never be 100% sure. Each attestation layer
> collapses one axis of the uncertainty space; the seal is the binary that forces
> the collapse. Compose the layers a contract needs; **refuse to seal until every
> one is satisfied by a real mechanism.**

Nothing here ever fakes a guarantee it can't deliver (**Label ≤ Mechanism**): a
model never computes a hash, and a gate never passes a layer it can't verify.

---

## The three tools

| Tool | Language | What it does |
|------|----------|--------------|
| **[`scribe/`](scribe/)** | Rust (TUI) | Scan a disk → heatmap of where the data weight is → select by folder/extension → **sign with real SHA-256** (`--hash`) and re-check with `--verify`. Collapses *content-integrity*. |
| **[`dial/`](dial/)** | Python | Set the rigor tier **once** (`fast` / `heavy` / `forensic`); it's applied automatically to every run, journaled atomically (outage-proof, resumable), and sealed with a real hash. |
| **[`sealgate/`](sealgate/)** | Python | A **fail-closed contract gate**. A contract composes the attestation layers it requires; `sealgate` refuses to seal until every layer is really satisfied, and reports the *collapse profile* — which axes closed, which remain. |

```
                 scribe            dial              sealgate
 signs an        picks & signs     sets the rigor    composes layers into a
 artifact   ───▶ artifacts    ───▶ tier per run ───▶ contract, seals iff all
 (real SHA-256)  (--hash/--verify)  (auto, resumable)  layers pass (fail-closed)
```

Each is proven standalone. Wiring an external attestation device (identity/permission
lattice, host + signer anchors, zk-media / liveness / distance verifiers) is done by
registering **verifier commands** with `sealgate` — see [`sealgate/README.md`](sealgate/README.md).

---

## Quickstart

```bash
# build + test everything (what CI runs)
make all

# scribe: scan, select in the TUI, sign the selection
cd scribe && cargo build --release
./target/release/scribe /path/to/scan --hash
./target/release/scribe --verify scribe-plan.json /path/to/scan

# dial: set rigor once, then every session inherits it
python3 dial/dial.py init --mode forensic
python3 dial/dial.py new --task "audit module X"

# sealgate: refuse to seal until every contract layer is real
python3 sealgate/sealgate.py init
python3 sealgate/sealgate.py seal contract.json artifact --verifiers verifiers.json
```

A sealgate run shows the collapse profile and forces the binary:

```
collapsed axes (6/9): authority, content-integrity, history-integrity, liveness, location, media-provenance
residual uncertainty (not attested): distributed-trust, proximity, time
VERDICT: SEAL PERMITTED  (binary collapse — the only 100% is the decision, not the evidence)
```

---

## Status & guarantees

- **CI-green:** `cargo fmt` + `clippy -D warnings` + `cargo test` (8 tests) + Python
  smoke tests (dial + sealgate end-to-end). See [`.github/workflows/ci.yml`](.github/workflows/ci.yml).
- **Real integrity:** SHA-256 is computed in code (Rust `sha2`, Python `hashlib`),
  verifiable with standard `sha256sum -c`. No fabricated seals.
- **Honest limits:** signing (ed25519 / two-anchor) and a hash-chained audit log are
  specced but not yet built — see [`scribe/FORK-01-crypto-fingerprint.md`](scribe/FORK-01-crypto-fingerprint.md)
  and [`scribe/NOTESTATION-CONCEPT.md`](scribe/NOTESTATION-CONCEPT.md).

## Docs

Under [`scribe/`](scribe/): the architecture (`SPEC.md`, `LAYERS.md`), the signer/
attestation ladder (`FORK-01-crypto-fingerprint.md`), the epistemic model
(`NOTESTATION-CONCEPT.md`), and the prompt-engineering method that produced the
system (`SKILL-tiered-rigor.md`, `PROMPT-*`, `DOUBLE-BLIND-TEST.md`, `PITCH.md`).

## License

MIT — see [LICENSE](LICENSE).
