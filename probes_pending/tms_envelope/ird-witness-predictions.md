# I-RD witness dump-read predictions (logged BEFORE any runs)

Scope: the remaining open I-RD witnesses — SEVEN, not six (D-203's
"six" miscounted its own list): fz_123_7219, fz_42_6368,
fz_777_{1278,2956}, fz_7_{1591,5988,8757}. Basis for these
predictions: the scenario STATICS ONLY (drl+facts+epochs read;
no oracle/engine/dump runs yet) held against the two pinned laws.

The laws' required machinery:
- DYNAMIC (survive-the-delete): a mixed key (stated face + belief
  same value), the stated face killed (unstage), the unstage-born
  handle killed with another act QUEUED on it → engine misses the
  surviving firing (firings delta).
- STATIC (key-lifecycle rule 4): a justified-born key with stated
  siblings, the last justification breaks, the value RE-justified →
  engine one-short on finals (finals delta).

## Per-witness static reads + classification predictions

1. **fz_123_7219 → DYNAMIC, high confidence.** R0 states AND
   justifies T2(-1,true,true) in one firing (mixed key, stated-born
   → L3 sibling); R5 (sal 6) is the value-matching deleter (f1==true,
   f0<=4) → kills the stated face (unstage), re-kills the unstaged;
   R1 (sal -7) is a BARE T2 observer queued below the deleter — the
   b1/b2 shape exactly. PREDICT: firings delta = engine missing R1
   firing(s) on the unstage-born T2(-1,true,true); finals identical.
   Side machinery: R2's justified T2(12) killed/re-justified per T1
   (fresh-restart law, both sides have it); R4's stated T2(11).

2. **fz_42_6368 → DYNAMIC via or-twin self-observation,
   medium-high.** R3 (sal 4) states AND justifies T1(2.0) (mixed
   key); R2 is an or-TWIN deleter (identical branches, f0>1.0) —
   branch A's delete of the unstage-born T1(2.0) leaves branch B's
   act queued: the surviving act is the deleter's own twin. PREDICT:
   firings delta = extra oracle R2 firing(s) on T1(2.0) (delete of
   dead → no-op); finals identical. R1 dead ("zz" > "a"); R0 dead
   (f2>="" always → negated).

3. **fz_777_1278 → NEITHER law, medium-low.** No live deleters (R3's
   LHS self-contradicts; R4/R5 alphas dead on ""/"a"), no premise
   changes → no breaks, no unstages. The only TMS motion is R2's
   3-branch or insertLogical(T2("a")) where branch 2's exists
   (contains "a") becomes true only AFTER the first firing's belief
   lands — an or-branch activation-count/exists-trigger question,
   not this arc's laws. PREDICT: firings delta on R2's count (and/or
   R0's acc refire), finals likely identical (one T2("a") either
   way). If the dump shows ONLY dep-count bookkeeping differences,
   this witness leaves the I-RD family.

4. **fz_777_2956 → NEW MECHANISM candidate: break-of-PENDING-belief
   (same-RHS premise self-update), medium.** R1 justifies
   T1(100,false,false) then setF2(true)+update on its OWN premise →
   its justification (and R0's, same premise) breaks possibly while
   the beliefs are still PENDING. No value-shared keys (all four
   inserted T1 values distinct) → NEITHER pinned law's surface.
   PREDICT: divergence rides whether the broken-while-pending
   beliefs (T1(-1000000007,false,true), T1(100,false,false)) ever
   materialize — R4-firing-count and/or finals delta on those
   values. Engine has a cancel-pending path (engine.rs ~2063); the
   corner is whether its timing matches Drools.

5. **fz_7_1591 → same pending-break family, epoch-repeated,
   medium.** R2 (sal 8) justifies T2(11,1.5,false) then updates its
   own premise T1(f0 false→true) — self-break, repeated per epoch
   (epochs 1-2 insert fresh T1(false) triggers). Deleter wall: R4
   (sal 10) kills T2(f1<=10), R1 (sal 3) kills any T2. No stated
   siblings on the belief's value → static rule 4 not reachable.
   PREDICT: pending-break divergence ×3 (once per epoch); firings
   delta on R4/R1 counts and/or finals on T2(11,1.5,false).

6. **fz_7_5988 → same pending-break family, medium.** R0 justifies
   T1("ab",10), states T1("b",9), then setF1(true)+update on its own
   premise → self-break (T1("ab",10) has no same-value sibling; the
   initial T1("ab",12) is a DIFFERENT value). R1/R2 delete walls.
   R3 dead (exists self-contradicts). PREDICT: divergence on whether
   T1("ab",10) materializes — R1-firing count / finals.

7. **fz_7_8757 → unstage-kill chain present, observable UNCLEAR,
   low.** R2 or-twin (2 live branches, no-loop) fires twice: each
   states T1(-1e9,-1e9,true) (2nd append = rule 1) AND justifies the
   same value (mixed key, stated-born ×2 siblings + belief). R4
   deletes T1(f0<5) — kills s1 (no unstage, s2 remains), s2 (LAST
   stated dies → unstage), then the unstage-born. But NO other T1
   observer is alive (R1/R5 alphas dead: f2=true) — at the
   unstage-born's delete no act should remain queued → the dynamic
   law has nothing visible to save. PREDICT: the delta is in the
   R4×R2-or-twin interleaving (R4 firing count / whether the second
   or-branch's stated insert lands before or after the unstage) —
   POSSIBLY the dynamic law via an interleaving I can't hand-derive;
   possibly or-twin machinery outside this arc. The dump decides.

## Pre-registered outcome mapping

- 7219 + 6368 dumps showing the oracle's post-delete firing on an
  unstage-born handle (and the engine's absence of exactly that) ⇒
  the DYNAMIC law covers them; they join b1/b2 as port targets, no
  new mechanism.
- 2956/1591/5988 dumps showing the belief RETRACTED-BEFORE-FLUSH on
  one side and MATERIALIZED-then-killed on the other ⇒ a THIRD
  mechanism (the pending-break corner) — needs its own splitter
  cells (pending-vs-materialized × who-breaks) BEFORE any pinning;
  do NOT fold it into either law by resemblance.
- 1278 showing only or-branch/exists count differences ⇒ reclassify
  OUT of I-RD (route to the or-twin/exists machinery family).
- 8757: if the dump shows a queued act surviving the unstage-born
  delete that my static read missed, DYNAMIC; if it shows or-twin
  staging order, reclassify.
- ANY witness whose dump contradicts a pinned law's prediction ⇒
  stop, re-open that law's cell family before proceeding (the laws
  are falsifiable here — 7219/6368 are out-of-sample tests of the
  dynamic law, per the D-093 doctrine's out-of-sample demand).

Instrument bar: oracle 3× identity-stable per witness (harness),
SdDump 3× identity-normalized per witness; divergence shapes
measured (engine 1×) BEFORE dumps so each dump is read against a
known target delta. Epoch note: only fz_7_1591 has epochs (2) —
SdDump's epoch replay covers it (bc943c0); the other six are
single-fire.
