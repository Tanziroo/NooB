# Grok Build — Cheat Sheet & Everyday Companion

*The plain-language companion to the Council Report. No jargon. If you just want to use Grok well and not get burned, this is the page to keep open.*

---

## What Grok is, in one line
A coding assistant that lives in your terminal, reads your code, runs commands, edits files, and can spin up helper agents — you steer, it does the work.

---

## The 10 commands you'll actually use

| You want to… | Type this |
|---|---|
| Start fresh | `/new` |
| Switch model | `/model grok-build` |
| Stop it asking permission for everything | `/yolo` (careful — see safety below) |
| Free up space when it gets sluggish | `/compact` |
| Save the important stuff before it forgets | `/flush` |
| Undo file changes it made | `/rewind` |
| See how full its memory is | `/context` |
| Plan a big change before it codes | it offers plan mode — say yes |
| Attach a file to your message | `@path/to/file` |
| Run a check every few minutes | `/loop 5m check the tests` |

---

## "If you want X, do Y"

- **Big, fuzzy task** ("redesign the auth") → let it enter **plan mode**, read the plan, comment, *then* approve. Cheaper than redoing code.
- **A few independent jobs at once** → ask it to use **subagents** so they run in parallel without stepping on each other.
- **A task where one attempt is unreliable** → ask for **best-of-n**: it tries several ways and keeps the best.
- **Same rules every session** → put them in an **`AGENTS.md`** file in your repo. This is the single most powerful thing you can do (see below).
- **A repeatable workflow** → make a **skill** (`/skillify` right after you do it once).
- **Run it unattended** (CI, overnight) → use the safety stack below. Never unattended without it.

---

## The one trick that matters most: AGENTS.md

Drop a file called `AGENTS.md` in your project. Whatever's in it gets fed to Grok **every single turn**. It's how you stop repeating yourself.

Keep it **short, bossy, and specific**:
```
# Rules
- Run `npm test` before committing.
- Use TypeScript, functional components.
- Never edit files in /vendor.
- Commit messages: conventional commits format.
```
Why it matters: if you ever turn its memory off, this file is the *only* thing steering it. Long rambling files get ignored — keep it tight.

---

## Safety — the part that saves your weekend

**Golden rule:** the only thing that *truly* stops Grok from touching something is the **sandbox** (a kernel-level fence). Permission prompts and hooks help, but a confused agent can slip past them. The sandbox can't be talked out of.

**Safe ways to run it:**
- **Your own code, watching it:** just use it normally. Approve prompts as they come.
- **Untrusted code / not watching:** run with a sandbox →
  ```
  grok --sandbox strict          # for code you don't trust
  grok --sandbox workspace       # for your own projects, write-protected outside the folder
  ```
- **Unattended automation:** combine "don't ask, just deny anything not allowed" with a tight allow-list:
  ```
  grok -p "do the task" --permission-mode dontAsk \
    --allow 'Read' --allow 'Grep' --allow 'Bash(git *)'
  ```

**Things that are NOT safety nets** (don't trust them alone):
- The "safe commands" list — it's for convenience, not protection.
- A safety hook — if its script crashes, it silently lets things through.
- `--yolo` / `bypassPermissions` — removes the brakes. Only inside a sandbox.

**Always protected no matter what:** your `~/.ssh`, `~/.aws`, `~/.gnupg`, and Grok's own login keys. It can't write to those.

---

## Gotchas (plain words)

- **It forgets between sessions** unless memory is turned on (it's off by default). Use `/flush` to save the good stuff.
- **`/rewind` rewrites files on disk** — make sure your work is in git first.
- **Each `grok -p` starts fresh.** To continue, name the session: `grok -p "..." -s my-task`.
- **In a big monorepo** it may grab the wrong project root — add `--no-project-root`.
- **If you stop it mid-task**, files it already changed are NOT undone.
- **Skills don't always auto-trigger** — if it's important, call it by name: `/my-skill`.

---

## The thing to remember if you remember nothing else

> **Let the tools do the guarding, let the AGENTS.md do the steering.**
> Anything that *must* be true (don't delete this, don't touch that) → enforce it with the sandbox or a permission rule, not by asking nicely in a prompt. Anything you want it to *tend* to do → put it in AGENTS.md.

That one habit is the difference between a helper and a hazard.
