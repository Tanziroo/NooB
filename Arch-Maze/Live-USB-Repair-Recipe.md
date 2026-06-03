# Arch Maze — Live-USB Repair Recipe (WRITE mode)

**For: fixing the sabotaged system from the Live USB. This EDITS the disk.**

---

## ⚠️ Read this first (10 seconds)

- This is **write mode** — the opposite of the read-only map. You CAN change things now, so the one rule is: **get the partition name right** (Step 1) and **verify with a cold reboot** (Step 8).
- Best case: run the **read-only Mapping Guide first** so you know the REAL partition and the REAL respawn unit. The one spot below marked 🔶 needs that info — don't guess it.
- Repair order is fixed and not optional: **repos → coreutils (`mv`) → configs → Hyprland → kill respawn → cold reboot.** Each step depends on the one before.

---

## Step 1 — Find the system partition (no guessing)

```bash
lsblk -f
```
👀 Big ext4/btrfs = system. Note its name (e.g. `nvme0n1p2`). Small FAT32 = boot.

---

## Step 2 — Mount it and step inside (chroot)

Replace `ROOT` with the name from Step 1:

```bash
sudo mount /dev/ROOT /mnt
sudo mount --bind /dev  /mnt/dev
sudo mount --bind /proc /mnt/proc
sudo mount --bind /sys  /mnt/sys
sudo arch-chroot /mnt
```
👀 Your prompt changes — you're now "inside" the broken system, editing it live.

---

## Step 3 — Fix the repos FIRST (nothing installs until this works)

Get a network inside the chroot, then restore mirrors:

```bash
# Restore a working mirrorlist (overwrites the sabotaged one):
echo 'Server = https://geo.mirror.pkgbuild.com/$repo/os/$arch' > /etc/pacman.d/mirrorlist

# Sanity-check pacman.conf for sabotage (commented repos, fake URLs):
nano /etc/pacman.conf      # ensure [core] and [extra] sections are present + uncommented

# Refresh:
pacman -Syy
```
👀 `pacman -Syy` should download fresh databases. If it errors, the repo is still broken — fix that before continuing. **Do not skip ahead.**

---

## Step 4 — Restore the missing tools (`mv`, etc.)

```bash
pacman -S coreutils
which mv cp rm        # confirm they're back
```
👀 `which mv` should now print a path. If "not found" persists, coreutils didn't install — go back to Step 3.

---

## Step 5 — Fix the configs (edit live)

Edit whatever the map flagged. Common ones:

```bash
# Login / forced-logout / no-tab-complete:
nano /etc/profile
nano /home/NAME/.bashrc
nano /home/NAME/.zshrc

# Greeter:
nano /etc/greetd/config.toml
```
👀 Remove any line that force-`exit`s, logs out on error, or disables completion. Save with Ctrl-O, exit with Ctrl-X.

---

## Step 6 — Reinstall Hyprland

```bash
pacman -S hyprland
nano /home/NAME/.config/hypr/hyprland.conf   # fix/restore the broken config
```

---

## Step 7 — Kill the respawn trap 🔶

🔶 **This is the one you must NOT guess** — use the exact unit name the map found.

```bash
# Example only — replace with the real name from the map:
systemctl disable the-respawn-unit.path
systemctl disable the-respawn-unit.service
# Also check cron:
crontab -l            # and remove sabotage lines if present
```
👀 Without this, your fixes get undone on next boot. If you didn't map it, STOP and run the map — guessing here wastes the whole repair.

---

## Step 8 — Leave and COLD reboot to verify

```bash
exit                  # leaves the chroot
sudo umount -R /mnt
```
Then **fully power off, remove the USB, power back on.** Not a warm reboot — a cold one. Nothing is trusted until a clean boot from the real disk proves it: log in, open a terminal, run `mv --version`, launch Hyprland.

---

## If anything fights back

If a fix won't stick or a file reappears → the respawn trap (Step 7) is still live, or you mapped the wrong unit. That's a "stop and re-map," not a "force it harder." Forcing blind is the move that started this.
