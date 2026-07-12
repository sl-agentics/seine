# COLD-START HANDOFF: I-RD + the arc endgame (post-D-202)

_Everything a fresh context needs. State as of `bc943c0` (pushed):
THE POPULATION SLAB IS EFFECTIVELY CLOSED — census 599 → 72
(−88.0%; 12 seeds × 150, `tools/fuzz_tms_sd.py`), of which 64 are
PERMANENT (the d3/d5 no-amut eager-lead runaway family: oracle hits
its fire limit, the engine terminates by the terminates-invariant —
the census floor) and 8 are fixable tails. P1-P6 are ALL LANDED
(D-197..D-202). SIXTEEN witnesses graduated arc-total; drift bank
59; corpus 11/1124/370. Read port-target-list.md (per-round status)
+ DECISIONS D-199..D-202 before touching anything._

## What is in the engine now (rounds 1-3 + P6, all receipts green)

- The deferral CAUSE MODEL (D-197): `tms.deferred: Vec<(usize, Tup,
  u8)>` — bit0 LIA-hit (t20 flush discipline), bit1 NOT-side
  self-defeat, bit2 late-dep (the D-195 race, now INCLUDING a
  dead-at-attach premise — the del flavor), bit3 join-right, bit4
  (=16) the composite last-key RIDE (set at defer-push when an
  eager mutfirst bit1+bit2 key has NO surviving alpha-passing P —
  drains at drain[pop] ONLY).
- The park machinery (D-198/D-199): the self-defeat leak (depth-match
  `env.1 == pos` — creator-agnostic, node-sharing safe), prefix
  parks + tms_parked_suppress full-width RECORDING, the t15 revive
  (tms_parked_del: foreign left-death revives, ACTOR EXCLUSION for
  self-inflicted, lazy-plain-non-ortwin scope), the ⚖ t15
  foreign-death SWEEP (tms_p_death_sweep in on_delete_ex: WM-level
  trigger, stale-value alpha admit = the starvation law; re-add
  lead=insertion / trail=REVERSED — see the open corner below), the
  ⚖ land_eager lead-k1 unpark (self-killed premises only — the
  no-amut shape is the runaway family, engine fences it).
- The drain sites (D-200/D-201/D-202): post-fire-continue (P3
  pop-precedence: equal-salience decl-preceding preemption defers
  bit1 entries of LAZY rules to the pop; bit16 excluded),
  tms_flush_drain (flush-pre → eval → flush-MID [the eager
  decl-law: in-eval pushes drain at the rule's OWN slot] + pass-2
  flush-post), drain[pop] (unconditional). Each drain: or-sibling
  eval → ⚖ k0 CHURN (tms_churn_del_group: del-group rules
  force-evaluate pre-retract; lazy=all, eager=sink-order rj<l) →
  ⚖ mutfirst teardown (eager bit1+bit2: blocked_reverse_of the
  victims pre-retract ⇒ t0 release) → retract → unpark.
- SEINE_TMS_DEBUG tags: defer-push flags / drain[site] / park-own /
  park-leak / park-record / park-del / park-revive / sweep-revive.
- SdDump (oracle graft) now has EPOCH REPLAY (insert/update/delete
  actions + per-epoch boundaries) — 9902's epochs were silently
  skipped before; CHECK any dump target for `epochs`.

## RESUME POINT 1 — the I-RD cells (dump-first is DONE)

Ground truths banked 3× identity-stable:
`graft_targets/ird/fz_7_4048.dump.txt`, `fz_7_9902.dump.txt`,
README = the full read. Summary:

- **4048 (mixed-key kill)**: the dump7 route CONFIRMED engine-side
  (Delete@stated + Insert@unstaged-belief, one batch). THE
  DIVERGENCE: a queued activation on the UNSTAGED justified handle
  SURVIVES its RHS delete (oracle FIRING 7 fires R3 on the alpha
  tuple AFTER R2 deleted it); acts on stated (@2,@3) and ordinary
  justified (@4) handles cancel EAGERLY. The engine's whole 4048
  miss = that ONE firing (7 vs 8, finals identical).
  **LAW CANDIDATE (do not pin yet — ⚖ method law): the
  unstage-born handle's delete takes the STAGED cancellation path;
  ordinary deletes take immediateDelete.** BUILD THE SPLITTER
  CELL: two sub-scenarios — delete an unstaged-justified vs delete
  an ordinary-justified, an equal/lower-salience observer queued on
  each; does the observer fire post-delete? Oracle 3× per cell
  (TMS bar). Then check which engine path 4048's delete takes
  (tms_route_delete_ex + the act-cancel site).
- **9902 (I-ST bookkeeping)**: firing-identical 14/14 (epochs in).
  Finals-only: the multi-handle EqualityKey — stated siblings
  coexist per-handle on one key (`JUSTIFIED fhs[@4+@8+@14+@20+]`,
  `STATED fhs[@5+@10+@15+]`); the engine's value-keyed store
  dedups. BUILD THE DUPLICATION LADDER: stated-insert ×N onto a
  justified key / onto a stated key / justified-insert onto stated
  — count the WM-visible handles per route. This is the identity
  law's STATIC face; the fix (if ported) lands in the store/TMS
  key model, not the executor.
- Then: extend model_check/model_sd or a small I-RD replica with
  the law, population via the arc fuzzer if the law generalizes,
  port validate-and-revert with FULL receipts. I-RD ledger: 12
  witnesses − 941/9175 (graduated) − 2864/9360 (moved toward) ⇒
  ~8 open: fz_123_7219, fz_42_6368, fz_777_{1278,2956},
  fz_7_{1591,4048,5988,8757}, (+2864/9360 tails, fz_7_9902 I-ST).

## RESUME POINT 2 — the lazy-trail re-add order (⚖ epicycle-stopped)

The 6-case corner (sdp7005x63 / sdp6003x77 / sdp7002x33 + kin; the
fresh per-case dirs are listed in the memory — regenerate with
`python3 tools/fuzz_tms_sd.py 150 <seed> --keep` if gone):
dropping the sweep's trail-reversal fixes all six AND breaks
fz_42_5213 (its round-1 revive wants the REVERSED chain; the sweep
currently keeps the reversal — 5213 green, the six open). sd_c1
passes BOTH ways. The park list is TOO FLAT for the model's
phys/jstaged fold history — the model (model_sd.py, 0-div on all)
KNOWS the law; the engine's park-replay altitude is wrong. NEXT
INSTRUMENT: SdDump the per-round phys on the x63 shape (3
launches), read WHICH list Drools actually re-adds from per round,
then rebuild the engine's re-add source to match — never toggle
reverse/as-is again.

## The iteration loop (per change)

```
cargo build -q -p seine-harness
python3 probes_pending/tms_envelope/interposer_ladder.py --run /tmp/ip   # 6/6
python3 probes_pending/tms_envelope/validate_cells.py                    # 39/39
python3 probes_pending/tms_envelope/check_witnesses.py                   # 26/26
make diff          # 11/1124/370 + drift 59; movement ⇒ triage_xfail 10x,
                   # graduate CONVERGED (git mv + xfail-rebank + D-entry)
make lint-probes; cargo test -q                                          # 1738/0/0; 9 suites
# ⚠ D-106 receipts (worktree baseline, never stash-in-place):
cargo run -q -p seine-harness -- run probes_pending/agenda_open/*.json   # ×19 byte-identical
# the census (the metric + the panic net; ~25 min, background):
for seed in 7001 7002 6001 6003 7004 7005 7006 7007 7008 7009 7010 7011; do
  timeout 900 python3 tools/fuzz_tms_sd.py 150 $seed; done   # 0-div 12×150/150; 72 baseline
```

⚠ Census hazards learned this arc: a spin STALLS a seed (the
timeout wrapper makes it fail loud — keep it); `cargo run` batches
pick up a MID-CENSUS rebuild (never rebuild while a census runs —
kill and restart it); populations are the panic net (two
panic-class catches in three rounds).

## Standing discipline

Bryan's sequencing was: the port slab → the tails → I-RD (now
live). Commit per green slab with a D-entry; Bryan holds pushes
(everything through `bc943c0` is PUSHED). Never push v* tags.
Predictions before instrument runs; ⚖ method law (underdetermined
output ≠ finding — build the splitter first); ⚖ epicycle stop (a
rule that needs a proxy variable doesn't know its mechanism — a
toggle that flips per-case IS a proxy variable); the identity/
dedup/landing laws + the ⚠ D-106 caveat live in the workflow
memory and docs/tjupd-ledger-mechanisms.md. gen.rs walls STAY UP
(fuzz_tms_sd is arc-local recon). Oracle 9.44.0.Final+p1; rebuild:
`cd oracle && mvn -q -DskipTests package` (run FROM REPO ROOT
after — the cd hazard bit again this sitting). Gates on resume:
make diff + lint-probes + validate_cells (39/39) + check_witnesses
(26/26).
