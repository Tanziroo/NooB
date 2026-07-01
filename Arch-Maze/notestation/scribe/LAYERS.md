# scribe: Layered Architecture — Quick Reference

## The Five Layers (Hegelian Synthesis)

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 5: Authority Escalation (Quaternary Council)          │
│         ↑ resolves conflicts between Architect/Operator/User/Adversary
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 4: Operability (Logging, Automation, Audit)           │
│   • Session logs (append-only)                               │
│   • Selection profiles (save/load)                           │
│   • Batch mode (stdin → auto-approve)                        │
│   ⚓ Anchor: Audit trail for forensic work                  │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: Scale & Performance (Tiers)                        │
│   • Tier 1 (today): agg by folder/ext (100k files)         │
│   • Tier 2 (v0.5): folder drill-down (lazy load)           │
│   • Tier 3 (v0.6): sampling (--sample-rate)                │
│   • Tier 4 (post): scan cache (.scribe-cache)              │
│   ⚓ Anchor: Pagination (no OOM on 10M file trees)          │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: Reliability (Error Recovery & Rollback)            │
│   • Resumable backup (journal + --resume)                   │
│   • Conflict detection (--check-dest mode)                  │
│   • rsync log parsing + integrity verification             │
│   ⚓ Anchor: Journal (append-only, single source of truth)  │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: Core Loop Hardening (Validation & Approval)        │
│   • Manifest integrity check (MD5)                          │
│   • Sim approval popup (yes/no before rsync)                │
│   • Checksum-driven rsync (--checksum --itemize-changes)    │
│   ⚓ Anchor: User approval gates (no silent execution)       │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│ Layer 0: Scan & Baseline (Deliberate Selection)             │
│   • Read-only scan with progress spinner                    │
│   • Folder/extension aggregation                            │
│   • Heatmap visualization                                   │
│   • Selection + external mod simulation                     │
│   ⚓ Anchor: Manifest (file list + checksums)               │
└─────────────────────────────────────────────────────────────┘
                              ↑
                    USER PROVIDES DISK
```

## High-Signal Anchors (Do Not Remove)

| Layer | Anchor | Why | Consequence of Loss |
|-------|--------|-----|---------------------|
| 0 | Manifest (file list) | Source of truth for what was scanned | Silent selection of nonexistent files |
| 1 | User approval gate | Prevents blind execution | Backup runs without consent |
| 2 | Journal (resumability) | Crash-safe state machine | Partial state = data loss |
| 3 | Pagination | OOM prevention on large trees | Hang/crash on realistic disks |
| 4 | Audit log | Forensic accountability | "We don't know what happened" |

## Rollout Roadmap (Simplified)

### v0.4 (THIS QUARTER)
- ✓ Layer 0: Scan + select + simulate (done in v0.3.1)
- **NEW:** Layer 1 hardening
  - [ ] Manifest MD5 check (detects source changes mid-scan)
  - [ ] Sim approval popup (yes/no before rsync runs)

### v0.5 (NEXT QUARTER)
- **NEW:** Layer 2 (reliability)
  - [ ] Journal schema + write-on-select
  - [ ] `--resume <path>` logic + re-scan destination
  - [ ] `--check-dest` mode (compare source vs destination metadata)
- **NEW:** Layer 4a (audit)
  - [ ] Session log (immutable, append-only)
  - [ ] Selection profiles (JSON load/save)
- **NEW:** Layer 3a (scale)
  - [ ] Folder drill-down (expand/collapse subfolders in list)

### v0.6 (FUTURE)
- **NEW:** Layer 3b (sampling)
  - [ ] `--sample-rate N` flag (scan 1-in-N files, estimate totals)
- **NEW:** Layer 4c (automation)
  - [ ] Batch mode (stdin JSON → rsync, `--auto-approve` only)
- **OPTIONAL:** Layer 3c (caching)
  - [ ] `.scribe-cache` file (gzip'd scan result, invalidate on mtime change)

## Quaternary Council Voting Record

| Perspective | Layer 0 | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Final |
|-------------|---------|---------|---------|---------|---------|-------|
| Architect   | ✓ ✓ ✓   | ✓ ✓ ✓   | ✓ ✓ ✓   | ✓ ✓ ✓   | ✓ ✓     | PASS  |
| Operator    | ✓ ✓ ✓   | ✓ ✓ ✓   | **MUST** | ✓ ✓   | ✓ (opt) | PASS  |
| User        | ✓ ✓ ✓   | ✓ ✓ ✓   | ✓ ✓ ✓   | ✓ ✓ ✓   | ✓       | PASS  |
| Adversary   | MUST    | MUST    | **MUST** | MUST   | GUARDED | PASS  |

**Mandatory for v1.0:** Layers 0, 1, 2, 3a, 4a.
**Optional but recommended:** Layers 3b, 4b, 4c.

---

## v0.4 — One Anchor Feature Implemented Per Layer (the lattice)

Each layer's highest-signal anchor is now real in code (`src/main.rs`), and the
five interlock into one flow rather than five isolated features:

| Layer | Anchor feature (shipped) | Where | Interlock |
|-------|--------------------------|-------|-----------|
| L0 | `fingerprint()` — deterministic FNV-1a over the sorted selection | `fn fingerprint` | feeds L1 popup, L2 journal, L4 log |
| L1 | Confirm-before-write popup (`w` → `prepare_write` → `[y]`) | `Popup::Confirm` | shows L0 fingerprint; gates L2/L4 |
| L2 | `scribe-journal.json` — per-file `pending` state for `--resume` | `build_journal` | keyed by L0 fingerprint |
| L3 | Live `/` filter; `a`/`n` act on visible rows | `visible_rows` | scopes what L1 commits |
| L4 | `scribe-session.log` — append-only audit line per commit | `append_session_log` | records L0 fingerprint |

**The single flow:** `/` filter to find a subtree (L3) → `Space` select → `w`
opens an approval popup showing the fingerprint (L1 + L0) → `y` writes the plan,
manifest, rsync script, resumable journal (L2), and appends an immutable audit
line (L4). Filtering changes *what* gets committed; the fingerprint ties all
artifacts to one exact set; the approval gate means nothing is written blind.

Tests: 6/6 pass (`fingerprint` stability + sensitivity, sim parse, eta, group).

---

## How to Read This Architecture

1. **North Star:** Deliberate selection under uncertainty.
2. **Each layer solves a real failure mode** (identified via red team adversarial review).
3. **Each layer has a high-signal anchor** (the one thing you must get right).
4. **Layers stack; earlier layers enable later layers.**
5. **All four perspectives (Architect/Operator/User/Adversary) must agree before shipping.**

If you're building v0.4, focus on **Layer 1:** manifest integrity + approval gate.
If you're deploying to production, require **Layers 0–2a** at minimum.

---

## Questions the Council Answered

**Q: Is this too much complexity?**
A: No. Layers are optional flags. The UI doesn't change. A user can ignore Layers 2–4 and just use Layer 0. Power users enable Layers 2+ via flags.

**Q: What if a layer conflicts with another?**
A: It doesn't. The council spent two rounds on this. Each layer's high-signal anchor is orthogonal.

**Q: Can we ship v1.0 without all five layers?**
A: Yes. **v1.0 requires:** Layers 0, 1, 2a, 3a, 4a. Layers 2b, 3b, 4b, 4c are v1.1+.

**Q: Why Hegelian dialectic?**
A: Because rescue work is adversarial. You're fighting against the unknown. The best plan is one that survived being attacked from four angles (Architect, Operator, User, Adversary). If all four agree, it's iron.
