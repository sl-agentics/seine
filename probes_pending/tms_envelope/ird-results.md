# I-RD splitter results (predictions: ird-predictions.md)

Oracle 3× identity-stable (byte-diff across launches); truth banked at
truths/ird_oracle_r1.ndj. Engine run for the diagnostic picture.

## The observed bit — ROBS fires on "tgt" after its delete

| cell | P-STATUS pred | P-WINDOW pred | ORACLE | engine |
|------|---------------|---------------|--------|--------|
| ird_a1_ord_fresh     | cancel | FIRE   | **cancel** | cancel |
| ird_a2_ord_stale     | cancel | cancel | cancel     | cancel |
| ird_b1_unstage_fresh | FIRE   | FIRE   | **FIRE**   | cancel |
| ird_b2_unstage_stale | FIRE   | cancel | **FIRE**   | cancel |
| ird_c1_stated_ctl    | cancel | cancel | cancel     | cancel |

Outcome pattern = the pre-registered "b1 FIRE + a1 cancel + b2 FIRE"
row: **P-STATUS pinned**. Both discriminators landed on the status
side — a1 kills P-WINDOW's fresh⇒survive arm, b2 kills its
stale⇒cancel arm. Controls behaved (a2/c1 cancel), so the salience
scaffold reproduces 4048's eager cancellation where expected.

## ⚖ THE SURVIVE-THE-DELETE LAW (pinned)

A queued activation on an **unstage-born handle** (a belief unstaged
into WM by the mixed-key kill: Delete@stated + Insert@belief in one
batch) **survives that handle's RHS delete** and fires in its normal
agenda position (b2: between its salience-0 siblings). Deletes of
stated and ordinary-justified handles cancel queued activations
eagerly. The fact itself dies either way — finals are unaffected
(oracle finals == engine finals in all 5 cells; tgt absent). The
survival is keyed on the handle's ORIGIN, not on flush-window timing,
dep-count (a1 is n=1), or justified status per se (a1/a2 cancel).

Scope note: in the reachable scenario space "unstage-born" is
extensionally equal to "WM-visible but absent from the TMS equality-
key map" (the 4048 dump shows the unstaged @5 leaves the TMS map).
The cells cannot split those two phrasings — the engine port should
pick the altitude from the engine's own delete-path recon, and the
model should carry the law as origin-keyed (the observable form).
CLOSED by ird_x1_orphan_del (ird-ladder-results.md round 3): the
other TMS-dropped population (break-orphans) turned out UNDELETABLE
(delete no-ops both sides), so the deletable TMS-dropped set =
unstage-born exactly; the law stays origin-keyed, no wider form is
reachable.

## Engine picture (the port target)

Engine matches oracle everywhere EXCEPT the surviving firing itself
(b1/b2: missing exactly ROBS('tgt') post-delete — the 4048 miss shape
reproduced minimally). The engine already has the unstage/re-kill
machinery (RD fires twice in b1 both sides; dump7 route). The port =
the act-cancel site must exempt unstage-born handles' deletes.

## Engine delete-path recon (done; port NOT built)

- tms_materialize (engine.rs:9898) RE-REGISTERS the unstaged handle
  in the TMS map (by_fact.insert + justified=Some(f) +
  had_justified) — the oracle's dump shows @5 LEAVING its TMS map at
  the unstage. The re-registration itself must stay (the b1 re-kill
  routing and fz_42_1395's fresh-restart key cleanup ride it); the
  port surface is NOT the route.
- The queued-act cancel lives downstream of on_delete_ex
  (engine.rs:7861): the delete stages into s0_add_del /
  s_right.add_del and the TERMINAL drain cancels queued acts. The
  port = an unstage-born exemption at that act-cancel site (origin
  mark set at tms_materialize), leaving WM removal, memory
  retraction, and TMS cleanup untouched.
- ⚠ Port hazard to check red-first: the surviving act fires with a
  DEAD fact's values (oracle b2 fires ROBS('tgt') two firings after
  the delete). If the engine's fire path re-reads the store by
  FactId at fire time, a stale act panics or misreads — check how
  tuples carry values before wiring the exemption.

Next per the plan: the 9902 ladder, then model extension +
population, then port validate-and-revert with full receipts
(red-first; Bryan gates the landing).
