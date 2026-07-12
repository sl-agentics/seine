# D-167 — the SEINE_TJUPD residual ledger: mechanisms (recon report)

_2026-07-11, follow-on to D-166. All six quarantined axis finds
reproduced on HEAD and re-verified pre-existing on a `cb7d443`
(pre-D-166-port) worktree — nothing here is a D-166 regression.
Batteries: `probes_pending/cep/tj_upd/` (minimals + s/r/m ladder cells
+ controls); model: `probes_pending/cep/tj_upd/model_tjupd_v4.py`._

## ⚖ STANDING BEHAVIORAL LAW (Bryan, D-172): THE IDENTITY MODEL

**The engine kills by VALUE; Drools kills by tuple OBJECT identity.**
Drools' tuple lifecycle is object-scoped: a staged delete retracts the
specific LeftTuple/child objects it was staged against, and a
re-created tuple with an identical fact composition is a NEW object
that earlier-staged deletes cannot touch. The engine's children,
queue, and staging are value-keyed (`Tup = Vec<FactId>`), so ANY
delete whose processing is DEFERRED past a same-value re-creation
over-reaches and kills fresh state. The two models coincide exactly
as long as processing ORDER preserves del-before-recreate — every
divergence in this class is therefore an ORDERING deferral made
lethal by value-keying, and the faithful fix is to restore the stage
order (as D-172 does), not to bolt on identity.

Known instances: the `Staged` no-fold rule (c13 — del+ins never fold
because the re-created child is a NEW object) and the D-171 relink
SET loss (§4). TRIAGE RULE: a SET loss whose trace shows a deferred
del draining after a same-value re-creation is THIS law — check the
del's deferral path (dstash / unlinked staging / fire batching)
before hypothesizing anything else. Expect it to explain future
finds wherever deletes defer: TMS retraction cascades, expiration
drains, halt-model re-adds.

## The ledger resolves into THREE mechanisms

### 1. SET family — cf6001x384 (the correctness gap): a stale staged
### UPD starves the D-125 flush and the pair is lost across unlink/relink

Minimal (`tju_384_min`): CH2 = `E0() E1(after[0,100]) E2(after[0,100])`;
E2 @expires 100 expires at the ep0 advance → **CH2 unlinks** (E2 side
empty); ep1 updates an UNRELATED E1 (leaving a staged upd on node0),
then inserts E1(205) + E2(206). Trace-pinned mechanism: E1(205)'s
arrival reaches the D-125 per-arrival flush loop while the rule is
unlinked, but the **eligibility gate requires clean insert-only staging**
(`s_right.upd.is_empty() && …`); the stale upd fails it, so the arrival
falls to `self_drain_delta` — memory WITHOUT children — and the later
relink evaluation never re-derives the (162,205) pair. One CH2 firing
lost (SET), durable across epoch splits (`tju_m4_split`). Controls: no
update (`tju_m2`), no advance (`tju_m3`), no initial E2 (`tju_m5`) all
converge.

**FIX (validated in recon; ⇒ LANDED at D-168, Bryan-gated):** in
`stream_flush_ex`'s self-drain fallback, leave MIXED staging (any
staged upds alongside the ins) in place for the eval instead of the
childless self-drain:

```rust
} else if (!self.trie[ni].node.s_right.ins.is_empty()
    || !self.trie[ni].node.s_left.ins.is_empty())
    && self.trie[ni].node.s_right.upd.is_empty()
    && self.trie[ni].node.s_left.upd.is_empty()
    && self.trie[ni].s0_in.upd.is_empty()
{
    … self_drain_delta …
```

Gate evidence with the fix in-tree: `tju_384_min`, `tju_m4_split` and
the FULL `cf6001x384` (19/19 firings) all PASS; corpus 11/1084/328
byte-identical; tjt battery 26/26; mju population 0/200 (seed 42);
fuzz_cep 313/941/943/945 ×400 = 0; SEINE_TJUPD 6001-6003 ×400 = only
the known ORDER/hang names (see D-167 for the full list); cargo green.

### 2. ORDER family — cf6001x245 / cf6003x274 / cf6004x233 / cf6005x208:
### the self-join/anchored modify composition

All four minimize to one- or two-fact-margin reproducers of the same
composition: a temporal join whose `$a` carries an alpha constraint
(`tag=="z"`), mutated by updates. The v4 model (built ON the certified
D-125 Node + D-156 arrival discipline) pins, via ~20 ladder cells and
a 3-knob grid over live-oracle populations:

- **alpha ENTRY** (y→z): a fresh left-insert of the anchor that scans
  the **pre-move** `$b` memory (certified forward-scan + prepend);
  **alpha EXIT** kills the anchor's children and **cancels their
  same-epoch pending fires** (matchCancelled).
- **in-place anchor update** (tag-touched, value-blind): refires the
  anchor's own children in REVERSED child-list order (**A′**); the
  child list APPENDS on creation and MOVES-TO-END on refire; ts-only
  updates do NOT A′-refire (the `$a` watch mask is {tag}).
- **`$b`-side refires on every update** (β listen-all), ltm-scan +
  prepend; **every update moves the `$b` tuple to the memory tail**
  (phase C, AFTER the entry scan — r3's double value-identical update
  restores insertion order only by coincidence, which is why single-
  update controls pass); a surviving ANCHOR also moves to the tail of
  the LEFT memory on tag-touched updates.
- per-epoch dedup keeps the first staging position; rendering at the
  fire boundary.

**Validation: 1,181 / 1,200 (~98.4%)** across three seeds (200+400+400,
6/9/4 divergences). The residual was ONE named sub-rule: the queue
position of a **same-fact double-touch within one epoch**.

**⇒ RESIDUAL CLOSED (D-169): spec 0-div on 2,200/2,200** (bank seeds
11/21/31 + fresh 41/51/61, 200+400×5), via a 5-round 31-cell
discriminator ladder (`probes_pending/cep/tj_upd/ladder_dt*.py`; cells
graduated to `tjdt_*.json` — 17 open_divergence order pins + 13 live
controls). The T6 sub-rules (full statement in the model docstring):

- **Emission movability**: an upd emission staged by a TAG-writing
  action (noop y→y, both-fields, in-place z→z, exit z→y) is
  MOVABLE-by-f; ts-only actions stage ANCHORED emissions.
- **Relocation**: re-emitting a movable emission during a LATER
  alpha-ENTRY of the SAME fact moves it to the current position
  (behind the entry's ins batch). Anchored / different-fact /
  same-action / non-entry re-emissions keep their first position (the
  u5 keep-first discipline — u5 was different-facts all along);
  ins-staged emissions absorb re-touches.
- **Self-slot**: an entry's scan sees the entering fact ITSELF at its
  pre-epoch slot when its same-epoch moves were tag-class (exits
  included), at its moved slot after ts-only moves; other facts always
  at current positions. Moves are otherwise immediate post-scan, every
  class (the D-167 "phase C" statement stands).
- ⚠ **Oracle flake** (fz_42_84 class, quarantine-and-document): the
  exit-move's visibility to a later same-epoch DIFFERENT-fact entry
  scan is JVM-nondeterministic (cell ex9: 16 moved / 2 unmoved across
  JVM instances, each internally consistent). The model encodes the
  moved majority; ex9 is deliberately NOT in the battery.

**Engine port (NOW UNBLOCKED, Bryan gate pending)** — the D-167 seam
map stands (alpha-entry pre-move scan / A' reversed child-list /
lseq-refresh on the mask-miss path / anchor left-re-add), plus the T6
delta: the per-arrival update flush must reproduce movability +
entry-relocation + the self-slot scan.

### 3. HANG witness — cf6002x359: the D-117-guarded executor spin,
### now with a 4-fact minimal

`tju_359_spin_min`: E0(9,dur80)@S1 + E2(0) + two E1s (load-bearing via
their expiration deadlines); `advance 51`; then `advance 50` +
**delete of the just-expired E2** + a fresh matching pair. The engine's
`next_activation` re-add cycle trips the 50M-step spin guard. This is
the KNOWN deferred E1-hardening root cause (D-117), not a new family;
the minimal makes the eventual checker-first recon tractable. ⚠ D-106
halt-model caveat applies to any fix attempt.

### 4. The RELINK-SET family — tu51x80 / tu51x187 (D-171 recon): the
### exit's s0-del outlives the unlink and kills the re-entry's pairs

Minimal (`tju_relink_min`, from tu51x80 via the now-committed
`tools/minimize_keyed.py`): RJ = `$a : E1(tag=="z") $b : E0(before
[0,100] $a)`; E0(62,y) + E1(84,z) fire initially; ep0 = the anchor
EXITS (rule unlinks) and a backlog E0(72,z) arrives; ep1 = the anchor
RE-ENTERS. Oracle re-derives both pairs (ts0 frozen at 84); the engine
derives NEITHER.

Trace-pinned mechanism (SEINE_TRACE + EVAL/FLUSH debug):
1. the EXIT's s0-DEL is consumed by NOTHING in its own epoch — dels do
   not count as flush-TOUCH (`touched_node` checks ins/upd growth
   only) so the exit's own trigger flush skips the eval, and while the
   rule is UNLINKED no later eval folds the s0 staging (lazy PHREAK —
   the oracle defers too);
2. the ep0 backlog arrival's flush eval (queued && touched) runs with
   the del DSTASHED ("staged deletes from earlier actions batch to the
   fire") and pairs the arrival against the STALE left;
3. at the RE-ENTRY the dstash again hides the del from the relink
   eval, which re-creates the pairs (same tuple VALUES); the del then
   drains at the NEXT FIRE — after the re-entry — and the value-keyed
   child/queue kill destroys the re-created pairs. Drools' relink
   drain processes del-then-INS in stage order and kills only the OLD
   tuple OBJECTS (the Staged no-fold comment's object-identity point,
   made lethal by the deferral).

Ladder (probes_pending/cep/tj_upd/tju_relink_*): the SAME-EPOCH
exit+re-entry diverges too (`_sameepoch` — the dstash hides even a
same-epoch earlier-action del; the SJ analogs never hit this because
their re-entries carry a $b-side upd ⇒ ineligible ⇒ the D-170 replay
consumes del-then-ins in order); the del survives gap epochs
(`_gap`); WITHOUT the backlog the engine converges (`_nobacklog`,
live control — the del drains at its own epoch's pop when no arrival
eval interleaves).

**FIX (validated in recon, REVERTED pending the Bryan gate):** the
dstash exempts an s0-del whose fact RE-ENTERS in the same flush (a
fresh same-fact s0-ins) — the eval then processes del-then-ins in
stage order, exactly Drools' relink drain:

```rust
let mut s0_dtail = t.s0_in.del.split_off(dd0);
// D-171 (relink out-and-back): a pre-existing s0-del whose
// fact RE-ENTERS in THIS flush (a fresh same-fact s0-ins)
// stays VISIBLE — the eval then processes del-then-ins in
// stage order (Drools' relink drain kills the OLD tuple
// objects before the fresh pairs derive).
let fresh_ins = t.s0_in.ins.len() - p.0.min(t.s0_in.ins.len());
if fresh_ins > 0 && !s0_dtail.is_empty() {
    let fresh: Vec<FactId> =
        t.s0_in.ins[..fresh_ins].iter().map(|(f, _, _)| *f).collect();
    let mut keep: Vec<(FactId, Origin, u8)> = Vec::new();
    s0_dtail.retain(|e| {
        if fresh.contains(&e.0) {
            keep.push(e.clone());
            false
        } else {
            true
        }
    });
    t.s0_in.del.extend(keep);
}
```

Gate evidence with the fix in-tree: tju_relink_min + _sameepoch +
_gap + tu51x80 + tu51x187 ALL PASS; corpus 11/1084/333
byte-identical; fast battery 71/71; population **2,199/2,200** (only
tu51x207 — the separate 3-touch ORDER compound — remains); fuzz_cep
313 + SEINE_TJUPD 6001 ×400 = 0; cargo 9. Executor/halt machinery
untouched (the fix is flush-layer — D-106-clean).

### 5. The 3-TOUCH ORDER compound — tu51x207 (D-173 recon): the
### childlist move is per-ACTION, not per-staged-upd

Minimal (`tjx207_min`, keyed-minimized with the ORDER-class signature
`'!firing count differs'`): SJ; four y-facts; ep0 = [tag-VI on F,
ENTRY of F], ep1 = [in-place on F]. The ep1 A′ block fires the
self-pair at its entry-scan slot (engine) instead of the moved END
(oracle/model).

MECHANISM: the model — 0-div on this exact cell — re-runs phase B
($b-refire) at EVERY touch against the CURRENT children: the ENTRY
action's own phase B finds the just-created self-child and MOVES it
to the childlist end (the emission dedups away; the move side effect
lands). The engine's replay dedups the staged upd to ONE RUpd op at
the FIRST touch's stamp (TupleSets keep-first — correct for
EMISSIONS), and the childlist re-adds rode that op — so with a
leading same-epoch tag-VI the refire pass runs BEFORE the entry
exists and the self-child never moves. The 3-touch is structurally
necessary (why the 2-action generator can never reach it): without
the leading VI the un-dedup'd refire stamp lands AFTER the entry and
the move happens.

DISCRIMINATION (the round-3 bar): the x2l1-vs-x2l2 minimal pair
differs ONLY by the leading VI and the engine flips exactly as the
stamp-elision predicts (`ladder_x207.py`, 5 cells, all oracle-stable,
model==oracle on every cell). Controls: `x2l3` (VI in its OWN epoch —
vacuously correct: the prior-epoch move relocates the memory slot so
the scan itself orders the self-child last) and `x2l4`'s ep2 (an
un-dedup'd in-place refire DOES move). `x2l5` pins the move's
position in time (at the entry touch, BEFORE later partner appends).
NOT the identity-model law: no delete exists anywhere in the
composition — the law's activation condition is categorically absent.

**FIX (validated in recon, REVERTED pending the Bryan gate):** the
childlist re-adds move from the RUpd (emission) op to the per-ACTION
RMove ops in `temporal_upd_replay`:

```rust
Op::RMove(f, tagc, log) => {
    // D-173: the $b-refire's CHILDLIST move-to-end is a
    // per-ACTION side effect too — the model re-runs
    // phase B each touch against the CURRENT children,
    // even when its emission dedups away.
    let ids: Vec<usize> =
        node.by_right.get(&f).cloned().unwrap_or_default();
    for c in ids {
        if !node.children[c].dead {
            node.re_add_left(c);
        }
    }
    ... existing rtm move ...
}
// and Op::RUpd's emission loop drops its re_add_left(c)
```

Gate evidence with the fix in-tree: ladder 5/5 engine==oracle;
tu51x207 + tjx207_min PASS; fast battery **81/81**; corpus
**11/1084/333** byte-identical; cargo 9. CONTROLS (per the
recon directive, not evidence): the model population
**2,200/2,200** (the axis fully converges) and fuzz 313 + TJUPD 6001
×400 = 0.

## Status

_Updated at D-173._ Deliverables:
0. ✅ the RELINK-SET family (§4) — **LANDED (D-172)**: the dstash
   relink exemption is in-tree; tu51x80/x187 + the tju_relink_*
   ladder are live pins. The IDENTITY-MODEL LAW (top of this doc) is
   the standing distillation;
0b. the 3-TOUCH ORDER compound (§5) — **mechanism CRACKED, fix
   validated in recon and REVERTED — awaiting the Bryan gate**
   (tu51x207 + tjx207_min + the x2l* ladder; landing takes the model
   population to 2,200/2,200);
1. ✅ the SET fix — **LANDED (D-168)**: cf6001x384 graduated;
2. ✅ the ORDER family — **spec closed (D-169, 0-div on 2,200) and
   ENGINE-PORTED (D-170)**: cf6001x245/cf6003x274/cf6004x233/
   cf6005x208 GRADUATED to regressions/ (corpus 11/1084/333); the
   port = the T6 replay (stamp-ordered per-action op replay for
   upd-carrying temporal 2-pattern batches) + pending per-action
   moves + the self-slot view + terminal movability/relocation with
   entry-eval ins-first consume + the A' child-list discipline.
   Population A/B: **+719 fixed / 0 regressed**; the TJUPD axis
   6001-6005 now flags ONLY cf6002x359. Three pre-existing residuals
   quarantined (tu51x80/x187 = SET losses in the exit→unlinked→
   re-enter relink shape — kin of the §1 family, own recon; tu51x207
   = a 3-touch ORDER compound);
3. the spin root-cause arc — cf6002x359 / tju_359_spin_min, the LAST
   open TJUPD item (own slab, checker-first, D-106 caveat).
