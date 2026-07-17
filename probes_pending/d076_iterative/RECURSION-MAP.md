# D-076 step A — the recursion map (pre-edit, HEAD e8d0c7f)

The handoff's required first move: map every path that re-enters
`tms_drop_act_deps` before editing. Line numbers at HEAD `e8d0c7f`.

## The cycle (there is exactly one)

```
tms_drop_act_deps(act)                                [engine.rs:11647]
  per victim jf (to_retract sorted by (seq, fact_id); alive-checked
  at its turn — an earlier sibling's cascade may have killed it):
    store.kill(jf)
    on_delete(jf, None) → on_delete_ex(jf, None, false) [8661 → 8665]
      ├─ pn_on_wm_delete / px_on_wm_delete   — shadow-struct bookkeeping
      ├─ mark_queries_pending                — flag
      ├─ LIA + trie staging (s0_add_del / s0_in.add_del /
      │    s_right.add_del)                  — STAGED, consumed later
      │    at evaluate_rule (not in the cascade)
      ├─ note_link_effects_ex                — queued/dirty flags only
      ├─ tms_p_death_sweep(f, None)          — unpark + push_activation
      │    (queue pushes only, no evaluation)
      └─ tms_eager_break(jf, true)           [11019]
           per broken act (snapshot scan of tms.by_act in list order;
           acts whose tuple contains jf and jf is dead/alpha-broken):
             ├─ expiration-routed (in_expiration_drain ∨ expiring∧all-
             │    alive) → tms.exp_deferred.push — EXITS the cycle
             └─ else → tms_drop_act_deps(act2)  [11105]
                        ← THE ONLY RECURSIVE EDGE (return DISCARDED)
```

Verified non-re-entrant: every other `on_delete_ex` callee is pure
bookkeeping/staging (list above). `evaluate_rule`, `tms_on_terminal_del`
and `execute_rhs` are never reached from inside `on_delete_ex`, and
`on_update` (the other `tms_eager_break` caller) is never called from
it either — so **cascade machines cannot nest**.

## Cascade starters (each begins at depth 0 → 1, outside the cycle)

1. `delete_fact` (7228) — external delete: `tms_route_delete` → kill +
   `on_delete_ex(victim, None, defer_acc)` → eager_break.
2. RHS `Delete` action (10533) — `tms_route_delete_ex(rhs=true)` → kill
   + `on_delete(victim, Some(ri))`; a dump7 unstage then runs
   `tms_materialize` → `on_insert` (insert side, no teardown).
3. Refire-supersede epilogue in `execute_rhs` (10551–10606) — breaks
   stale deps INLINE (its own belief-break copy, not via
   tms_drop_act_deps), then per victim: kill + `on_delete(jf, None)`
   under `pn_churn_ctx`.
4. `on_update` (8658) — `tms_eager_break(f, false)`: alpha-breaking
   update cancels justifying acts.
5. `tms_on_terminal_del` (11117; the direct call at 11220 when not
   defer/exp-routed) — the ONLY caller that USES the return value
   (the level-1 self-blocker walk at 11221). Its own callers are all
   agenda-level: k=1 terminal consume (9093), k≥2 terminal consume
   (9337), post-fire drains (7860/7908), quiescence drains
   (8154/8168), item-pop drain (8248), tms_flush_drain (11607).

Expiration drains and halt-model re-adds reach teardown ONLY through
the deferred lists (`tms.deferred`, `tms.exp_deferred`) drained at
agenda level — they never nest inside a cascade. **Step A's scope is
exactly the TMS cascade**, as the handoff assumed; nothing wider.

## The order contract the worklist must replay

- Per act: victims retract in `to_retract.sort_by_key((seq, fact_id))`
  order. `tms.seq` is not bumped during collection, so within one act
  the seq component is constant (fact-id order, stable).
- Per victim: kill → on_delete (staging, sweeps) → the victim's own
  eager-break finds (in by_act scan order), EACH fully cascaded before
  the next find, all before the victim's next sibling. Depth-first.
- Return value: level-1 victims that were alive at their turn, in
  processing order.
- `cascade_depth` semantics: nesting count; assert `< 8192` per nested
  call (the D-284 belt-and-suspenders — unreachable while
  stratification holds).

## The cut (step A design)

Replace the 11105 edge: when a cascade machine is active,
`tms_eager_break` APPENDS the broken act to the machine's collect
buffer instead of recursing; otherwise it calls `tms_drop_act_deps`
(starting a machine) exactly as today. `tms_drop_act_deps` becomes an
explicit LIFO stack machine over two frame kinds:

- `Act(act, depth)` — pop: assert depth < 8192, remove act's by_act
  entry, run the identical belief-break/collect/sort body, push
  `Victim(jf, depth)` frames in REVERSE (first victim pops first).
- `Victim(jf, depth)` — pop: if alive → kill, on_delete (its
  eager-break fills the collect buffer), drain buffer, push
  `Act(a, depth+1)` frames in REVERSE; record jf if depth == 1.

LIFO + reverse-push replays the recursion's exact DFS order: a
victim's discovered acts (and their entire subtrees) stack above the
victim's siblings. Depth is carried per frame (machines never nest, so
entry is always depth 1); the 8192 assert keeps its message and trip
condition, now checked at frame entry instead of after the frame's
belief-breaks — observable only mid-panic, unreachable as before. A
new non-reentrancy assert (collect buffer must be None at machine
entry) converts the "machines never nest" map fact into a loud check.
The `Tms.cascade_depth` field is replaced by the collect buffer
(`cascade_collect: Option<Vec<(usize, Tup)>>`); nothing else reads
`cascade_depth` (grep: decl 1705 + the one use site).

Gate: the SD census (72 EXACT, cell-for-cell) is the order-sensitive
detector — any SD drift = the refactor changed teardown order, never
census noise. Full battery per the handoff.
