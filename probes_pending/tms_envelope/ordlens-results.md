# Order-cluster dumps (gt16/gt17) — RESULTS (read against ordlens-predictions.md)

_2026-07-12. gt16 + gt17 ×3 JVM launches each: identity-normalized
3/3 identical; instrumented firing sequences == the banked oracle
sequences. The two dumps + the mb1 banked truths + the 9-case census
pinned ONE slate; encoded same sitting; witnesses 10/10._

## gt16 trace (the mf-lazy-trail mechanism)

- F0 (J fires P1): both private NotNodes' ltm [4,3,2,1] (add-at-head).
  LK(1,false)@5 JUSTIFIED n=1. Pending [Update@1 Insert@5].
- F1 (D fires P4): **J's not holds a STALE right-tuple** — LK@5 is
  GONE from WM/TMS (dep torn at J's pop-eval consuming Update@1) but
  its right-tuple persists in J's not with blocked{P2,P3,P4}; P1
  alpha'd out. **D's not processed the full break+unbreak history at
  its own first eval**: ltm REVERSED to [1,2,3,4], D fired 4 = the
  re-add scan head (pre-reversal phys order).
- F2 (J fires P3 — the "skip"): the unblock happened ONLY because
  D's Delete@4 TOUCHED J's not (P4 was in the blocked chain), forcing
  the eval that consumed the stale retract; J's re-add consume order
  = pre-fold private-phys scan of survivors [3,2] ⇒ fires 3.
- F3..F5 (D fires 1,2,3): J's new stale breaker LK(3,false) blocks
  {P2}; **P1's delete never touches J's not (P1 alpha'd out — pmut)
  ⇒ no eval is ever forced ⇒ P2 stays blocked under a dead LK and J
  STARVES** (never fires P2). D's second fold: ltm re-reversed,
  consume [1,2,3].

## gt17 trace (the nb-trail mechanism)

Three nb generations: D's not NEVER breaks (LK f1=true) — no blocked
chains, no folds; D's ltm stays [3,2,1] in place (gt10) and D fires
**[1,2,3] = its t0 staged-insert order FIFO** (updates fold into the
staged inserts, keep-first). The model's old miss: the pending_fold
PROXY ("unshared lazy set_break churns any del_not group") — a
cross-node contamination; the alpha'd set_break J's folds are its
own. P-17 CONFIRMED verbatim.

## The pinned slate (all ⚖-candidates; encoded in model_sd.py)

1. **Two-phase unbreak / stale-rtm starvation** (breaks=True lanes):
   at the justifier's pop the DROP lands (WM retract — finals
   correct) but the not-level UNBREAK stays staged; blocked tuples
   revive only on a TOUCH = another rule's WM delete/update of a P
   that reaches the node — **the alpha gate**: for set_break rules a
   pmut'd P never reaches it (lead and trail alike; gt16-F3, x108).
   Revived continuation order: trail = the rule's OWN pre-fold
   private-phys scan (new jphys, reversing per fold); lead =
   insertion (banked). No touch ⇒ starve (mb1_st/sl/dt truths are
   exactly this with no deleter).
2. **Deleter-side fold per LANDED breaking LK** (set_break lane):
   each landing REPLACES the pending fold scan with the current
   group-phys scan and reverses phys AT THE FOLD (the dump shows D's
   ltm reversed before its firing — the consume block must not
   re-reverse: pf_reversed flag). Landing count: LAZY ⇒ every
   generation; EAGER+ilfirst ⇒ every (the insert drains before its
   update); EAGER+mutfirst ⇒ last only (mid-run nets out, D-195).
   nb keys NEVER fold. Plain-lane folds keep the old
   consume-reverses flow untouched.
3. **del-lane eval-consumption** (x88/x0): the actor's own dep rides
   to its next POP (higher-salience observers glimpse each
   generation); the FOREIGN cascade stays D-076-eager (x130). gt12's
   "del = eager cascade" was salience-confounded — its observer sat
   BELOW the justifier, where pop-landing predicts the same output;
   mb1_dt/dl truths (J once, starve) hold under the slate.

## Verdicts on the pre-registered readings

P-17 confirmed. P-16-D confirmed (fold contamination was real but
the fix is per-landing folds, not staging survival alone). P-16-J:
reading (i) staged-without-dirty CONFIRMED in the stronger not-side
form (the stale right-tuple + blocked chain; the t10-leak's grown-up
sibling) — with the alpha gate as the touch scope, which none of the
pre-registered readings had fully. Reading (iii) queue-tie inversion
dead (the queue-head discipline held everywhere).

## Witnesses at encode time

x68, x41, x88, x90, x67, x108, x131, x103, x0, x51 — **10/10 exact**
(firings + finals) against banked oracle truths; validator 39/39
held through every edit; gt13/gt14 exact. Population 0-div gate:
seeds 7001/7002/6001/6003/7004 + never-used 7005 → see below.

## The fresh-seed rounds (v4→v7): three more corners, two more dumps

- v4 (5 base + 7005): 899/900 — x6001x66 flushed BY the del-lane
  edit: an EAGER del justifier's mid-run key must net out at the
  between-firings eval (the same-salience earlier-decl obs_join never
  glimpses), not ride to the pop. The del lane got the D-195
  eager/mid-run split (survivors ⇒ eager_pend; last/lazy ⇒ drops).
- v5 (+7006): 1049/1050 — x7006x34 (eager LEAD ilfirst composite,
  3 gens) broke the per-landing fold parity. **gt18 dump (3/3): D's
  ltm PRISTINE [3,2,1], rtm empty — ZERO folds; D fired t0 order
  [1,2,3]; J's own not holds the stale last-gen LK.** ⇒ eager LEAD
  cycles net out at ALL foreign nodes (x131's 2-fold match was
  parity coincidence). Encoded: eager set_break landings don't fold.
- v6 (+7007): 1197/1200 — x7002x56 (eager ilfirst TRAIL, 1 gen,
  del_not): oracle D [2,1] ⇒ **gt19 dump (3/3): D's ltm REVERSED
  between F0 and F1, consume = pre-reversal scan [2,1], no stale
  right-tuples — the TRAIL cycle PROCESSES at foreign nodes.** ⇒ the
  net-out is LEAD-topology-scoped; eager TRAIL composites fold per
  landed cycle. Plus x7007x79/x98: the del+breaks eager+mutfirst
  composite needed the D-195 last-key pop re-route (obs_lk@10
  glimpses the final generation once) — the survivors test is
  unfired-P (the live breaking LK blinds tuples()).
- All 15 witnesses MATCH after the three fixes; validator 39/39
  throughout. v7 gate (8 used seeds + fresh 7008) → below.
- v7: 1494/1500 — x7005x51/x7008x32 already-fixed classes (the gate
  ran mid-edit); x7002x119/x6003x30 = eager TRAIL **MUTFIRST**: no
  fold even D-decl-first (the key never propagated — the fold gate
  gains the ilfirst condition); x7007x10/x7008x11 = eager trail
  ILFIRST with the OPPOSITE order to x56/gt19 ⇒ the 2×2 (decl order
  × interposing observers) was built and BOTH missing corners run
  3×: **THE AXIS IS DECL ORDER** — the eager-ilfirst-trail cycle
  folds the deleter's node IFF the del_not is DECLARED BEFORE the
  justifier (sink-order shaped; corners banked as gt20a/gt20b).
  Lazy folds stay decl-unconditional (gt16 was J-first and folded);
  mutfirst-eager stays no-fold. 21 population witnesses + 2 corners
  + validator 39/39 all green; v8 gate (9 used + fresh 7009) below.
- v8: 1499/1500 — all nine used seeds CLEAN; fresh 7009 flushed
  x147 = the ilfirst nb eager last key has NO pop window (obs@7
  silent; gt13's mutfirst twin keeps the RO2 window) — the pure
  lane's last-key routing gained the RHS-order gate.
- v9: 1648/1650 — fresh 7010 CLEAN, but the x147 gate overreached
  onto LEAD: x6001x131 (obs_lk fires once on the last LK) and
  x7004x92 (obs_join fires across the full reversed P-scan) show
  the ilfirst nb eager last key KEEPS the window when the not is
  LEAD; only the ilfirst TRAIL corner loses it. Gate narrowed to
  (ilfirst ∧ trail). The eager-composite matrix {mutfirst,ilfirst}
  × {lead,trail} × {breaks,nb} now has a dump or 3×-stable witness
  in every populated cell.
- Witness registry: check_witnesses.py (26 inlined oracle truths +
  the 2 decl-axis corners; pure model regression, no oracle
  needed). v10 gate (11 used + fresh 7011) below.

## FINAL: v2 populations at 0-DIV (v10 gate, 2026-07-12)

**1800/1800 — all twelve seeds clean: 7001/7002/6001/6003 (the
original base), 7004..7010 (each a former fresh seed, now
comparison base), and NEVER-USED 7011 clean on first contact.**
Validator 39/39; witness registry 26/26 + 2 corners; engine census
steady (47/56/45/54/55/47/44/52/51/48/46/54 — the port A/B
baseline). Both order clusters closed, every fresh-seed corner
closed, ZERO open divergences in the v2 A-shape envelope.
