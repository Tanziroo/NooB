# dial

**Set the rigor once. Apply it automatically every time.**

The tiered-rigor discipline (FAST / HEAVY / FORENSIC) shouldn't depend on you
remembering to paste it, on the model inferring it, or on surviving an interruption
right when you need it. `dial` makes it a **dial you set once** — then it:

- **assembles the correct operating prompt automatically** on every session (the
  laws, the internal council, the integrity model — prepended for you);
- **journals every step to disk atomically**, so a power outage loses nothing and
  `dial resume` picks up exactly where you were;
- **computes the real integrity seal in code** (SHA-256 via stdlib) — the one thing
  the model cannot honestly do itself.

No dependencies. Python 3 stdlib only. Real `hashlib` SHA-256.

## Use

```bash
dial init --mode heavy        # set the dial ONCE (fast | heavy | forensic)

dial new --task "audit this module for races" --inputs src/mod.rs
#   → writes .dial/sessions/<id>/prompt.txt with the HEAVY operating prompt
#     already prepended. Paste it into your model.

#   save the model's reply to     .dial/sessions/<id>/response.txt
#   (forensic mode: save its CANONICAL RECORD block to  record.txt)

dial seal <id>                # computes the real sha256, appends to audit.log
dial status                   # list sessions + states
dial resume                   # after any interruption: what's unfinished + next step
```

## The three tiers (the dial)

| mode | for | adds |
|------|-----|------|
| `fast` | low-stakes, throughput | dense stance + checkable laws |
| `heavy` | apex reasoning | + internal rotating council, self-verify pass, pre-output self-audit |
| `forensic` | defensible/auditable | + canonical record + **external sha256 seal** (dial computes it) |

## Why it's built this way

- **Dial applied automatically** — `config.json` is read every run; you never infer
  or remember the mode. (Override one session with `--mode` if needed.)
- **Outage-proof** — every state transition is an atomic write (`tmp` + `os.replace`
  + `fsync`). `dial resume` reads the on-disk journal; nothing is held only in
  memory. This is the "power outage right when I need it" fix.
- **Honest integrity by construction** — the model emits a deterministic canonical
  record; `dial` (code) computes the SHA-256. Label ≤ mechanism: the seal is real
  because a program made it, not a language model. Verify any seal independently:
  `sha256sum .dial/sessions/<id>/record.txt`.
- **Append-only audit** — `.dial/audit.log` records every `new` and `seal` with its
  hash; never rewritten.

## Status

v0.1 — the spine works end-to-end (init → new → seal → resume), verified against an
independent `sha256sum`. It prepares prompts and seals results; it does not (yet)
call a model API — that keeps it model-agnostic and dependency-free. Roadmap:
optional model-call adapter, hash-chained audit log (tamper-evident), and a
`dial verify` that re-checks a sealed record against its stored digest.
