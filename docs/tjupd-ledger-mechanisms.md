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

**FIX (validated in recon, REVERTED pending the Bryan gate):** in
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
6/9/4 divergences). The residual is ONE named sub-rule: the queue
position of a **same-fact double-touch within one epoch** (u5-style
chains keep FIRST position; x199-style self-join re-entries move the
refire BEHIND the entry) — agenda-queue territory adjacent to the
D-106 caveat; needs its own discriminator ladder before the engine
port. **No engine change for this family yet** (model must reach 0-div
first, per doctrine).

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

All six cf* stay QUARANTINED with sharpened `_finding`s. Deliverables
awaiting the Bryan gate, in order of value:
1. the SET fix (6 lines, fully gate-validated in recon — re-apply and
   land);
2. the ORDER-family port (blocked on closing the ~1.6% model residual:
   the double-touch queue ladder, then the engine composition — the
   engine work spans alpha-entry staging, A′ refires, child-list
   order, and the left-memory re-add);
3. the spin root-cause arc (own slab, checker-first, D-106 caveat).
