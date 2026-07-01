# Notestation

**Canonical home for this project.** Everything lives here — three pieces, one
system. (If you see a loose tarball copy anywhere, it is NOT canonical; this
directory is.)

## The idea

Certainty is asymptotic — you can never be 100% sure. Each attestation layer
collapses one axis of the uncertainty space; the seal is the binary that forces the
collapse. Compose the layers a contract needs; refuse to seal until every one is
satisfied by a *real* mechanism.

## The three pieces

| dir | what it is | collapses / does |
|-----|-----------|------------------|
| [`scribe/`](scribe/) | Rust TUI + real SHA-256 **signer** (`--hash`, `--verify`) | content-integrity; scan → heatmap → select → sign |
| [`dial/`](dial/) | the **rigor dial** (Python) | set the tier once, applied automatically, outage-proof/resumable |
| [`sealgate/`](sealgate/) | fail-closed **contract seal-gate** (Python) | composes layers, reports the collapse profile, refuses to seal until every layer is really satisfied |

## Design docs

All under [`scribe/`](scribe/) (kept together so their cross-references stay intact):

- `NOTESTATION-CONCEPT.md` — the family+permission lattice, two-anchor signer, the epistemic core
- `FORK-01-crypto-fingerprint.md` — the signer/attestation ladder (Rings A/B/C)
- `SPEC.md`, `SPEC-v1-LAYERED.md`, `LAYERS.md` — scribe architecture
- `PITCH.md` — three markets and the strategic fork
- `PROMPT-GRADER-COUNCIL.md`, `MEGAPROMPT-prompt-pal.md`, `PROMPT-opus-ultracode-class.md`,
  `SKILL-tiered-rigor.md`, `EVAL-prompt-pal.md`, `DOUBLE-BLIND-TEST.md` — the
  prompt-engineering method that produced this system

## Status

Each piece is built and proven standalone. Integration (wiring your attestation
device in as verifiers) is deferred until you give the go — see
`scribe/NOTESTATION-CONCEPT.md`.
