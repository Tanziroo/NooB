# Grok Build — Cheat Sheet (Quick Card)

*One screen. Scan it. The apex reasoning is in `Council-Report.md`; the gentle walkthrough is in `Everyday-Companion.md`.*

## Commands you'll actually use
| Want to… | Type |
|---|---|
| Start fresh | `/new` |
| Switch model | `/model grok-build` |
| Stop the permission prompts | `/yolo` *(only in a sandbox)* |
| Free up space | `/compact` |
| Save the good stuff before it forgets | `/flush` |
| Undo file changes | `/rewind` *(commit to git first)* |
| Check context usage | `/context` |
| Attach a file | `@path/to/file` |
| Recurring check | `/loop 5m check the tests` |

## If X → do Y
- Big/fuzzy task → **plan mode**, review, then approve.
- Several independent jobs → **subagents** (parallel).
- One attempt is unreliable → **best-of-n**.
- Same rules every time → **`AGENTS.md`** (short, bossy, specific).
- Repeatable workflow → **skill** (`/skillify`).
- Unattended → safety stack below, never without it.

## Safety in three lines
- Own code, watching: use normally.
- Untrusted code: `grok --sandbox strict`.
- Unattended: `grok -p "…" --permission-mode dontAsk --allow 'Read' --allow 'Grep' --allow 'Bash(git *)'` + `--sandbox workspace`.

## Never trust these alone
Safe-command list · a hook (fail-open) · `--yolo`/`bypassPermissions`.
**Only the kernel sandbox is a hard fence.** `~/.ssh`, `~/.aws`, `~/.gnupg`, Grok's keys: always protected.

## Gotchas
Forgets between sessions (memory off by default → `/flush`) · `/rewind` writes to disk · each `grok -p` is fresh (use `-s name`) · monorepo may grab wrong root (`--no-project-root`) · stopping mid-task doesn't undo files · skills don't always auto-fire (`/call` them).

## The one rule
**Tools guard, AGENTS.md steers.** Must-be-true → enforce with sandbox/rules. Want-it-to-tend-to → put in AGENTS.md.
