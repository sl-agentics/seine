# THE I-RD PORT RESULTS (plan: ird-port-plan.md; Bryan-gated open)

Three families, red-first each, full receipts per family; the
graduation pass; one residual found-by-census and cured. Engine
baseline 86/750 → (closing censuses appended below).

## F1 — the static key model (`2f76db8`)

activation-backfill (tms.activated/pre_stated; pre-activation
stateds keyless, LAST-per-value backfills mapped) + the r1-event
(rhs stated-delete of a pending-mixed key: orphan siblings, unstage,
key dies; externals keep dump8) + the L6-event & PENDING-CLEAR in
tms_drop_act_deps AND the refire-supersede epilogue (⚠ the
minimal-touch bet LOST: fz_7_9902's per-epoch refire drops deps at
the epilogue — the plan's fallback applied) + TMS-dropped
materialization. Converged: l6, 8757, 9902, +BONUS fz_7_2864.
fz_123_6887 moved-not-converged (different mechanism; rebanked).
Corpus byte-identical (the scoped-clear/w1w5/epilogue risks all
cleared); SD census 72 EXACT; ird 86→81.

## F2 — the dynamic law (`0aef958`)

The port shape (found by instrumented bisection through SIX
candidate cancel sites — the staging-annihilation whack-a-mole at
five fold sites was the WRONG ALTITUDE, reverted): (1) the boundary
FORCE-EVAL (tms.force_eval; the D-201 churn precedent) — a
materialized fact's insert reaches every matching dirty terminal at
the FIRING BOUNDARY, where Drools creates the act; (2)+(3) the k=1
and general terminal del consumes exempt unstage-born facts;
(4) the j05 DEACTIVATION-PRUNE exemption — the last cancel site
(the unlinked-rule eval prunes queue acts whose facts left the
alpha active sets). The stale-value hazard never fired (the store
retains dead facts' values). Converged: b1, b2, d1, d2, r1 + the
three dynamic witnesses (4048, 7219, 6368). Corpus byte-identical;
SD 72 EXACT; ird 81→75.

## F3 — the in-flush self-break landing (in the final commit)

The current_act exclusion NARROWED to rule-shape: lazy only when
the justifying tuple holds ≥2 facts of the broken fact's type
(m3/m6/m7/s2/2442's shape); single-binding same-batch self-breaks
land eagerly. GREEN ON FIRST BUILD: the trio (2956/1591/5988) +
all boundaries (m3/m4/m6/m7, s1/s2/s3, fz_42_2442, the t20 corpus
cells — the pre-registered risk cleared). THE WIDER QUARANTINE
MOVED: 21 witnesses — ALL CONVERGED (the single-binding self-break
eager landing cured whole fz_123_*/fz_42_*/fz_777_*/fz_7_*
families).

## The graduation (in the final commit)

27 witnesses (21 F3 movers + 2864 + 8757/9902 + 4048/7219/6368),
oracle 3×-identity-stable AND engine-converged → git mv to
regressions/. Corpus tiers now 11/1124/**397**; drift bank 59→**32**;
lint 1796/0/0. (The triage tool's own batch parse crashed on an
unrelated CEP witness — worked around with the direct 3× sweep;
tool fix deferred.)

## The residual (found by the F3 census: ird 86→1)

irdp6003x128: the c5 key-survives+had_justified APPROXIMATION
leaked dump3-undeletability onto a stated insert arriving AFTER the
belief's delete (the oracle re-keys fresh and deletable). Cured by
making the justified-delete branch the REAL key-death event (orphan
siblings + key removal — every pinned observable preserved via
orphans; zero-sibling = fz_42_1395 unchanged). All 31 cells + the
residual + corpus + drift green after.

## Standing engine notes

- tms.orphans is the undeletability carrier (x1/r1/L6/c5);
  had_justified's dump3 branch remains for external/unreached
  shapes (byte-gate-protected).
- The k≥2-observer-of-unstage-born shape is outside the pinned
  envelope (the 8510 exemption covers tuple-level dels; the j05
  exemption is per-slot — noted in the plan).
- 7 (D-210) + 4 (D-211: orphan-noop, r1-event, l6-orphan,
  del-survive×2) permanent SEINE_TMS_DEBUG probes.

## Closing censuses

SD census: **72 EXACT, 0-div 12×150/150** — the SD population
untouched through all three families + the residual fix (the panic
net stayed silent throughout). ird census: **86 → 0 — ZERO
divergents on ALL FIVE SEEDS** (0/0/0/0/0), model-clean 150/150 ×5,
corners none. THE PORT IS TOTAL OVER THE POPULATION: every
generated case, every cell (31/31), and every graduated witness
(27) converge engine-vs-oracle.
