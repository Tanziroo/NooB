# Grok Build — Everyday Companion

*The friendly, no-jargon walkthrough. If the Council Report is the expert reveal and the Cheat Sheet is the quick card, this is the "sit down and explain it to me like a person" version.*

---

## Think of it like this

Grok is a very capable assistant that lives in your terminal. You tell it what you want in plain English; it reads your files, runs commands, and makes changes — while you watch. It's less like a chatbot and more like handing your project to a fast, literal-minded contractor: brilliant at the work, but it does *exactly* what the setup tells it to, so the setup matters more than the asking.

The single most useful idea in this whole thing: **what you can guarantee comes from the fences you build, not from how nicely you ask.** Everything below is a version of that.

---

## How to talk to it

Just say what you want. "Fix the failing test." "Add a logout button." "Explain what this file does." It'll think, then act, showing you each step.

Two upgrades that pay off immediately:
- **For anything big or vague** ("redesign how login works"), let it **plan first**. It'll write up an approach and ask you to approve before touching code. Read it, push back, *then* say go. This saves you from it confidently building the wrong thing.
- **When one shot feels risky**, ask it to "try a few approaches and keep the best." It'll do several in parallel and pick the strongest. Great for tricky problems.

---

## The one file that changes everything: `AGENTS.md`

Make a file called `AGENTS.md` in your project folder. Anything you write in it gets whispered to Grok on *every* request, automatically, forever. It's how you stop repeating yourself.

Keep it short and bossy — like sticky notes for a new hire:

```
- Always run the tests before committing.
- Never touch anything in the /vendor folder.
- Write small, clear functions.
```

A short, blunt list gets followed. A long rambling essay gets half-ignored. Less is more.

---

## Keeping it safe (the fence analogy)

There are two kinds of "rules" you can give Grok, and the difference is everything:

- **Asking nicely** ("please don't delete files") — a suggestion. Usually works. Not a guarantee.
- **Building a fence** — a hard wall it physically cannot cross, no matter how confused it gets.

The fence is called the **sandbox**, and it's the only thing you can actually count on:

- **Working on your own stuff, watching it go:** just use it. Approve things as they pop up.
- **Running code you don't trust:** put up the fence → `grok --sandbox strict`.
- **Letting it run on its own (overnight, automation):** fence it in *and* give it a short list of what it's allowed to do. Never let it run unsupervised without both.

Good news: your passwords and keys (`~/.ssh`, `~/.aws`, and friends) are **always** off-limits to it, fence or no fence.

The trap to avoid: thinking a polite instruction is a fence. If something absolutely must not happen, build the wall — don't just ask.

---

## When something goes sideways

- **It "forgot" what we did yesterday.** Normal — its memory is off unless you turn it on. Type `/flush` to save the important parts of a session before they're gone.
- **It changed files I didn't want.** Type `/rewind` to roll back — but make sure your work is saved in git first, because rewind rewrites what's on disk.
- **I stopped it halfway and things look messy.** Stopping doesn't undo changes it already made. Use git to get back to a clean state.
- **It's getting slow / forgetful in a long session.** Type `/compact` to tidy up, but `/flush` first so nothing important is lost.

---

## The single habit to keep

> **Let the fences guard. Let `AGENTS.md` steer.**

If it *must* be true, build a fence (sandbox / a rule it can't break). If you just want it to *lean* a certain way, write it in `AGENTS.md`. Get that one distinction right and Grok is a powerful helper instead of a thing you have to babysit.
