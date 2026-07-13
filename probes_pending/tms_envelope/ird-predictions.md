# I-RD splitter predictions (logged BEFORE the runs)

Target: the 4048 survive-the-delete LAW CANDIDATE (graft_targets/ird/
README). The banked read: a queued activation on the UNSTAGED
justified handle survives its RHS delete; acts on stated and
ordinary-justified handles cancel eagerly.

## Two live readings (the dump is consistent with BOTH)

- **P-STATUS (the banked candidate)**: the survive is keyed on the
  handle's ORIGIN — an unstage-born handle (belief made WM-visible by
  the mixed-key kill) takes a cancellation path that misses queued
  acts; ordinary deletes cancel eagerly.
- **P-WINDOW (the unbanked alternative)**: the survive is keyed on
  TIMING — @5 was born (unstage flush) and killed (FIRING 5) within
  ONE flush window, no intervening TMS flush; @2/@3/@4 were all old
  handles at their deletes (@4: three flushes intervened since its
  birth). A same-window insert+delete race would also leave the
  queued act to fire.
- P-JUSTIFIED (justified ⇒ survive) is already refuted by @4 in the
  4048 dump (ordinary justified, cancelled eagerly, n=2 deps); cells
  a1/a2 re-check it with n=1 as a side effect.

Dump evidence AGAINST reading the pending line as the mechanism: the
post-FIRING-5 TMS pending is `-` — @5's delete does NOT appear in the
TMS pending queue, exactly like @4's immediateDelete. If a staged
cancellation path exists it is not the TMS pending list (a WM action
queue would be invisible to this dump). So "takes the STAGED
cancellation path" is currently an inference, not an observation —
hence the splitter.

## The cells (all single-epoch; T0/T1 vocabulary = 4048's)

Scaffold deviation from 4048, logged: cells use EXPLICIT salience
(setup 20 > killer 10 > mid 5 > deleter 3-5 > observer 0) instead of
4048's all-default decl-order, to make deleter-before-observer
deterministic rather than pick-order-dependent. If b1 fails to
reproduce the survival, the salience regime is confound #1 and the
cells get rebuilt decl-order-only.

The observed bit per cell = does ROBS fire on value "tgt" AFTER the
deleter killed it (act queued before the delete by construction).

| cell | shape | P-STATUS | P-WINDOW |
|------|-------|----------|----------|
| ird_a1_ord_fresh | insertLogical tgt; delete next firing (no flush between) | cancel | **FIRE** |
| ird_a2_ord_stale | insertLogical tgt; RMID stages+flushes a mid insert; then delete | cancel | cancel |
| ird_b1_unstage_fresh | stated tgt + belief sibling; kill stated face (unstage); re-kill unstaged next firing | **FIRE** | **FIRE** |
| ird_b2_unstage_stale | unstage as b1 via one-shot RKS (arm-fact consumed); RMID flush intervenes; RD kills unstaged | **FIRE** | cancel |
| ird_c1_stated_ctl | stated tgt only; delete it | cancel | cancel |

## Outcome → conclusion

- b1 FIRE + a1 cancel + b2 FIRE  ⇒ P-STATUS pinned (the banked law).
- b1 FIRE + a1 FIRE + b2 cancel  ⇒ P-WINDOW pinned (the banked
  candidate is WRONG as stated; the law is a flush-window race).
- b1 FIRE + a1 cancel + b2 cancel ⇒ conjunction (unstage-born AND
  same-window) — narrower law, both properties load-bearing.
- b1 FIRE + a1 FIRE + b2 FIRE    ⇒ disjunction/justified-wide — P-
  JUSTIFIED resurrected in fresh form; needs a new discriminator.
- b1 cancel                      ⇒ the 4048 read misidentified the
  surface (minimal unstage does NOT reproduce; suspects: salience
  scaffold, the R0/R1 double-belief environment, the n=2 key, or
  epoch structure). Rebuild cells toward 4048's exact shape.
- c1 FIRE or a2 FIRE             ⇒ scaffold broken (contradicts 4048
  @2/@4 directly); fix cells before reading anything else.

Oracle bar: 3× identity-stable per cell (TMS bar). Engine runs are
diagnostic only (expected: engine cancels everywhere — the 4048 miss
is exactly the missing survive path).
