# scribe — Pitch & Strategic Direction

## The one-liner

**scribe makes the unknown known, lets a human decide, and proves what they
decided.** Deliberate selection under uncertainty — with a simulation hook to
foresee the outcome and a tamper-evident trail to defend it.

## The primitive (why it generalizes)

Strip scribe to its core and it isn't a backup tool. It's a **decision engine**
with five composable moves:

```
SEE      heatmap        — where is the weight?
NARROW   filter         — scope the candidate set
DECIDE   select         — pick under that scope
NAME     fingerprint    — one hash = one exact set
FORESEE  executor sim   — what happens if I commit this?
PROVE    audit log      — append-only record of the decision
```

Every use case below is the *same primitive* pointed at a different fear. That's
the whole thesis: rescue is use case #1, not the product.

---

# Use Case 1 — Field Data Rescue

**Buyer:** repair shops, MSPs, sysadmins staring at a dying disk on a live USB.

**Reverse-Socratic descent (end → cause):**
- Why would they pay? → To not lose data that can't be recreated.
- Why can't `rsync` do that? → Because they don't know what's on the disk, or
  which of it matters. rsync copies; it doesn't *decide*.
- Why does deciding blind fail? → Because the disk is full of noise (caches,
  coredumps, OS files) hiding the 4 folders that actually matter.
- **Terminal anchor:** *The product is the decision, not the copy.* scribe sells
  the five minutes of seeing-and-choosing, not the bytes moved.

**Why scribe specifically:** heatmap collapses "what's here?" to a glance;
filter + select collapse "what do I take?" to a few keystrokes; the sim answers
"will it fit / how long?" *before* a single byte moves on a flaky drive you may
only get one read pass from.

### Direction-shaping questions

**Q1.1 — Artisan tool or fleet platform?**
Is scribe one tech / one disk at a kitchen table, or does an MSP run 40 rescues a
week and need a central console, queue, and per-job reporting? The fork is brutal:
a beautiful CLI vs. a multiplayer platform with auth, storage, and dashboards.
Same core, completely different company. *Which customer writes the check?*

**Q1.2 — Cockpit or autopilot?**
Does the human stay in the loop forever, or is the endgame a `--profile medical`
that auto-selects so a junior tech (or a script) never decides at all? If the moat
is UX, you invest in the TUI. If the moat is the *selection policy*, the TUI is
scaffolding you'll throw away. *Are we making the human better, or unnecessary?*

---

# Use Case 2 — Forensic & Legal Chain-of-Custody

**Buyer:** DFIR firms, corporate eDiscovery/legal, incident response, law
enforcement imaging.

**Reverse-Socratic descent:**
- Why would they pay? → To defend a data collection under audit or in court.
- Why is that hard? → Because the killer questions are *"what exactly did you
  take?"* and *"can you prove the set didn't change between collection and
  production?"* — and ad-hoc rsync answers neither.
- Why does scribe answer them? → The fingerprint names the exact set; the
  append-only session log timestamps the decision; the journal records intended
  vs. actual. The selection becomes **evidence about the evidence.**
- **Terminal anchor:** *The artifact isn't the data — it's the provable record of
  the decision.* In this market the audit trail IS the product; the files are
  incidental.

**Why scribe specifically:** nobody else treats *selection* as a first-class,
hashable, logged event. Imaging tools copy whole disks; scribe captures the
human judgment of *what was responsive/relevant* and makes that judgment
tamper-evident. That's the exact gap between "we collected data" and "we can
defend our collection."

### Direction-shaping questions

**Q2.1 — Convenience hash or court-admissible crypto?**
Today the fingerprint is FNV-1a — fast, deterministic, *not* cryptographic. For
this market it must be SHA-256 over content (not just path+size), with signed,
write-once manifests. That single decision gates whether you can even enter
regulated/legal markets. *Is the fingerprint a UX nicety or a legal instrument?*

**Q2.2 — Who is the adversary of the log?**
An append-only file defends against *accident*. It does nothing against a
*motivated insider* who edits the log. Court-grade custody needs a hash-chained
ledger and external notarization (timestamp authority / write-once media). The
answer rewrites the entire trust architecture. *Are we defending against mistakes,
or against people?*

---

# Use Case 3 — Reproducible Dataset Curation (ML / Data Engineering)

**Buyer:** ML teams and data engineers carving training sets out of messy data
lakes.

**Reverse-Socratic descent:**
- Why would they pay? → Because *"which exact data trained this model?"* is
  almost never answerable six months later — and that kills reproducibility,
  audits, and debugging.
- Why is it unanswerable? → Selections are ad-hoc globs in a notebook, never
  named or versioned. The set drifts; nobody can reconstruct it.
- Why does scribe fix it? → The fingerprint is a **dataset version number**. The
  heatmap surfaces class/size imbalance. The executor sims the *consequence* —
  token count, cost, balance — before you burn GPU hours.
- **Terminal anchor:** *The fingerprint is a dataset version; the sim is a dry-run
  of the experiment.* You're selling reproducibility and a cheap look-before-you-
  spend, not file management.

**Why scribe specifically:** the `--json` + executor hooks already make it
headless-friendly. A curation step that emits a hash + a cost/quality projection
slots straight into an ML pipeline — and gives every model a provable lineage to
its exact training set.

### Direction-shaping questions

**Q3.1 — UI tool or policy-as-code?**
Is this an interactive curation cockpit, or a *headless data contract* that runs
in CI with a declarative selection spec and no TUI at all? scribe already leans
headless (`--json`, executor). Leaning in means selection becomes versioned code
in the repo, diffable in PRs. *Is the deliverable a session, or a checked-in
spec?*

**Q3.2 — Does the sim model cost, or quality?**
An executor that sims bytes/ETA is ops plumbing. One that sims *class balance,
near-duplicate rate, and train/test leakage* is doing ML-aware analysis — and
that's where the real money and the mod ecosystem live. The fork decides whether
the executor hook is a pipe or the actual product. *Is the mod plumbing, or the
brain?*

---

# Synthesis — the one fork that dominates all three

Across every use case the question underneath the questions is the same:

> **Is scribe a rescue tool that grew features, or a general "deliberate
> selection engine" where rescue is merely use case #1?**

- If it's a **tool**, optimize the TUI, ship rescue-specific polish, win repair
  shops, stay small and excellent.
- If it's an **engine**, the TUI is one frontend; the real product is the
  primitive — *see → narrow → decide → name → foresee → prove* — exposed as a
  library/protocol that forensic, ML, and migration tools build on.

The five-layer lattice you already built (fingerprint, approval gate, journal,
filter, audit) is suspiciously general for a "backup tool." That's the tell. The
architecture is already voting for *engine*. The only real question is whether the
go-to-market follows the architecture — or fights it.

**Recommendation:** keep rescue as the wedge (concrete, demoable, real pain), but
harden the primitive — crypto fingerprint, tamper-evident log, headless selection
spec — so use cases 2 and 3 are a configuration away, not a rewrite. Build the
wedge; protect the engine.
