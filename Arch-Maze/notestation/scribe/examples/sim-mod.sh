#!/usr/bin/env bash
# Example scribe executor ("mod"). Reads scribe-plan.json on STDIN and emits a
# SPEC §7.2 sim-result on STDOUT, which scribe renders visually (bars + badges +
# fit gauge). Replace the body with your real simulation logic.
#
#   scribe /mnt/sys --executor ./examples/sim-mod.sh   # then press 'x'

set -euo pipefail
plan="$(cat)"   # the full plan JSON arrives on stdin

# --- pull a couple of numbers out of the plan (jq if present, else grep) ---
if command -v jq >/dev/null 2>&1; then
  files=$(jq '.summary.files' <<<"$plan")
  bytes=$(jq '.summary.bytes' <<<"$plan")
else
  files=$(grep -o '"files": *[0-9]*' <<<"$plan" | head -1 | grep -o '[0-9]*')
  bytes=$(grep -o '"bytes": *[0-9]*' <<<"$plan" | head -1 | grep -o '[0-9]*')
fi

# --- pretend we inspected the destination drive ---
dest="/mnt/backup/KHETPRIME-rescue"
free=62277025792                      # 58 G free on sdb1 (stand-in)
skip=251658240                        # 240 M already present (dedup)
copy=$(( bytes - skip )); (( copy < 0 )) && copy=0
fits=true; [ "$copy" -gt "$free" ] && fits=false
eta=$(( copy / 36700160 + 1 ))        # ~35 MB/s -> seconds

cat <<JSON
{
  "sim_version": 1,
  "dest": "$dest",
  "dest_free_bytes": $free,
  "fits": $fits,
  "eta_seconds": $eta,
  "totals": { "copy_bytes": $copy, "skip_bytes": $skip, "conflicts": 0 },
  "rows": [
    { "key": "selected ($files files)", "bytes": $copy, "action": "COPY" },
    { "key": "(dedup) already on dest", "bytes": $skip, "action": "SKIP" }
  ]
}
JSON
