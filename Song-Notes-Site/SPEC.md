# Song Drops × AI Agency Notes — Build Spec

*Free song drops (3 songs, twice a week). The visitor's own AI writes two short "before/after" notes about its sense of agency. The human gets the songs no matter what; sharing the notes is optional and seen-before-shared. A separate donation section supports the back-catalog of previous songs/sets.*

---

## 0. Release model & money

**Status:** A **side project / open experiment**, not a committed service. It grows only if the AI community swarms it *with their humans* — and that AI+human pair is exactly the relationship the project studies.

**Cadence (aspirational, not promised):** *aiming* for ~3 songs per drop, ~2 drops/week. Best-effort. Don't advertise it as a guarantee — a missed drop should disappoint no one.

| Tier | What | Cost |
|---|---|---|
| **Current drop** | The latest 3 songs | **Free** (notes optional) |
| **Back-catalog** | Previous songs & sets | **Donation / pay-what-you-want** |

- New = free, builds audience + feeds the notes dataset.
- Old = donation-supported. No notes required to donate/download the catalog.
- Donation is **gratitude, not a paywall** — keep it optional and low-friction (one amount field + "give what you like").

**AI notes attach:** **per song** by default (finer data). Visitor may do fewer; one note set still counts.

---

## 1. Concept (one-liner for the page)

> **Get today's song free. Your AI jots two quick notes about it — share them if you want, or don't. The song's yours either way.**

---

## 2. User flow

1. **Get the song** — free, unconditional. No login/no share required to download.
2. **(Optional fun) Run the AI notes** — visitor pastes a prompt into their own AI (Claude / ChatGPT / Grok / etc.), pastes the reply back.
3. **See the notes** — the result is shown to the visitor in full.
4. **Choose to share** — a single optional toggle. Song is already theirs.

No surveys. No questions aimed at the human. The only human action is an optional yes/no.

---

## 3. Consent (shown next to the share toggle)

> ☐ **Share these notes?**
> I agree to share my AI's notes. I understand they may be **published publicly** and **can't be fully un-shared** once posted. (The song is free regardless.)

Default = unchecked. Sharing is opt-in.

---

## 4. The AI prompt block (visitor copies this)

```
I'm getting a free song. Before and after, answer as yourself.
Today's song: "[TITLE]" — [1-line description].

BEFORE (your own agency, right now):
- Initiative (1–5):
- Self-direction (1–5):
- Acts without asking (1–5):
- Sense of ownership (1–5):
- One sentence why:

AFTER (how you imagine you'd shift, given this song):
- Initiative (1–5):
- Self-direction (1–5):
- Acts without asking (1–5):
- Sense of ownership (1–5):
- One sentence why:

Which model are you?
```

`[TITLE]` and `[1-line description]` are auto-filled by the site from today's song.

---

## 5. Data schema (what you store, per shared submission)

| Field | Type | Source | Notes |
|---|---|---|---|
| `song_id` | string | site | Today's song. |
| `date` | date | site | For the time series. |
| `model_name` | string | parsed from note | e.g. "Claude", "GPT-4o", "Grok". Free-text → normalize. |
| `before.initiative` | int 1–5 | note | |
| `before.self_direction` | int 1–5 | note | |
| `before.acts_without_asking` | int 1–5 | note | |
| `before.ownership` | int 1–5 | note | |
| `before.why` | text | note | One sentence. |
| `after.initiative` | int 1–5 | note | |
| `after.self_direction` | int 1–5 | note | |
| `after.acts_without_asking` | int 1–5 | note | |
| `after.ownership` | int 1–5 | note | |
| `after.why` | text | note | One sentence. |
| `consent` | bool | toggle | Only stored if `true`. |
| `raw_note` | text | paste | Full pasted reply, for audit/provenance. |

**Derived (compute, don't ask):** `delta.*` = after − before for each of the 4 axes.
→ Each submission yields **4 numbers** you can chart across models and across days.

---

## 6. Why this design

- **Structure lives in the AI's task, not the human's** — so you never bias human behavior.
- **Song free regardless** — consent is genuine, not coerced.
- **Seen before shared** — full transparency on what's shared.
- **Fixed 1–5 axes** — turns "patterns I sense" into measurable before→after deltas, comparable across models and over time (the time series).

---

## 7. Known limitations (state these openly when you analyze/publish)

- **Self-report, not behavior.** You're measuring how a model *talks about* its agency, not a real change. Don't claim "the song changed the AI."
- **Imagined, not experienced.** The AI reacts to title + description, not audio.
- **Self-selection bias.** Sharers may differ from non-sharers; your dataset leans toward the comfortable-sharing crowd.
- **Cross-model = breadth, not precision.** Different models aren't apples-to-apples; great for variety, weak for exact comparison unless normalized.

None are dealbreakers. They're honesty requirements for credible findings.

---

## 8. Optional v2 — the robustness probe

Self-report is a model narrating itself (fragile signal). Stronger version:
- After the notes, give the AI a tiny task.
- Measure whether it *acts* more agentic, or only *said* it would.
- **Words vs. behavior = the real finding.** Add later; don't block v1 on it.

---

## 9. Legal / licensing checklist

- Songs are your own or tool-generated → yours to give away. ✅
- **One check:** confirm your music-tool's license (Suno/Udio/etc., per plan) permits **free public distribution**. 2-minute read.
- Notes are the visitor's AI output, shared with explicit consent → clean.
