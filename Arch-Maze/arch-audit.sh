#!/usr/bin/env bash
# arch-audit.sh — READ-ONLY diagnostic for a sabotaged/broken Arch install.
# Run from a live environment (SystemRescue). It MOUNTS THE TARGET READ-ONLY,
# reads configs, and writes a report. It NEVER modifies the target disk.
#
# Answers two questions:
#   1. Why can a user reach root? (sudoers, empty passwords, SUID, groups, polkit)
#   2. What config kicks the terminal back to login? (shell rc/profile traps, greeter)
#
# Usage:  sudo bash arch-audit.sh            (defaults to /dev/nvme0n1p2)
#         sudo bash arch-audit.sh /dev/sdXN  (override the partition)

set -u                      # error on unset vars
# NOTE: deliberately NOT 'set -e' — we want the audit to keep going and
# REPORT failures rather than silently abort. Hiding errors is the whole bug we're avoiding.

PART="${1:-/dev/nvme0n1p2}"
MNT="/mnt/sys"
REPORT="/root/arch-audit-REPORT.txt"

# Everything prints to screen AND to the report file.
exec > >(tee "$REPORT") 2>&1

hr(){ echo "============================================================"; }
sec(){ hr; echo "## $*"; hr; }

echo "Arch read-only audit — target: $PART"
echo "Report will be saved to: $REPORT"
echo "Started: $(date 2>/dev/null || echo 'date unavailable')"
echo

# ---------------------------------------------------------------------------
sec "0. Tool bootstrap (only installs if MISSING and a network is up)"
# Core tools we rely on. Most exist on SystemRescue already.
need=(find stat awk grep sort getent)
missing=()
for t in "${need[@]}"; do command -v "$t" >/dev/null 2>&1 || missing+=("$t"); done
if [ "${#missing[@]}" -eq 0 ]; then
  echo "All required tools present: ${need[*]}"
else
  echo "Missing: ${missing[*]}"
  if ping -c1 -W2 archlinux.org >/dev/null 2>&1; then
    echo "Network is up — attempting: pacman -Sy --noconfirm ${missing[*]}"
    pacman -Sy --noconfirm "${missing[@]}" || echo "WARN: install failed; continuing with built-ins."
  else
    echo "WARN: no network — cannot download tools. Continuing with whatever is present."
  fi
fi
echo

# ---------------------------------------------------------------------------
sec "1. Mount target READ-ONLY"
mkdir -p "$MNT"
if mountpoint -q "$MNT"; then
  echo "Something already mounted at $MNT — unmounting first."
  umount -R "$MNT" 2>/dev/null || true
fi
if mount -o ro "$PART" "$MNT"; then
  echo "OK: $PART mounted READ-ONLY at $MNT"
else
  echo "FATAL: could not mount $PART. Check the name with 'lsblk -f'. Aborting."
  exit 1
fi
echo
echo "Filesystem tree top level:"; ls -la "$MNT"
echo
# If this is a btrfs root with subvolumes, the real root may be a subvol.
echo "(If folders like /etc /home are missing above, this may be a btrfs subvolume layout —"
echo " tell me and I'll give the subvol mount command.)"
echo

# Discover the human users (UID >= 1000) for later loops.
echo "Human accounts (UID >= 1000):"
USERS=$(awk -F: '$3>=1000 && $3<65534 {print $1":"$6}' "$MNT/etc/passwd" 2>/dev/null)
echo "$USERS"
echo

# ===========================================================================
sec "2. WHY CAN A USER REACH ROOT?"
# ===========================================================================

echo "--- 2a. Extra UID 0 accounts (should be ONLY 'root') ---"
awk -F: '$3==0 {print "  UID0:",$1}' "$MNT/etc/passwd" 2>/dev/null
echo

echo "--- 2b. Password status in /etc/shadow (empty hash = passwordless login!) ---"
# Field 2 empty = no password. '!' or '*' = locked. A real hash starts with \$.
awk -F: '{
  s="?";
  if($2=="") s="*** EMPTY = NO PASSWORD ***";
  else if($2=="!"||$2=="*"||$2 ~ /^!/) s="locked";
  else if($2 ~ /^\$/) s="has password";
  print "  "$1": "s
}' "$MNT/etc/shadow" 2>/dev/null | grep -Ei 'root|EMPTY|NO PASSWORD'
echo "  (full list is in the report; root + any EMPTY shown above)"
echo

echo "--- 2c. sudoers: NOPASSWD / blanket ALL grants ---"
for f in "$MNT/etc/sudoers" "$MNT"/etc/sudoers.d/*; do
  [ -f "$f" ] || continue
  echo "  File: ${f#$MNT}"
  grep -nEv '^\s*#|^\s*$' "$f" 2>/dev/null | grep -E 'NOPASSWD|ALL\s*=|^\s*Defaults' | sed 's/^/    /'
done
echo

echo "--- 2d. Who is in wheel / sudo groups ---"
grep -E '^(wheel|sudo):' "$MNT/etc/group" 2>/dev/null | sed 's/^/  /'
echo

echo "--- 2e. SUID-root binaries (a planted SUID shell = instant root) ---"
echo "  Scanning $MNT for setuid files (this can take a minute)..."
find "$MNT" -xdev -type f -perm -4000 2>/dev/null \
  | sed "s|$MNT||" \
  | grep -Ev '^/usr/(bin|lib)/(sudo|su|passwd|mount|umount|newgrp|chsh|chfn|gpasswd|pkexec|fusermount3?|ping)$' \
  | sed 's/^/    SUSPICIOUS SUID: /'
echo "  (Known-normal SUID binaries filtered out above; anything listed is worth a look.)"
echo

echo "--- 2f. World-writable files in /etc and writable dirs early in PATH ---"
find "$MNT/etc" -xdev -type f -perm -0002 2>/dev/null | sed "s|$MNT||;s/^/    WORLD-WRITABLE: /"
echo

echo "--- 2g. polkit rules granting admin without auth ---"
grep -rnEl 'ResultActive\s*=\s*"?yes|return polkit.Result.YES' \
  "$MNT/etc/polkit-1" "$MNT/usr/share/polkit-1/rules.d" 2>/dev/null \
  | sed "s|$MNT||;s/^/    POLKIT RULE: /"
echo

# ===========================================================================
sec "3. WHAT KICKS THE TERMINAL BACK TO LOGIN / CRASHES ON ERROR?"
# ===========================================================================
# We scan every shell startup file (system + both users) for the usual sabotage:
#   exit / logout / trap on EXIT or ERR / kill of the shell / loginctl terminate /
#   'set -e' in an interactive file / commands aliased to something fatal.

SHELLFILES=( "$MNT/etc/profile" "$MNT/etc/bash.bashrc" "$MNT/etc/bashrc" )
for d in "$MNT"/etc/profile.d/*.sh; do [ -f "$d" ] && SHELLFILES+=("$d"); done
for z in "$MNT"/etc/zsh/*; do [ -f "$z" ] && SHELLFILES+=("$z"); done
# add per-user dotfiles
while IFS=: read -r _ home; do
  for rc in .bashrc .bash_profile .profile .bash_logout .zshrc .zprofile .zlogin; do
    [ -f "$MNT$home/$rc" ] && SHELLFILES+=("$MNT$home/$rc")
  done
done <<< "$(awk -F: '$3>=1000 && $3<65534 {print $1":"$6}' "$MNT/etc/passwd" 2>/dev/null)"

echo "--- 3a. Suspicious lines in shell startup files ---"
PAT='(^|[^[:alnum:]_])(exit|logout|trap[[:space:]]|loginctl[[:space:]]+terminate|kill[[:space:]]+-9|pkill|set[[:space:]]+-e|vlock|kill[[:space:]]+\$\$|exec[[:space:]]+false)([^[:alnum:]_]|$)'
for f in "${SHELLFILES[@]}"; do
  hits=$(grep -nE "$PAT" "$f" 2>/dev/null)
  if [ -n "$hits" ]; then
    echo "  >>> ${f#$MNT}"
    echo "$hits" | sed 's/^/      /'
  fi
done
echo

echo "--- 3b. Aliases / functions that override common commands ---"
for f in "${SHELLFILES[@]}"; do
  hits=$(grep -nE '^\s*(alias\s+(ls|cd|mv|cp|rm|cat|sudo|clear)=|(ls|cd|cat)\s*\(\)\s*\{)' "$f" 2>/dev/null)
  if [ -n "$hits" ]; then
    echo "  >>> ${f#$MNT}"
    echo "$hits" | sed 's/^/      /'
  fi
done
echo

echo "--- 3c. PROMPT_COMMAND / TMOUT (auto-logout timers) ---"
grep -rnE 'PROMPT_COMMAND|^\s*TMOUT=|export\s+TMOUT' "${SHELLFILES[@]}" 2>/dev/null \
  | sed "s|$MNT||;s/^/    /"
echo

echo "--- 3d. Login manager / greeter config (greetd, autologin, getty overrides) ---"
for g in "$MNT/etc/greetd/config.toml" "$MNT"/etc/systemd/system/getty@*.service.d/*.conf \
         "$MNT"/etc/systemd/system/getty@*.service "$MNT/etc/sddm.conf"; do
  [ -f "$g" ] || continue
  echo "  >>> ${g#$MNT}"
  grep -nEv '^\s*#|^\s*$' "$g" 2>/dev/null | sed 's/^/      /'
done
echo

echo "--- 3e. Auto-respawn mechanisms (what UNDOES your fixes on next boot) ---"
ls -la "$MNT"/etc/systemd/system/*.path "$MNT"/etc/systemd/system/*.timer 2>/dev/null | sed "s|$MNT||;s/^/    /"
echo "  cron:"; cat "$MNT/etc/crontab" 2>/dev/null | grep -Ev '^\s*#|^\s*$' | sed 's/^/      /'
ls -la "$MNT"/var/spool/cron/ 2>/dev/null | sed 's/^/      /'
echo

# ===========================================================================
sec "4. Done — target left mounted READ-ONLY at $MNT"
echo "Nothing on $PART was modified."
echo "Full report saved to: $REPORT  (copy it to a USB stick or photograph it)"
echo
echo "To unmount when finished:   sudo umount -R $MNT"
