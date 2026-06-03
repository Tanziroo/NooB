# Grok Build — Council Report

*A four-seat expert breakdown of the Grok Build CLI agent: its patterns, how to get the most work out of them, and in what situations. Read through the "high-signal" lens — what actually moves outcomes on a non-deterministic frontier model vs. what's convenience or plumbing.*

---

## Chair's headline finding (the convergence)

Four seats analyzed four disjoint slices of the system — Orchestration, Context, Extensibility, Safety. **They independently arrived at the same dividing line**, and it's the most important thing in this report:

> **The patterns that hold up are the ones enforced *outside* the model. The patterns that wobble are the ones that *trust the model's behavior*.**

- **Robust / load-bearing** — enforced by tooling, git, or kernel, so they hold even when the model misbehaves stochastically: capability modes, worktree isolation, permission rules, the OS sandbox, `--max-turns`, file snapshots/`/rewind`, deterministic config load.
- **Fragile (still valuable, but needs a backstop)** — depends on the model making a good call: skill auto-invocation, persona file-handoff contracts, plan-mode entry, first-turn memory injection (`min_score 0.0`), best-of-n's self-evaluation, `/loop` per-fire usefulness.
- **Plumbing** — provenance, storage, transport. Necessary, zero effect on answer quality.

**The operating rule that falls out of this:**
> Put **guarantees** in the deterministic layer (tools / git / kernel). Put **steering** in the first frame (`AGENTS.md` + persona). **Pair every model-trusting pattern with a tool-enforced guardrail.**

This is the same conclusion the non-determinism discussion reached: on a frontier model you can't lock the substrate, so only *robust-in-distribution* patterns matter — and "robust" almost always means "enforced outside the stochastic part."

---

## Master table — every pattern, one verdict

**Legend:** 🟢 Load-bearing (moves outcome quality) · 🟡 Convenience (ergonomics, degrades gracefully) · ⚪ Plumbing (infra/audit) · ⚠️ Fragile under non-determinism (needs a backstop)

### Orchestration & Autonomy
| Pattern | Verdict | Note |
|---|---|---|
| Subagents (`task`, parallel) | 🟢 | Context isolation genuinely raises quality; fan out in one message. |
| Agent types (`explore`/`plan`) | 🟢 | Tool-enforced roles (`explore` *can't* write). Robust. |
| Capability modes (read-only/…/all) | ⚪🟢 | Tool-level least-privilege; robust *because* it doesn't trust the model. |
| Worktree isolation | ⚪ | Deterministic git; the backbone that makes parallel writers safe. |
| `resume_from` chaining | 🟢 | Stage-2 inherits stage-1 context cheaply. |
| Personas | 🟡 / 🟢⚠️ | Tone = convenience; the file-handoff IO contracts are load-bearing but fragile (chain breaks if model skips a write). |
| Headless (`grok -p`) + `-s/-r/-c` | ⚪🟡 | Execution substrate; value rides on disciplined session-ID hygiene. |
| `--best-of-n` | 🟢⚠️ | *Designed* to beat non-determinism (sampling diversity + pick winner) — but the evaluator is the same stochastic model (single fragile point). |
| `--effort` / `--check` | 🟢 | Change quality. `--max-turns` ⚪ is a robust safety bound. |
| `--yolo` | 🟡 | Removes prompts; shifts all safety onto permission rules + sandbox. |
| Background / `monitor` / `/loop` | 🟡⚪ | Ergonomics & transport; `/loop` ⚠️ drifts across unattended fires with no human gate. |

### Context & Reasoning
| Pattern | Verdict | Note |
|---|---|---|
| First-turn memory injection | 🟢⚠️ | *The* behavioral seed when memory's on — but `min_score 0.0` = no filter = anchoring risk. |
| `/flush` | 🟢 | Only path to durable *rich* knowledge (auto-save is metadata-only). |
| Plan mode | 🟢⚠️ | Gates whether ambiguous work goes down the right path; entry depends on model self-assessment (can't force on). |
| Hybrid search + source weights | 🟢 | Determines what the model actually recalls (vec 0.7 / BM25 0.3, project > global). |
| `/dream`, MMR, temporal decay, `/rewind`, `/compact` ctx | 🟡 | Improve quality, degrade gracefully. `/rewind` writes to disk — have git first. |
| Session storage / watcher / embeddings / `/context` | ⚪ | Mechanical. |

### Extensibility & Integration
| Pattern | Verdict | Note |
|---|---|---|
| **AGENTS.md / project rules** | 🟢⚠️ | The persistent **first frame** — highest-leverage steering surface. Robust load, fragile *influence* (diluted by length/depth/under-weighting). Keep terse, imperative, high-priority-first. |
| Skills | 🟢⚠️ | Encoded procedure (2nd first-frame surface). Auto-invocation is a model judgment off `description` — the most non-deterministic link. Make must-run skills explicitly `/called`. |
| Hooks (PreToolUse) | 🟢⚠️ | Deterministic (harness runs them) → most reliable enforcement — *but* fail-open: a crashed/`jq`-missing guard silently allows. Only load-bearing if it provably emits `deny`. |
| MCP servers | 🟢⚪ | Define capability reach; tool *selection* inherits model non-determinism. Wiring is robust plumbing. |
| Plugins | ⚪ | Distribution/packaging over the above. |
| Custom models | ⚪ | Wiring is mechanical, but swapping the model re-rolls *all* downstream non-determinism — re-validate everything fragile. |

### Safety & Control
| Pattern | Verdict | Note |
|---|---|---|
| **OS sandbox** (Landlock/Seatbelt) | 🟢 **HARD** | Kernel-enforced, irreversible, no runtime escape. *The* containment boundary — survives total policy bypass. |
| Sensitive-path protection | 🟢 **HARD** | `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.grok/auth/` always write-protected. |
| `dontAsk` mode | 🟢 | Default-deny = safe failure direction. |
| Policy rules (deny > allow) | 🟢 | Deterministic given correct rules; bypassable by unanticipated command shapes. |
| PreToolUse deny-hook | 🟢⚠️ | Soft + fail-open; robust only if bulletproof. |
| Per-segment shell parsing | 🟢 | Catches `ls && rm -rf /`; obfuscation/subshells are the soft spot. |
| Safe-shell fast-path list | 🟡 | Explicitly **"not a security boundary"** — prompt-fatigue reducer only. |
| `bypassPermissions` / `acceptEdits` | 🟡 | Anti-safety conveniences; only the sandbox saves you. |
| Network seccomp | 🟢 (partial) | Blocks child-proc egress; in-process web tools remain an exfil path. |
| Auth precedence / event log | ⚪ | Plumbing / observability. |

---

## How to get the most work out of it — by domain

**Orchestration.** Fan out independent work as parallel `task` calls in one message. Use `explore` (read-only) for investigation and `plan` to front-load design. Chain stages with persona IO contracts (`researcher.summary_file` → `implementer`) and `resume_from`. Run real parallel *writers* in worktrees, then apply the winner via `best-of-n`. Bound runaway loops with `--max-turns`; raise quality with `--effort high` and `--check`.

**Context.** If you need cross-session continuity, turn memory on (`[memory] enabled=true`), curate `MEMORY.md` with durable "remember" statements, and `/flush` before every compaction (auto-save keeps metadata only). Raise `initial_injection.min_score` toward `0.35` to kill the anchoring risk. Use plan mode only for genuine architectural forks; steer via line-range comments in plan review before approving.

**Extensibility.** Treat `AGENTS.md` as your highest-leverage surface: actionable, project-specific, imperative, under 10k chars, deeper files for divergent conventions. Make skills auto-fire by writing concrete trigger phrases in `description`; make critical ones explicitly `/callable`. Put real guarantees in `PreToolUse` deny-hooks, not prose. Prefer native `url` MCP servers over `npx` proxies.

**Safety.** Stack four independent layers for safe autonomy: `dontAsk` + narrow allow rules + a restrictive `git-gh-only`-style hook + `--sandbox`. Loosen the *narrowest sufficient* layer to widen work — never drop to `bypassPermissions`.

---

## Situational playbook

| Scenario | Use | Don't rely on |
|---|---|---|
| **Daily driver (own code)** | `default` mode + narrow `Bash(...)` allows; optional `--sandbox workspace`. | `acceptEdits` (removes review checkpoint). |
| **CI / headless automation** | `dontAsk` + explicit allows + `git-gh-only` hook; `XAI_API_KEY`; `workspace`/`strict` sandbox; named `-s` sessions; `--max-turns`; `--output-format json`. | Interactive prompts (none exist — `dontAsk` *denies*). Hook deps must exist (fail-open). |
| **Reviewing untrusted code** | Full stack: `dontAsk` + `deny edit/bash` + **`--sandbox strict`**; do NOT `/hooks-trust` the repo. | The permission layer alone — the **kernel sandbox** is the real boundary. |
| **Long ambiguous task** | Plan mode → review comments → approve; subagents for research; `best-of-n` for high-variance work. | Auto-compact to preserve nuance (`/flush` first). |
| **Unattended / YOLO** | `--yolo` *inside* `--sandbox workspace` + `--rules`/`--deny` fences. | Safe-bash list as protection (it isn't a boundary). |

---

## The high-signal rule (what to remember)

1. **Guarantees go in the deterministic layer** — kernel sandbox, capability modes, permission rules, worktrees, `--max-turns`. These hold no matter what the model does.
2. **Steering goes in the first frame** — `AGENTS.md` + persona. With memory off, this is the *sole* behavioral seed; keep it short and imperative.
3. **Every model-trusting pattern gets a backstop** — best-of-n writers run in worktrees under capped capability modes behind `--deny` rules; must-run skills are explicitly called; safety hooks must provably emit `deny`.
4. **A pattern earns "high signal" only if its effect survives the substrate's noise.** Fragility-under-noise is grounds for rejection, not a tuning problem.

## Fragility watchlist (the model-trusting links to watch)

- Skill **auto-invocation** (description matching) — make critical skills explicit.
- Persona **file-handoff contracts** — a skipped write breaks the chain.
- Plan-mode **entry** — model decides; can't force on.
- First-turn **injection** at `min_score 0.0` — anchoring; raise the floor.
- best-of-n **self-evaluation** — evaluator is the same stochastic model.
- `/loop` **per-fire quality** — drifts unattended with no human gate.
- **Fail-open hooks** — a crash silently allows.
