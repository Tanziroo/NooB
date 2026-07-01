# scribe: Layered Architecture SPEC (v1.0-draft)
## Built via Hegelian Dialectic & Quaternary Council Adversarial Review

---

## Preamble: Reverse Socratic Method → North Star

**Q: Why does scribe exist?**
- A: To back up data from a broken/unknown disk without destroying it.

**Q: What makes a backup "successful"?**
- A: Files arrive at the destination, unmodified, and the operator knows which ones came.

**Q: What's the deepest fear in rescue work?**
- A: Overwriting or losing data you didn't mean to, because you didn't know what was on the disk.

**Q: So scribe's job is not "copy faster" but "copy *deliberately*"?**
- A: Yes. **Deliberate selection under uncertainty.**

**Q: What uncertainty does scribe remove?**
- A: "Where is the data?" (heatmap), "What am I taking?" (selection), "Will it work?" (sim).

**Q: What uncertainty does it NOT remove?**
- A: Corruption in flight, dedup conflicts, space exhaustion mid-copy, operator mistakes, rollback.

**North Star (Fundamental Anchor):**
```
scribe is a DELIBERATE SELECTION engine, not a backup engine.
Its job: make the unknown known, let humans decide, then execute cleanly.
Corollary: scribe must never silently fail, lose a file, or execute without permission.
```

---

## Layer 1: Core Loop (Scan → Select → Simulate → Execute)

### Thesis (Blue Team — Defender of Scope)
The core loop is sufficient and correct. It:
- Scans in linear time, reports progress
- Selects by folder/extension (simple, human-readable)
- Simulates via an external mod (loose coupling, extensible)
- Executes via `rsync --files-from` (battle-tested, atomic per file)

**Strength:** Minimal, works. No unnecessary features.

### Antithesis (Red Team — Adversary / Attacker)
The core loop is blind to real-world failure modes:
1. **No integrity check** — what if a file corrupts mid-scan? scribe selects a ghost.
2. **No conflict detection** — if the destination has version X and source has X, scribe blindly copies.
3. **No rollback** — if rsync fails halfway, destination is in limbo.
4. **Selection is too coarse** — "extension" doesn't distinguish `jpg` (photos) from `jpg` (thumbnails); human still guesses wrong.
5. **Simulation is just text** — a mod could lie; scribe has no way to verify it's honest.

**Weakness:** Assumes everything downstream works perfectly.

### Synthesis (Architect Council Decision)
**Layered confidence model:**
- **Layer 0 (scan):** Use a checksum of the manifest (MD5 of file list) to detect if source changed mid-scan.
- **Layer 1 (select):** Add a "conflicts check" mode: scan destination, compare metadata (size/mtime/hash), mark duplicates + changes.
- **Layer 2 (sim):** The mod's output is echoed back to the user for **manual approval** (yes/no popup) before rsync runs.
- **Layer 3 (execute):** Run rsync with `--checksum --itemize-changes` to log what actually moved; verify count against plan.
- **Layer 4 (post-flight):** Write a manifest of what was copied + checksums; user can verify with `scribe --verify <backup> <dest>`.

**Decision:** Add **layers of confidence**, not remove speed. Start with layer 0 (manifest checksum), escalate to 4 if user enables `--paranoid`.

---

## Layer 2: Reliability (Error Recovery & Rollback)

### Thesis (Blue Team)
Recovery is out of scope. scribe emits a plan; `rsync` executes it. If rsync fails, the user inspects and re-runs. scribe's job is done.

### Antithesis (Red Team)
That's how data dies. Real scenarios:
1. Disk fills mid-copy → rsync stops → 50% of files on destination, 50% missing. No rollback. User is confused.
2. Checksum mismatch detected during rsync → should we skip or retry? rsync has opinions; scribe has none.
3. Network drop (if over SSH) → connection lost. Did the last batch land? Did rsync clean up?
4. Operator cancels mid-run (Ctrl-C) → destination has partial state. Next time scribe runs, does it resume or start over?

**Weakness:** Assumes human supervision and recovery knowledge.

### Synthesis (Architect + Operator Council)
**Resumable backup model:**
- scribe writes a **recovery journal** (JSON) to destination: `scribe-journal.json` with entries like `{ file, size, checksum, status: [pending|copying|done|failed], timestamp }`.
- rsync runs with `--log-file=scribe-rsync.log`, which scribe parses to update the journal in real time.
- If rsync stops (any reason), `scribe --resume <path>` reads the journal and resumes from the last incomplete file.
- If destination is full → user frees space, runs `scribe --resume`, and it picks up where it left off.
- On success, journal is marked `complete` and moved to `scribe-journal.archive`.

**Decision:** Add **resumable backup** as Layer 2. Requires:
- Journal schema (write it in SPEC §8)
- `--resume` flag + logic
- rsync output parser
- Integrity check on resume (don't re-copy done files, but verify their checksums exist in destination)

---

## Layer 3: Scale & Performance

### Thesis (Blue Team)
Current scan is O(n) per file. For a 10M-file tree, this is minutes. That's acceptable for rescue work (not a daily backup).

### Antithesis (Red Team)
Minutes is too long when you're on a live USB with no swap:
1. **Memory bloat** — storing 10M FileRec structs is GB of RAM. Embedded systems (e.g., old laptops) may crash.
2. **UI unresponsiveness** — rendering 10M rows is impossible. We need **pagination/filtering**.
3. **Dedup logic scales poorly** — comparing every file's hash to destination's is O(n²) in the worst case.
4. **Progress is coarse** — showing "500 files" every 500 files is noise at 10M scale; should be dynamic.

**Weakness:** Doesn't scale to enterprise/forensic disk images (1TB+).

### Synthesis (Architect + Performance Council)
**Tiered aggregation:**
- **Tier 1 (Today):** Aggregate by folder + extension (current). Works to ~100k files.
- **Tier 2 (On-demand):** User selects a folder → scribe refines the view to subfolders. Lazy-load children.
- **Tier 3 (Sampling):** For massive trees (>5M files), offer a `--sample-rate` flag (scan 1-in-100 files, then estimate totals). Shows `(estimated)` label.
- **Tier 4 (Compression):** Store the scan result to a `.scribe-cache` file (gzip'd JSON of aggregated rows). Next run, skip the scan if source hasn't changed (check by walking just the top-level mtimes).
- **Tier 5 (Async):** Offer a `--scan-to-file` mode that writes JSON to disk without entering the TUI (useful for batch/automation).

**Decision:** Add **Tier 2 (folder drill-down)** as Layer 3a; Tier 3 (sampling) as Layer 3b, gated behind `--sample-rate`. Tiers 4-5 are post-v1.

---

## Layer 4: Operability (Logging, Automation, Auditability)

### Thesis (Blue Team)
scribe emits `scribe-plan.json` + `scribe-copy.sh`. That's the audit trail. User runs the shell script. Done.

### Antithesis (Red Team)
In a real rescue operation:
1. **No logging** — what mod ran? Did sim agree with reality? Was there any manual override?
2. **No automation** — if two disks arrive, can we script selecting the same folders on both? (No, selection is interactive.)
3. **No monitoring** — is the rsync still running? How long until done? Is it stuck?
4. **No audit trail** — weeks later, "which backup came from which disk?" You have no answer.
5. **No rollback of decisions** — if someone picks the wrong folder, there's no "undo" in the UI.

**Weakness:** Not enterprise-grade. Can't scale to multiple disks or repeated rescues.

### Synthesis (Architect + Operator Council)
**Operational layer additions:**
- **Session log:** scribe writes `scribe-session.log` with timestamp, version, user, root, selections, sim output, rsync result, md5 of what landed.
- **Selection profiles:** User can save a selection as `my-medical-backup.scribe-profile` (JSON: folders + exts). Later, load it with `--profile my-medical-backup` (pre-fills selections, still editable).
- **Batch mode:** `scribe --json-from-stdin --executor /path/to/mod --output /path/to/dest` reads selections from stdin JSON (for automation), runs simulation, and if `--auto-approve`, runs rsync silently.
- **Monitoring hook:** scribe writes to a Unix socket or named pipe (if provided via `--event-socket`) so an external monitor can watch progress in real time.

**Decision:** Add **session log** (Layer 4a) + **selection profiles** (Layer 4b) + **batch mode** (Layer 4c). Layer 4 gated behind flags; doesn't complicate the UI.

---

## Layer 5: Authority & Escalation

### Quaternary Council Final Review

**Architect (system design):**
"Layers 1–4 form a coherent system. Layer 0 is scan; Layers 1–2 guarantee correctness; Layers 3–4 add scale & ops. The north star — deliberate selection under uncertainty — is preserved at each layer. ✓ Approved."

**Operator (rescue technician):**
"Resumable backup is critical. Without it, a failed copy is a disaster. Session logs matter so we know what happened. Selection profiles would save time on repeated rescues. ✓ Approved with Layer 2 as mandatory, others optional."

**User (person running scribe):**
"I need the simulation to be honest. The UI needs to not hang on large disks. I need to know what I picked. ✓ Approved if progress spinner is always on and simulation output requires 'yes/no' approval."

**Adversary (red team, security/reliability):**
"Layers 0, 2, 3 are non-negotiable. Without manifest integrity (Layer 0), we're blind. Without rollback (Layer 2), we're taking risks. Without scaling (Layer 3), we fail on real disks. But Layers 4's automation must have strict gates: batch mode requires `--auto-approve`, which should not be the default, and session logs should be immutable (append-only). ✓ Approved with guards."

**Unanimous decision: APPROVED for v1.0 with layered rollout.**

---

## Rollout Plan (Layered)

| Layer | Version | Scope | Dependencies | Approval |
|-------|---------|-------|--------------|----------|
| 0 (Scan integrity) | v0.4 | Manifest MD5 checksum | None | ✓ |
| 1 (Sim approval) | v0.4 | Popup "run this? yes/no" | Layer 0 | ✓ |
| 2a (Resumable backup) | v0.5 | Journal + `--resume` | Layer 0, 1 | ✓ |
| 2b (Conflict detection) | v0.5 | `--check-dest` mode | Layer 0, 1 | ✓ |
| 3a (Folder drill-down) | v0.5 | Lazy subfolders | None | ✓ |
| 3b (Sampling) | v0.6 | `--sample-rate` flag | Layer 3a | Conditional |
| 4a (Session log) | v0.5 | Append-only log file | None | ✓ |
| 4b (Profiles) | v0.5 | JSON profile save/load | None | ✓ |
| 4c (Batch mode) | v0.6 | `--auto-approve` + stdin | Layers 1, 4a, 4b | Conditional |

**v0.4 ships:** Scan integrity + sim approval (core loop hardened).
**v0.5 ships:** Resumable backup + conflict detection + session logs + profiles (operator grade).
**v0.6 ships:** Sampling + batch automation (enterprise grade).

---

## Appendix: High-Signal Anchors (Per Layer)

**Layer 0:** Manifest integrity. If the source changes mid-scan, everything downstream fails silently. Anchor: checksum of file list before select.

**Layer 1:** User approval. Simulation is a lie if not validated. Anchor: "run this? yes/no" popup before rsync.

**Layer 2:** Resumability. A failed copy is worse than no copy. Anchor: journal + `--resume` logic.

**Layer 3:** Pagination. A 10M-file tree will OOM or hang. Anchor: folder drill-down (Tier 2).

**Layer 4:** Audit trail. Rescue work is forensic; history matters. Anchor: session log (append-only, immutable once written).

---

## End of Quaternary Council SPEC

All four perspectives have reached **consensus on the layered architecture.**
Next step: **implementation roadmap** (v0.4 → v0.6).

