# scribe

A small rescue TUI: scan a disk's important areas, see a **heatmap** of where the
data weight is, **select by folder or extension**, and emit a machine-readable
**plan** that any external tool can act on — including your own simulation "mod",
which hooks on without modifying scribe.

## Build & run

```bash
# on SystemRescue / Arch:
pacman -Sy --noconfirm rust      # if cargo isn't present (needs network)
cargo build --release
./target/release/scribe /mnt/sys
# During the scan of large trees, a progress spinner shows on stderr
# Once the TUI starts, pick folders/extensions + press 'x' to simulate
```

## Keys

| key | action |
|-----|--------|
| `Tab` | switch Folders ↔ Extensions |
| `↑/↓` or `j/k` | move |
| `Space` | toggle select on the highlighted row |
| `a` / `n` | select all / none of the **visible** rows |
| `/` | filter the list (type to narrow; `Enter` keep, `Esc` clear) |
| `w` | **confirm**, then write the plan/manifest/journal + audit log |
| `x` | simulate: pipe the live plan to `--executor` and show its output |
| `Esc` | clear filter / close popup · `q` quit |

## Artifacts written (on `w`, after you confirm)

| file | layer | purpose |
|------|-------|---------|
| `scribe-plan.json` | L0 | the contract + `manifest_fingerprint` |
| `scribe-selection.txt` | — | `rsync --files-from` manifest |
| `scribe-copy.sh` | — | ready-to-run rsync to your backup drive |
| `scribe-journal.json` | L2 | per-file `pending` state for a future `--resume` |
| `scribe-session.log` | L4 | append-only audit line (timestamp + fingerprint) |

## Safety

scribe only **reads** the scanned tree. The `w` key never writes silently — it
opens a confirm popup showing the fingerprint first. Nothing is copied or deleted
until you run `scribe-copy.sh` (or your mod does). See `LAYERS.md` for the layered
architecture and `SPEC-v1-LAYERED.md` for the full reasoning.

---

## The hook contract (build your mod against this)

### 1. `--json` — full scan, no TUI

```bash
scribe /mnt/sys --json > scan.json
```

Emits `{ scribe_version, root, total_bytes, folders[], extensions[] }` where each
entry is `{ key, bytes, files }`. Use this to drive selection programmatically.

### 2. `scribe-plan.json` — the selection plan (written on `w`)

```json
{
  "scribe_version": "0.2.0",
  "root": "/mnt/sys",
  "selection": { "folders": ["medical_backups","home/khet"], "extensions": ["pdf"] },
  "summary": { "files": 1234, "bytes": 5678901 },
  "files": [ { "path": "medical_backups/scan.pdf", "bytes": 2000000 } ]
}
```

`path` is relative to `root`. This is the stable interface — paths + sizes — so a
downstream tool has everything it needs to copy, dedupe, verify, or simulate.

### 3. `--executor PROG` — the live simulation hook

```bash
scribe /mnt/sys --executor /path/to/your-mod
```

Press `x` in the TUI. scribe runs `PROG`, writes the current `scribe-plan.json`
to its **stdin**, and shows whatever `PROG` prints on **stdout** (and stderr) in a
popup. Contract for your mod:

- **stdin:** the plan JSON (same schema as above)
- **stdout:** a human-readable simulation report (shown verbatim)
- **exit:** ignored; output is what matters

That's the whole integration. Your mod sits *between* scribe's plan and the real
copy, simulates the outcome, and needs no teardown of scribe to do it.

Minimal mod skeleton:

```bash
#!/usr/bin/env bash
plan="$(cat)"                       # read plan JSON from stdin
files=$(grep -c '"path"' <<<"$plan")
echo "SIM: would act on $files files"
# ... your simulation here ...
```

## Tuning

- `--depth N` — folder grouping depth in the Folders view (default 2, e.g. `home/khet`).
- `--areas a,b,c` — override which top-level folders get scanned.

Try it risk-free on any folder first: `scribe /tmp/somedir --executor ./your-mod`.
