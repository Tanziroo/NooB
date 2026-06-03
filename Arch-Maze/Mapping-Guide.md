# Arch Maze — Mapping Guide (READ-ONLY Recon)

**For: a beginner (Tanzi / Lenny). Goal: map every trap + clue. Change NOTHING yet.**

---

## ⚠️ The one rule that keeps you safe

We mount the broken system **READ-ONLY**. In read-only mode you **cannot break anything**, and the system's traps (the files that auto-respawn, the forced logouts) **cannot fire**, because we are *looking*, not *running* it.

- You are booted from the **Live USB**. That's the clean, trustworthy outside view.
- If any command asks you to type your password, that's `sudo` — normal.
- **If anything looks confusing or scary → STOP. Take a photo of the screen. Ask Ryan.** Stopping is always safe here.

You are a detective taking photos of a crime scene. You touch nothing.

---

## Step 1 — Find the disk (no guessing)

```bash
lsblk -f
```

👀 **Look for two things in the output:**
- The **big** partition (ext4 or btrfs) = the **system** (root). Write down its name, e.g. `nvme0n1p2`.
- The **small** one (~300MB–1GB, type **vfat/FAT32**) = the **boot** partition. Write down its name.

*Don't assume — use exactly what you see.*

---

## Step 2 — Mount it READ-ONLY (the safety rail)

Replace `ROOT` with the big partition's name from Step 1:

```bash
sudo mkdir -p /mnt/sys
sudo mount -o ro /dev/ROOT /mnt/sys
```

👀 **Check it worked:**
```bash
ls /mnt/sys
```
You should see folders like `home  etc  usr  boot`. If yes — you're in, read-only, totally safe.

---

## Step 3 — Find the two accounts

```bash
cat /mnt/sys/etc/passwd | grep -E "/home"
```

👀 **Look for:** the human usernames (each line shows a name and a `/home/NAME` folder). **Write down both names** — the clues are split between them.

```bash
ls -la /mnt/sys/home/
```
👀 Confirms both home folders exist.

---

## Step 4 — Hunt the clues (in BOTH homes)

For **each** username, run these (replace `NAME`):

**See everything, including hidden files:**
```bash
ls -la /mnt/sys/home/NAME
```

**Find clue-like documents:**
```bash
find /mnt/sys/home/NAME -type f \( -iname '*.txt' -o -iname '*.md' -o -iname '*readme*' -o -iname '*clue*' -o -iname '*note*' \) 2>/dev/null
```
*(The `2>/dev/null` only hides "permission denied" clutter so the list is readable — it does NOT hide your results.)*

**Read a clue you found:**
```bash
cat "/mnt/sys/home/NAME/whatever-file-you-found.txt"
```

👀 **Also check these less-obvious hiding spots:**
```bash
ls -la /mnt/sys/root /mnt/sys/opt /mnt/sys/srv /mnt/sys/var/tmp 2>/dev/null
```

📸 **Photograph every clue you read.** Don't try to act on them yet — just collect.

---

## Step 5 — Catalog the traps (read, don't change)

**Which tools were removed?**
```bash
ls -l /mnt/sys/usr/bin/mv /mnt/sys/usr/bin/cp /mnt/sys/usr/bin/rm /mnt/sys/usr/bin/ls
```
👀 If `mv` is missing → "No such file" confirms that trap.

**Why repos are dead:**
```bash
cat /mnt/sys/etc/pacman.conf
cat /mnt/sys/etc/pacman.d/mirrorlist
```
👀 Look for commented-out servers, weird URLs, or an empty mirrorlist.

**Forced logout + no tab-complete (login configs):**
```bash
cat /mnt/sys/etc/profile 2>/dev/null
ls -la /mnt/sys/etc/greetd 2>/dev/null
cat /mnt/sys/home/NAME/.bashrc /mnt/sys/home/NAME/.bash_profile /mnt/sys/home/NAME/.zshrc /mnt/sys/home/NAME/.zprofile 2>/dev/null
```
👀 Look for lines that `exit`/`logout` on errors, or completion turned off.

**Hyprland — what's missing/broken:**
```bash
ls -la /mnt/sys/home/NAME/.config/hypr 2>/dev/null
cat /mnt/sys/home/NAME/.config/hypr/hyprland.conf 2>/dev/null
```

**The hidden boot menu:**
```bash
cat /mnt/sys/boot/grub/grub.cfg 2>/dev/null
ls -la /mnt/sys/boot/loader/entries 2>/dev/null
```
👀 One of these will exist. Look for hidden/extra entries or a hidden-timeout setting.

**The auto-respawn mechanism (what re-creates files):**
```bash
ls -la /mnt/sys/etc/systemd/system/*.path /mnt/sys/etc/systemd/system/*.timer 2>/dev/null
cat /mnt/sys/etc/crontab 2>/dev/null
ls -la /mnt/sys/var/spool/cron 2>/dev/null
```
👀 A `.path` unit watching a file = the thing that respawns it. Note its name.

---

## Step 6 — Write the map

Make ONE list (in a notes app, a doc on a **second USB stick**, or photos — the live USB forgets everything on reboot):

- **Accounts:** name 1, name 2
- **Clues found:** where + what each said
- **Traps confirmed:** missing tools / repo sabotage / login configs / Hyprland gaps / hidden boot entries / the respawn unit
- **Still unknown / confusing:** anything to ask Ryan

---

## Step 7 — Close up safely

```bash
sudo umount /mnt/sys
```

You changed nothing. The maze is exactly as it was — but now it's **mapped.**

---

## What's next (after mapping)

With the map in hand, Ryan decides: **solve it as intended** (work the traps) or **full reset** (repair from the USB). Either way, the repair order is fixed:
**repos first → restore `mv`/coreutils → fix configs → reinstall Hyprland → cold-reboot to verify.**
Nothing is trusted until a clean reboot proves it.
