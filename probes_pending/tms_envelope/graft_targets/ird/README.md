# I-RD ground truth (D-202 tail; the arc plan's step 3)

SdDump (now epoch-capable — the D-202 graft extension) on the two
plan-designated witnesses, 3 launches each, identity-normalized
byte-stable. Dumps = post-firing + post-action + boundary snapshots
of WM handle/EqualityKey status, BeliefSets, and the pending queue.

## fz_7_4048 (the mixed-key kill path) — READ

- FIRING 1: insertLogical onto a STATED key ⇒ a SEPARATE non-WM
  handle (@5, `fhs[@5!@3+]`, lfh=@5) holds the belief; the stated
  handle stays the WM face (the engine's pending_vals / dump-b ✓).
- FIRING 3 (the MIXED-KEY DELETE): deleting the STATED face stages
  **Delete@3 + Insert@5 in ONE pending batch** — the pending belief
  UNSTAGES into a fresh WM handle (the engine's dump7 route ✓).
- FIRINGS 5+7 (THE DIVERGENCE SURFACE): R2 re-kills the unstaged @5
  — and R3's QUEUED activation on @5 STILL FIRES afterwards
  (FIRING 7). Acts on stated handles (@2, @3) and on the ordinary
  justified @4 were cancelled EAGERLY by their deletes; the act on
  the UNSTAGED justified handle SURVIVES its RHS delete.
  ⇒ LAW CANDIDATE (needs a splitter cell before pinning — ⚖ method
  law): the unstage-born handle's delete takes the STAGED
  cancellation path (queued acts fire); ordinary deletes take
  immediateDelete (queued acts cancel). The engine's miss on 4048
  is EXACTLY the one post-delete firing (engine 7 firings, oracle
  8 — R3(alpha)#2 missing; finals identical).

## fz_7_9902 (the I-ST stated/justified bookkeeping) — READ

- Firing-identical to the engine (14/14, epochs included). The
  divergence is FINALS-ONLY: the multi-handle EqualityKey — oracle
  final keys `JUSTIFIED fhs[@4+@8+@14+@20+]` + `STATED
  fhs[@5+@10+@15+]`: repeated stated inserts of a justified key's
  VALUE coexist as sibling handles on ONE key (each WM-visible),
  while the engine's value-keyed store dedups. Which handles stay
  WM-visible per insert route = the cell axis.

## Next (cells AFTER the dump — the plan's order)

1. A splitter cell for the survive-the-delete law: unstaged-justified
   delete vs ordinary-justified delete, an equal-salience observer
   queued on each (does it fire post-delete?).
2. A stated-onto-justified duplication ladder for the 9902 key
   bookkeeping (counts per insert route).
3. Then the model extension + port per the plan (validate-and-revert,
   full receipts).
