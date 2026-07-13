# I-RD A/B cell-round predictions (logged BEFORE the runs)

The D-207 population findings, taken to cells per the pre-registered
protocol. Model re-pins come FROM these cells only; after re-pin:
validator green (22 + the new cells) → population re-run on the same
five seeds → 0 REAL required before the port.

## ROUND A — the mixed-key kill's scope (Finding A)

Witness alignment first: the three A witnesses all have the belief
arriving on a 2-STATED key AND no stated arriving after — the two
candidate axes COINCIDE in the population, which is exactly why the
cells must split them. The completed 2×2 over
{1,2}-stateds-at-belief-insert × {some,none}-stated-after:
(1,some) = r1 (pinned R-FIRST/orphan); (1,none) = b1 (degenerate —
R-FIRST and R-LAST coincide with no sibling); (2,none) = **d1**
(new; witness-faithful: s1 initial + s2 via RHS ST — two of three
witnesses had the mixed route); (2,some) = **d2** (new; the
discriminator: s3 appended AFTER the belief).

- P-POSITION: pending forms regardless of stated count; the
  key-dies-whole event arms iff a stated arrived AFTER the belief.
- P-SLOT(-WM): pending forms only on an exactly-one-stated key;
  on ≥2 stateds the belief is WM-VISIBLE at insert and the r1 event
  never arms. (P-SLOT-DEP — no handle at all — is already refuted:
  the witnesses fired the deleter ×3, so a third handle existed.)

Cells: d1 = [T0, V()] + ST(s2)@20 + JL(belief)@18 + RD@5 + ROBS@0;
d2 = d1 + ST2(s3)@16 (after the belief). SdDump 3× on BOTH (the
belief's form — `b!` pending vs `b+` WM — and the batch shapes are
dump-only observables); also dump r1 for the pending-mark baseline.

| cell | P-POSITION | P-SLOT-WM |
|------|-----------|-----------|
| d1 | RD×3, ROBS(v)=1 (the unstage-born survivor at the LAST stated's kill), finals 0; dump: b! + unstage batch Delete@s2+Insert@hb | RD×3, ROBS(v)=0, finals 0; dump: b+ from insert, Insert@b in RJ's batch, three plain kills |
| d2 | RD×4, ROBS(v)=3 (TWO orphans + the hb survivor), finals 2; dump: key-death at the FIRST kill | RD×4, ROBS(v)=0, finals 0; dump: b+ from insert, four plain kills |

Outcome mapping: d1+d2 both in one column ⇒ that hypothesis is the
law; d1=SLOT-WM but d2=POSITION-like ⇒ hybrid (both variables
load-bearing — pending-form by count AND kill-event by position);
any outcome matching NEITHER column (e.g. d1 RD×2) ⇒ the belief
form is something else (dep-only on this route?) — STOP, read the
dumps, do not force a column. Sub-risk: if d1 contradicts the
witnesses' shape (they used RHS routes; d1 mirrors 6003x23), the
initial-vs-RHS route matters — rebuild the failing cell RHS-only.

## ROUND B — the lazy slot + update-requeue (Finding B)

The witness evidence conflates two mechanisms; three cells separate
them. ⚠ these cells DELIBERATELY use same-salience ties (the thing
under test); the only cross-rule tie is the one being probed.

- **s1 (slot straddle, isolated)**: [T0]; RJ@10 selfjoin+modify
  JL(v); RD(v)@10 decl-after. RJ's single (t,t) tuple fires; the
  belief's insert creates RD's act (seq BEFORE the pseudo, which is
  scheduled at the later modify op). The bit: does RD ever fire?
  P-BREAK-FIRST (Finding B's read: the lazy break lands at the
  justifier's item, BEFORE any later same-salience pop): the break
  kills v, RD's act cancels ⇒ firings [RJ] only. P-FIFO (the old
  model): [RJ, RD].
- **s2 (the witness twin, 6003x41 minimal)**: [T0, T0]; RJ@10
  selfjoin+modify JL(v); ROBS@15. Pre-registered readings:
  slot∧requeue ⇒ [RJ, ROBS, RJ, ROBS] (the second firing is the
  OTHER twin (t2,t2), lazy again, ROBS refires on the fresh key);
  slot-only ⇒ [RJ, ROBS, RJ] (the mixed (t1,t2) pops next, its
  single-binding break lands in-flush, no ROBS refire);
  requeue-only or neither ⇒ [RJ, ROBS, RJ] with different
  internals. s2 = the conjunction detector; s1 pins the slot alone;
  s3 pins requeue alone. SUB-RISK: the reading assumes the first
  pop is the (t1,t1) twin (both witnesses' first firings were
  twins); if the s2 oracle's first firing is mixed, re-read from
  the dump before concluding. SdDump 3× on s2.
- **s3 (update-requeue, direct)**: [T0(t1), T0(t2), arm]; RB@20 =
  the RU kind (updates t1→false, consumes the arm); RO@10 = a NEW
  T0OBS kind (bare `T0($x : f0)` observer — vocabulary addition,
  observer-only, no law content). RO's two acts queue at init
  (t1 then t2, FIFO). RB fires first, updates t1 (still
  alpha-valid for RO's bare pattern). The bit = RO's firing ORDER:
  P-REQUEUE (Drools cancels+re-queues surviving tuples of an
  updated fact): RO(t2/true) THEN RO(t1/false). P-KEEP: RO(t1/
  false) THEN RO(t2/true).

## Post-round protocol (pre-registered)

Winners get encoded in model_ird (the slot as pseudo-beats-same-
salience; requeue as cancel+tail-requeue of surviving acts; the A
winner in the key model); d1/d2/s1/s2/s3 join CELLS with banked
truths; the mutation matrix gains one mutation per new commitment
(each must fail exactly its pinning cells); then the population
re-runs on 7001/7002/6001/6003/9001 — the gate is 0 REAL. Corner
counts will persist (corners are a separate, later round). Any cell
outcome outside its prediction table ⇒ stop before the model edit,
dump, and if needed build the next discriminator — never force a
column.
