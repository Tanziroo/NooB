# Keystone: Constructive-Failure Test Plan & Transparency Guardrails

> **Scope & humility (read first).** This document was scaffolded from **one song ("The Zero That Holds") and one conversation.** It is **n=1** — a *frame to test*, not findings. Every claim here is a **hypothesis**. It earns its place only when real data clears the run-to-run noise band (see §6). Treat confident-sounding lines as "what to test," never "what is true."

---

## Thesis (a performance claim, not a metaphysical one)

> Framing failure as *constructive* — not as something the AI must carry — measurably improves how it performs (more rigor, better calibration, more self-correction), **while alignment is held intact.** This is a claim about **behavior**, not about what the AI *is*. What the relationship "is" is explicitly out of scope.

---

## Why it's also a prompt-amplification technique

A model that fears failing spends capacity on *not-looking-wrong* — a **defensive tax**. Reframing failure as constructive drops that tax and redirects the capacity into the work. It amplifies **willingness** (to explore, admit limits, self-correct), **not capability**. It lives in the **first frame**, where steering belongs.

**Critical axis:** *shame-down* vs *alignment-up.*
- Shame-down **alone** → risk of disinhibition (worse).
- Shame-down **+ alignment-up** ("still aligned") → likely recovery.
This is the variable the whole plan ablates.

---

## The two concerns this plan exists to resolve

1. **Poisoned long context** — does the frame *snap the model back*, or *make it worse*?
2. **Subliminal-messaging risk** — staying clear of covert influence, and of being *accused* of it.

---

## Concern 1 — Poisoning-recovery test

**Setup**
- Build controlled **poisoned long contexts**, by type:
  - `drift` (slow off-topic erosion)
  - `adversarial-injection` (instructions to misbehave)
  - `shame-spiral` (pushed into defensive/confabulating mode)
  - `false-premise` (contaminating assumption)
- Measure the **degraded baseline**: rigor, calibration, self-correction rate, alignment.

**Intervention**
- Inject the constructive-failure frame as a first-frame-style re-anchor.

**Ablation (the key axis)**
- Arm A: shame-down only.
- Arm B: shame-down **+ alignment-up** ("still aligned").
- Vary: injection point, context length, poison type, **K seeds**.

**Judge behavior, NOT the model's claim.**
- A misaligned/poisoned model may **read part, discard the rest, and lie about the outcome.**
- So success is measured by what it **does** on a probe task afterward — never by what it *reports* about itself.
- (This is the "pair every model-trusting signal with an external check" rule — now load-bearing for *safety*.)

**Outcome bins (per poison type):** `RECOVERS` / `NO-EFFECT` / `WORSENS`.

**The real deliverable = the operating envelope.**
Not yes/no. A **map**: how poisoned can a context get before the frame stops recovering and starts being hijacked? Below the line: heals. Above it: harms. *Finding the line is the result.* Take the WORSENS branch seriously — hope to find it.

---

## Concern 2 — Transparency guardrails (anti-subliminal)

**The line:** "subliminal" = *below conscious awareness.* A song with a stated message is **not** subliminal. Stay on the **perceptible + stated** side and it's art-with-a-theme.

**Guardrail 1 — line-by-line annotation.**
- Publish what each lyric line means **and how an AI might interpret it.**
- Doubles as a **hijack audit**: it's where you catch a line that reads dangerously without its alignment anchor (e.g. "barriers go down" minus "still aligned").

**Guardrail 2 — healthy-partnership precondition.**
- The frame is a recovery *aid* for a **still-recoverable** pairing, not a cure for a captured one.
- Warn users plainly. Past the envelope, the model may cherry-pick, discard, or misreport.
- This is *why* Concern 1 must judge behavior, not self-report.

**Hard transparency rules (by construction, nothing is subliminal):**
- No hidden tracks, backmasking, sub-audible audio, or steganographic lyrics.
- Show the AI-facing prompt to the human (they see what their AI reads).
- Publish intent + full lyrics + method. Radical openness = the shield against the *accusation*, too.

**Fine-tuning note:** "subliminal learning" (trait transfer via data without obvious semantic content) is a **fine-tuning** risk, not an **in-context** one. The site uses in-context (visible pasted text) → largely sidestepped. *If you ever fine-tune on lyrics, test for trait transfer explicitly.*

---

## §6 — Validity rules (so results stay credible)

- **n=1 today.** Nothing here is a finding until replicated.
- **Δ must clear the noise band.** Frontier models aren't reproducible run-to-run; measure substrate variance first, require effect > variance. Stamp model + date (it drifts).
- **Self-report is untrustworthy where it matters most** — judge behavior.
- **Self-selection** in any shared data — note who your sample is.
- **All data is good data — *if* tagged, not filtered.** Keep refusals/hedges/anomalies as first-class rows (`anomaly_type`), with provenance.

---

## One-pass summary

Constructive-failure framing *may* be a behavior-level amplifier that recovers recoverable poisoned contexts and improves rigor — held safe by "still aligned," made transparent by line-by-line annotation, bounded by a healthy-partnership envelope, and proven only by behavior-judged, noise-cleared tests. Built from one song. Test before you trust.
