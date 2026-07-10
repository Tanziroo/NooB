# FORK-01 — Cryptographic Fingerprint & Tamper-Evident Manifest

Status: **proposed** (design locked, not yet built)
Chosen because it is the single change that pays off in all three markets
(rescue, forensic, ML) — see `PITCH.md` §Synthesis.
Supersedes the L0 anchor in `LAYERS.md` (FNV-1a → SHA-256 content hash).

---

## 1. The problem this fork closes

Today's `fingerprint()` is **FNV-1a over `(path, size)`**. That is:

- **Fast and deterministic** — good for "did my selection change?"
- **NOT collision-resistant** — trivial to forge a different set with the same hash.
- **NOT content-aware** — two files with the same path+size but different *bytes*
  produce the same fingerprint. A silently corrupted or swapped file is invisible.
- **NOT signed** — nothing proves *who* produced the manifest or *when*, and
  nothing detects after-the-fact edits.

Reverse-Socratic descent to the anchor:
- Why upgrade? → Because a fingerprint that can be forged or that ignores content
  is worthless the moment anyone adversarial (or any silent corruption) is in play.
- Why does that matter beyond rescue? → Forensic custody and ML reproducibility
  both need "this is provably the exact set, unchanged." Rescue merely *wants* it;
  they *require* it.
- **Terminal anchor:** *The fingerprint must be a claim you can defend, not a
  convenience checksum.*

## 2. What changes (three concentric rings)

### Ring A — Content hashing (correctness)
Replace the identity of a file from `(path, size)` to `(path, size, sha256(bytes))`.

- Per-file: `sha256` of the file contents, computed during scan (streamed, 64 KiB
  buffer, no full-file load).
- Manifest fingerprint: `sha256` over the sorted list of per-file
  `"<sha256>  <size>  <path>\n"` lines (a Merkle-style root — stable, order-
  independent because we sort first).
- Result: any changed byte in any selected file changes the root. Corruption,
  substitution, truncation — all visible.

**Cost:** hashing reads every selected file's bytes. That's I/O-bound and, on a
dying disk, non-trivial. Therefore content hashing is **opt-in per tier** (§4),
never forced on the fast rescue path.

### Ring B — Signed manifest (authenticity)
Emit a detached signature over the manifest fingerprint.

- v1: **Ed25519** (small keys, fast, no config). Key from `--sign-key <path>` or
  a generated session key written to `scribe-key.pub` alongside the artifacts.
- Output: `scribe-plan.json` gains `"manifest_sha256"` and `"signature"` (base64),
  plus `scribe-plan.sig` (detached). A verifier only needs the public key + the
  manifest to confirm "this set, from this key, unmodified."
- v2 (regulated markets): PKCS#11 / smart-card / TPM-backed keys, and optional
  RFC-3161 timestamp-authority countersignature for "existed at time T."

### Ring C — Hash-chained audit log (tamper-evidence)
Upgrade `scribe-session.log` from append-only text to an append-only **hash
chain**.

- Each entry carries `prev_hash = sha256(previous_entry_canonical)`.
- Editing or deleting any past line breaks every subsequent `prev_hash` — the
  tamper is detectable without external state.
- `scribe --verify-log scribe-session.log` walks the chain and reports the first
  break (or `intact`).
- v2: periodically anchor the head hash externally (notary / append-only object
  store / transparency log) so even wholesale log replacement is caught.

## 3. Threat model (who each ring defends against)

| Ring | Defends against | Does NOT defend against |
|------|-----------------|-------------------------|
| A content hash | silent corruption, file swap, truncation | a liar who regenerates the manifest |
| B signature | forged/edited manifest, wrong author | a signer who signs bad data knowingly |
| C hash chain | edited/deleted past log entries | wholesale log replacement (→ v2 anchor) |

Layered on purpose: A makes the *data* honest, B makes the *manifest* honest, C
makes the *history* honest. Rescue needs A; forensic needs A+B+C; ML needs A (as
a dataset version) and often B (provenance).

## 4. Tiering — never tax the fast path

The rescue wedge must stay instant. So hashing is a dial, not a default:

| Mode | Flag | Per-file hash | Manifest | Signature | Log |
|------|------|---------------|----------|-----------|-----|
| Fast (today) | *(default)* | none (FNV path+size) | FNV root | — | plain append |
| Verified | `--hash` | sha256 (streamed) | sha256 Merkle root | — | plain append |
| Custody | `--hash --sign KEY` | sha256 | sha256 root | Ed25519 | hash-chained |
| Regulated | `--custody-profile` | sha256 | sha256 root | HW key + RFC-3161 | chained + anchored |

`--hash` shows a second progress spinner ("hashing N/M, X MB/s") so the operator
sees the cost they opted into. On a flaky disk they can still choose Fast and
accept path+size identity.

## 5. Schema deltas

`scribe-plan.json` (Verified+):
```json
{
  "scribe_version": "0.5.0",
  "hash_algo": "sha256",
  "manifest_sha256": "9f2c…",
  "signature": { "algo": "ed25519", "pubkey": "base64…", "sig": "base64…" },
  "files": [
    { "path": "medical_backups/scan.pdf", "bytes": 2000000, "sha256": "ab12…" }
  ]
}
```

`scribe-session.log` line (Custody):
```
seq=42 ts=1751320000 version=0.5.0 root=/mnt/sys files=1615 bytes=15.9e9 \
  manifest=9f2c… sig=…  prev=7d1a… self=4e88…
```
`self = sha256(canonical(this line without self))`; `prev = self of seq 41`.

## 6. New verbs

- `scribe --hash …` — content-hash the selection.
- `scribe --sign <key> …` — sign the manifest (implies `--hash`).
- `scribe --verify <plan.json> <root>` — re-hash the tree, confirm every file
  matches the manifest and the signature is valid. Exit 0 = intact.
- `scribe --verify-log <log>` — walk the hash chain, report first break.
- `scribe --gen-key <path>` — create an Ed25519 keypair.

## 7. Dependencies & cost

- `sha2` (RustCrypto) — pure-Rust SHA-256, no system OpenSSL. ~1 crate.
- `ed25519-dalek` — pure-Rust signatures. ~1 crate + `rand`.
- No C toolchain, so it still builds on a SystemRescue live USB with just cargo.
- Binary grows modestly; scan time in `--hash` mode is bounded by disk read speed.

## 8. Migration / compatibility

- Fast mode is byte-for-byte the current behavior; existing mods keep working.
- `manifest_fingerprint` (FNV) stays for Fast mode; Verified+ adds
  `manifest_sha256` alongside. A mod can read whichever exists.
- Version bumps to 0.5.0 because the plan schema gains fields (additive, not
  breaking).

## 9. Build order (when we implement)

1. Ring A: streamed `sha256` per file behind `--hash`; Merkle root; schema fields.
   + tests: root stability, order-independence, single-byte sensitivity.
2. `--verify` (re-hash + compare). This is the payoff verb; ship it with Ring A.
3. Ring B: Ed25519 sign/verify + `--gen-key`. + test: sign→verify roundtrip,
   tamper→fail.
4. Ring C: hash-chained log + `--verify-log`. + test: intact chain passes, edited
   line fails at the right seq.
5. v2 items (HW keys, RFC-3161, external anchoring) — separate fork, market-gated.

## 10. Council check (quaternary)

- **Architect:** additive schema, pure-Rust deps, tiered so the core loop is
  untouched. ✓
- **Operator:** `--hash` shows its own cost; Fast mode still exists for triage on
  dying media. ✓
- **User:** `--verify` answers "did it really copy intact?" — the question they
  actually lose sleep over. ✓
- **Adversary:** A+B+C is a real, staged threat model, not hashing theater. v1
  honest about what it does NOT stop (knowing signer, wholesale log swap) and
  defers those to v2 explicitly. ✓ **with the caveat that Custody must not be
  marketed as court-admissible until Ring C + external anchoring (v2) land.**

**Verdict:** approved as the next build, gated behind flags, wedge path untouched.

---

## 11. IMPLEMENTED — Ring A (v0.5.0)

scribe is now a real signer. Shipped:
- `--hash` (verified tier): streams SHA-256 of every selected file on write;
  embeds per-file `sha256` + a Merkle-style `manifest_sha256` in the plan and
  journal; also emits `scribe-manifest.sha256` (coreutils `sha256sum -c` format).
- `--verify <plan.json> [root]`: re-hashes the tree, confirms every file matches
  the signed manifest, recomputes the root. Exit 0 = intact, 1 = changed/missing,
  2 = plan is fast-tier (no seal). Verified end-to-end: intact→VERIFIED,
  one tampered byte→CHANGED/FAILED.
- Fast tier (FNV, default) unchanged; `attestation_tier` field records which tier
  sealed a plan. Ring B (ed25519 signing) and Ring C (hash-chained log) are next.

## 12. The CONTRACT LAYER (attestation ladder) — operator insight

The tier is not just a rigor dial; it is a **contract ladder**. Higher-stakes work
requires deeper attestation, automatically, by the contract's declared level:

| Contract level | Attestation required | Mechanism (who signs) |
|----------------|----------------------|------------------------|
| L0 casual | fast fingerprint (FNV) | scribe, `--` default |
| L1 verified | content SHA-256 seal | scribe `--hash` (code) |
| L2 signed | + author signature | scribe `--sign` (ed25519, Ring B) |
| L3 custody | + hash-chained audit + external timestamp | scribe + notary (Ring C / v2) |
| L4 sealed | + obfuscation / restricted access / multi-party attestation | Notestation policy |

Rule: a contract of level N will not `commit` unless the attestation for level N was
actually produced (Label ≤ Mechanism, enforced in code). The Notestation Layer reads
a contract's declared level and refuses to seal until the matching signer has run —
so "more serious" is not a vibe, it is a gate. This is the same tiered-rigor dial as
`dial`, applied to obligations instead of prompts.

---

## 13. IMPLEMENTED — Ring B + Ring C (v0.6.0)

- **Ring B (signing):** `--gen-key <path>` creates an ed25519 keypair (hex);
  `--sign <key>` (implies `--hash`) signs the `manifest_sha256` and embeds
  `signature{algo,pubkey,sig}` in the plan + writes detached `scribe-plan.sig`.
  `--verify` now checks the signature too (valid/invalid), noting that key trust
  requires out-of-band pinning. Tier recorded as `signed-ed25519`.
- **Ring C (hash-chained audit):** `scribe-session.log` is now a hash chain —
  each line carries `prev=<self of previous>` and `self=sha256(line-without-self)`.
  `--verify-log [path]` walks the chain and reports the first break (exit 0 intact,
  1 broken). Verified: intact→0, single-field edit→BROKEN at the right seq. The
  format is plain enough to re-check with `sha256sum`.
- Tests: 11 total (added ed25519 sign/verify roundtrip incl. wrong-key/wrong-msg,
  hex roundtrip, hash-chain tamper detection). clippy `-D warnings` clean.

Remaining (deferred, honest): the **two-anchor** signature (bind host anchor +
signer anchor) and external timestamp/notary anchoring — composed at the seal-gate
with the operator's device, per NOTESTATION-CONCEPT.
