# I-RD witness dump-read results (predictions: ird-witness-predictions.md)

All seven: oracle 3× identity-stable (harness), SdDump 3×
identity-normalized stable. Dumps banked: graft_targets/ird/
fz_*.dump.txt. Divergence shapes measured BEFORE dumps (engine 1×):

| witness | firings o/e | delta | finals |
|---------|-------------|-------|--------|
| fz_123_7219 | 11/10 | R1 (observer) −1 engine | identical |
| fz_42_6368 | 6/5 | R2 (or-twin deleter) −1 engine | identical |
| fz_777_1278 | 3/4 | R2 +1 ENGINE | identical |
| fz_777_2956 | 7/8 | R4 (deleter) +1 ENGINE | identical |
| fz_7_1591 | 8/11 | R4 +3 ENGINE | identical |
| fz_7_5988 | 6/7 | R1 (deleter) +1 ENGINE | identical |
| fz_7_8757 | 6/5 | R4 −1 engine | oracle keeps ONE T1 |

## Classification (against the pre-registered outcome mapping)

**1. fz_123_7219 = THE DYNAMIC LAW, out-of-sample PASS.** Textbook
joints in the dump: F5 stated+belief same value (mixed key), F6 the
unstage batch (`Delete@9 Insert@10`), F7 the immediate re-kill of
the unstage-born @10 (pending `-`, the 4048 F5 signature), F10 the
queued R1 act on @10 FIRING three firings post-delete in its
salience position. Every stated/ordinary-justified delete in the
same dump cancelled its acts both sides (incl. the T2(12)
kill/re-justify fresh-restart pair). Engine miss = exactly R1's
surviving firing.

**2. fz_42_6368 = THE DYNAMIC LAW via or-twin, out-of-sample
PASS + a law sharpening.** F3 unstage (`Delete@6 Insert@7`), F4
branch A kills @7 immediately, F5 **branch B of the SAME or-rule
fires on the dead @7** (delete-of-dead no-op). The surviving act is
the deleter's own twin ⇒ the survival is ACT-GENERIC (any queued
act on the unstage-born handle), not observer-specific.

**3. fz_777_1278 = OUT OF I-RD (reclassified per the mapping).** No
deletes, breaks, or unstages exist in the run. Delta = or-branch
activation count (oracle fires 2 of 3 or-branches, engine 3; the
second oracle firing is branch 2 triggered by the first's belief).
Justified-onto-justified dep-fold (bs n=2, one handle) matches the
pinned static rules — no TMS divergence. FILE under the or-branch
activation family (CE-or subrule counting), not this arc.

**4+5+6. fz_777_2956, fz_7_1591, fz_7_5988 = ONE NEW (THIRD)
MECHANISM: the SAME-BATCH SELF-BREAK landing.** Shared signature,
five instances (1591 ×3 across epochs): a rule justifies a belief
then UPDATES ITS OWN PREMISE in the same RHS — the TMS batch stages
`Insert@belief … Update@premise` (update LAST). The ORACLE lands
the update-sourced justification break WITHIN THE FLUSH: the belief
is registered in TMS bookkeeping at post-firing (5988: @5 =
`JUSTIFIED fhs[@5+]` with dep) and GONE by the next firing with no
Delete ever staged and NO ACT ON IT EVER FIRING. The ENGINE lands
the update-sourced break LAZILY — the belief materializes, its acts
fire (the deleter walls kill it), finals converge. Direction:
engine OVER-FIRES by one deleter firing per instance. NOT either
pinned law (the oracle CANCELS acts the engine fires — reverse
direction from the dynamic law). Landing-law row: mode=cloud,
cause=RHS-self-update, same-batch. ⚠ The engine's
`tms_eager_break(from_delete=false)` lazy path is the implicated
site BUT is annotated "certified lazy (unprobed on the staircase)"
— i.e. kept by default, not pinned by cells; still, splitter cells
must separate same-batch-self from (a) later-batch updates, (b)
foreign-premise updates, (c) the fz_42_2442 own-tuple-mid-firing
exclusion, before any fix — ⚖ method law.

**7. fz_7_8757 = THE STATIC FAMILY + A NEW REFINEMENT + x1 LIVE.**
Dump joints: F0/F1 build `STATED fhs[@3!@2+@4+]` (or-twin ⇒ TWO
stated siblings + the non-WM belief — rules 1+3 composing, n=2
deps). F3: R4 kills ONE stated sibling (@2) → the keys line goes
EMPTY and the batch stages `Delete@2 Insert@3` ⇒ **REFINEMENT: on a
stated-born mixed key with MULTIPLE stated siblings, the FIRST
stated delete kills the key WHOLE — orphaning the remaining stated
sibling AND unstaging the belief in one event** (the L6 key-death
and the 4048 unstage COMPOSED; my static-read assumption that the
unstage waits for the LAST stated is WRONG oracle-side). F4 kills
the unstage-born @3 (immediate). F5: R4 fires on the ORPHAN @4 and
the delete NO-OPS (x1 undeletability, live in a fuzz witness) — @4
is the oracle's finals survivor. Needs its own cell (two stated
faces + belief + deleter) before pinning the refinement. ⚠ ENGINE
ANOMALY flagged for port-time recon: engine fires R4 only ×2 with
NO finals survivor — inconsistent with a naive walk of
tms_route_delete_ex (expected ×3 with an unstage after the second
stated death); do NOT armchair it — instrument the engine when the
port slab opens.

## Scorecard vs predictions

Level-1 static classifications: 7219 ✓, 6368 ✓, 1278 ✓ (direction
unpredicted), 2956/1591/5988 ✓ (family predicted, mechanism refined
by the dumps: not "pending-cancel vs materialize" but BREAK-LANDING
within-flush vs lazy), 8757 ✗→reread (predicted "interleaving,
observable unclear"; actual = static family + refinement + x1 — the
finals delta was the tell the static read missed: the or-twin's
SECOND stated sibling changes the kill topology).

## The updated I-RD ledger

- DYNAMIC law: fz_7_4048 (+ b1/b2 cells) + **fz_123_7219 +
  fz_42_6368** — port target list of 3 witnesses + 2 cells.
- STATIC law: fz_7_9902 (+ l6 cell) + **fz_7_8757** (with the
  multi-stated refinement cell to build first).
- THIRD mechanism (same-batch self-break landing): **fz_777_2956 +
  fz_7_1591 + fz_7_5988** — splitter cells to build (same-batch vs
  later-batch, self vs foreign premise) before any pin.
- Out of I-RD: **fz_777_1278** → or-branch activation family.

Next per the plan: the refinement cell + the third-mechanism
splitters, then the model extension over all three mechanisms,
population, port validate-and-revert (Bryan gates).
