# D-298 map — the staged-scan quadratic: diagnosis flip + the StagedList port

## The named item vs what the profiler actually said

D-297 named the residual "the staged del-dedup scan" (`Staged::add_del`'s
`del.iter().any` per teardown delete) from a single gdb sample. The slab
opened by BUILDING that fix — a stale-positive `del_set` (D-266/267
pattern), registration mirroring `seen_add` at the cross-Staged sites —
and it was provably behavior-identical (`set ⊇ del` ⟹ `contains && any`
≡ `any`) and MEASURED ZERO: ub_deep_99k stayed at 52.1s.

Multi-sample gdb (6 samples + a 30-frame trace) then named the true
mechanism. The hot closure is add_del's closure#0 — the **ins-cancel
`position` scan** — under this chain:

    delete_fact → tms_eager_break → tms_drop_act_deps (D-293 machine)
      → on_delete → on_delete_ex → Staged<FactId>::add_del

ub_deep_99k's rule "WGone" (`P(tag=="e1") not T()`) is UNLINKED during
the 99k grow (no P exists), so its T staging accumulates 99,000
unconsumed INS entries. The teardown then deletes oldest-first; each
add_del is a cancel HIT (staged-but-never-materialized ins annihilates
with the del) whose entry sits at the far end of the deque — a
full-length walk per victim. A membership set cannot help: the scan is a
hit that must produce a position, not a dedup miss. The `del_set` was
REVERTED (subsumed) and the structural fix went in instead.

Why the del list itself was never the problem: drained windows get their
whole Staged take()n (fresh `seen`), so teardown dels there ride the
D-266 seen-miss O(1) fast path; only UNDRAINED stagings carry the fat
lists, and what they carry is ins.

## The fix — StagedList (engine/src/phreak.rs)

Drools' staged tuples are intrusive doubly-linked; removeFromStaging is
O(1). The port's `VecDeque` staging made every add_* dedup/cancel a
linear walk. `Staged`'s ins/upd/del are now `StagedList<T>`:

- entry arena + live-order id deque (arena `None` = tombstone) — the
  live order IS the certified staging order;
- per-key id lists kept in LIST order (front push prepends, back push
  appends), so first-occurrence-by-key — the exact entry every old
  `position`/`any` scan selected — is a key-list head: O(1)
  find/cancel/dedup, dup keys handled exactly (blind concats CAN stage
  the same key twice, so "check the back first" tricks were not sound);
- ends trimmed on removal; amortized compaction (dead*2 > order len,
  floor 64) rebuilds arena+order+keys in live order;
- the public surface mimics VecDeque (iter/len/is_empty/extend/
  split_off/drain/insert/remove/Index/From/FromIterator/Debug), so the
  raw engine.rs sites compile verbatim; split_off tails leave as plain
  VecDeques and re-enter through extend. Index-based ops resolve live
  rank by walk — confined to rare paths (slot restore, qce `[call_idx]`).

Every op maps 1:1 onto the VecDeque op it replaced, restricted to the
live subsequence. O(1) conversions inside Staged: add_del (ins-cancel via
remove_first_by_key, + _indexed variant for the slot_memory rank), upd
removal, del dedup via contains_key; add_ins_ph / add_upd_ph clash checks;
merge_into_pending move-to-head + del check; remove_ins/remove_upd.
Engine-side `.iter().any` key scans were left verbatim (correct, now
candidates for contains_key conversion only if ever measured hot).

`seen` (D-266) stays: its miss-path short-circuit is still the cheapest
exit and its invariant is untouched. With StagedList the exact scans it
guards are O(1) anyway; retiring `seen` is a possible later cleanup, not
this slab.

## Measured (debug)

| cell | pre (6483f68) | post |
|---|---|---|
| pr_ub_deep_9000 | 0.78s | 0.41s |
| ub_deep_99k | 52.1s | 4.69s (byte-identical output; ~linear: 11.4× time for 11× depth) |

Graduation per the bench_slow README: ub_deep_99k → scenarios/probes/
pr_ub_deep_99k.json; scenarios/bench_slow/ deleted.
