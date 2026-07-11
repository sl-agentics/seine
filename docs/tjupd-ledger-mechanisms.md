# D-167 — the SEINE_TJUPD residual ledger: mechanisms (recon report)

_2026-07-11, follow-on to D-166. All six quarantined axis finds
reproduced on HEAD and re-verified pre-existing on a `cb7d443`
(pre-D-166-port) worktree — nothing here is a D-166 regression.
Batteries: `probes_pending/cep/tj_upd/` (minimals + s/r/m ladder cells
+ controls); model: `probes_pending/cep/tj_upd/model_tjupd_v4.py`._

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

## Status

_Updated at D-169._ The five ORDER/hang cf* stay QUARANTINED with
sharpened `_finding`s. Deliverables, in order of value:
1. ✅ the SET fix — **LANDED (D-168)**: cf6001x384 graduated to
   regressions/, tju_384_min + tju_m4_split live pins, all gates
   green (corpus 11/1084/329, TJUPD 6001-6005 = only the ORDER/hang
   names);
2. the ORDER-family port — **spec residual CLOSED (D-169): model
   0-div on 2,200/2,200** (T6 double-touch sub-rules, §2 above; the
   31-cell ladder graduated to tjdt_* — 17 order pins + 13 controls).
   The engine composition spans alpha-entry staging, A′ refires,
   child-list order, the left-memory re-add, and the T6 movability/
   relocation/self-slot — **the port awaits the Bryan gate**;
3. the spin root-cause arc (own slab, checker-first, D-106 caveat).
