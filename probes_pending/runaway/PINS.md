# The runaway-class round (D-330) — oracle fire-limit × engine-terminates

The 22-member bank class: EVERY member is insertLogical × not
(the self-defeating logical derivation). Oracle = fire limit
100000 error; engine = terminates (the D-198/D-199 park fence —
HANDOFF-ird names it: "the no-amut shape is the runaway family,
engine fences it"). Target per the D-297 precedent: error-vs-
error parity — the engine should OSCILLATE to its own fire
limit on exactly the shapes Drools does, and keep the certified
leak-park termination on exactly the shapes Drools terminates
(t10/t11/t15 are PASSING diffed cells — untouchable).

## The decoded poles

- t10 (CERTIFIED PASS, terminates BOTH sides):
  `A() not LK() -> insertLogical(LK)` — the self-defeat leak:
  Drools leaks the dead blocker, the tuple ignores right churn.
- fz_123_4036 (the bank's pure specimen, 0 facts 1 rule):
  `(not T3(f1==false)) or (not T3(f3==true)) ->
  insertLogical(T3(false,false,100,true))` — oracle fire trail
  (SEINE_FIRE_LIMIT instrument): R0|InitialFact repeating
  FOREVER; engine: ONE firing, park-own r0 + park-leak r1, [].

Structural deltas t10 vs 4036: (a) positive premise vs
lead-not-only; (b) single branch vs OR; (c) single-defeat vs
both-branch defeat. HYPOTHESIS (from the D-198 "no-amut"
phrase): LEADNESS is the law — a LEAD not (no positive premise
in the branch) self-defeat RELEASES and refires (loop); a
non-lead not leak-parks (terminate).

## Round 1 predictions (REGISTERED BEFORE CELLS RUN)

- **q1_leadnot_selfdefeat**: `not LK() -> insertLogical(new
  LK(1))`, no facts, single branch. PREDICT oracle FIRE-LIMIT
  ERROR (high — leadness hypothesis; the or/cross-defeat is NOT
  required for the loop). Engine today: 1 firing, terminates.
- **q2_apremise_or_bothdefeat**: `A() (not LK(a==1) or not
  LK(b==1)) -> insertLogical(LK defeating both)`, A(1) present.
  PREDICT oracle TERMINATES (med — non-lead nots leak-park even
  under OR + cross-defeat). If it LOOPS instead, the driver is
  the OR/cross-branch defeat, not leadness — record and re-cut.
- t10 stays the certified non-lead single control (in-corpus).

## Round 1 MEASUREMENTS — BOTH PREDICTIONS WRONG (inverted)

- q1 (lead-not, single branch): oracle TERMINATES — 1 firing,
  [] — byte-matching the engine. The lead-not self-defeat
  leak-parks in Drools too. MATCH (and leadness is DEAD as the
  law).
- q2 (A-premise + or + both-defeat): oracle FIRE-LIMIT LOOP.
  The positive premise does NOT restore the leak.

THE OR IS THE DRIVER. Refined hypothesis: Drools' self-defeat
leak is per-tuple; an `or` compiles to MULTIPLE branch tuples,
and the TMS retraction of the derived fact releases the OTHER
branch for real (it never fired — no leak protection), which
refires and re-derives — the branches relay the oscillation.

## Round 2 predictions (REGISTERED BEFORE CELLS RUN)

- **q3_or_single_defeat**: `( not LK(a==1) or not LK(b==99) )
  -> insertLogical(LK(1,1))` — defeats branch A only; branch B
  stays true forever (its justification persists). PREDICT
  oracle TERMINATES, derived LK present at end (med-high): no
  relay — B's match never dies, D stays stably justified, A
  stays blocked.
- **q4_two_rules_no_or**: two separate single-branch
  self-defeating rules (own types each). PREDICT oracle
  TERMINATES (high): the leak is per-rule; "two terminals" via
  separate rules ≠ or.
- **q5_census**: grep the 22 members for `or` — PREDICT ALL 22
  carry an or-not compound (high; the class signature).

## Round 2 MEASUREMENTS — both predictions EXACT, then the confound

- q3 (or + single-defeat): oracle 2 firings, LK PRESENT —
  PREDICTED. The surviving branch's justification is stable; no
  relay.
- q4 (two rules, no or): oracle 2 firings, [] — PREDICTED.
- Census: 18/22 carry an or-not compound; FOUR DO NOT
  (fz_42_4442, fz_7_8623, fz_7_930, fz_7_9628) — a second
  mechanism, so "or is the driver" cannot be the whole law.

THE CONFOUND FOUND: every LOOPING not observed so far is
CONSTRAINED (930: join `f1 == $b`; 4442: alpha `f1 in
(true,false)`; q2/4036 or-branches: alpha) and every
TERMINATING self-defeat not is BARE (`not LK()` — t10/t15, q1,
q4). Round 1's q1-vs-q2 varied constrainedness along with the
or. D-031's linking law is the suspected mechanism (an
unconstrained not is linked only while its right input is
EMPTY; a constrained not is always linked — the unlink is the
oracle-side "park").

## Round 3 predictions (REGISTERED BEFORE CELLS RUN)

- **q6_alpha_not_selfdefeat**: `not LK(a == 1) ->
  insertLogical(LK(1,1))`, no or, one rule, no facts. PREDICT
  oracle FIRE-LIMIT LOOP (high). If so, the or was INCIDENTAL —
  constrainedness alone splits the classes.
- **q7_join_not_selfdefeat**: `T0($x : f0) not LK(a == $x) ->
  insertLogical(LK($x, 1))`, one T0(7). PREDICT oracle
  FIRE-LIMIT LOOP (high) — the join-constrained flavor (930's
  core).
- **q8_or_unconstrained**: `( not LK() or not L2() ) ->
  insertLogical both-defeating`? — inexpressible (one RHS
  derives one type; LK defeats branch A only = q3's shape).
  Instead: `( not LK(a == 1) or not LK(a == 1) )` twin
  constrained branches — already covered by q2. SKIP; the or
  axis is settled by q6 if it loops.

## Round 3 MEASUREMENTS — constrainedness DIES too

- q6 (alpha-constrained, no or): oracle 1 firing, terminates.
- q7 (join-constrained, one T0): oracle 1 firing, terminates.

FIRE TRAILS (SEINE_FIRE_LIMIT=13) of the no-or loopers:
- 930: strict alternation T0(100), T0(-1), T0(100), ... — TWO
  tuples of ONE rule relaying.
- 4442: strict alternation R0(T0=100), R1(T0=4), ... — two
  RULES relaying.

## THE LAW (fits all ten measurements)

**The self-defeat park/leak holds only while the parked tuple's
not node sees no FOREIGN churn: another actor's blocker
insert/retract at the SAME not-node (same right type) REVIVES
the parked tuple. Two or more self-defeating tuples sharing a
not-node type mutually revive — the relay oscillation (or
branches, sibling rules, or sibling left tuples alike). A lone
self-defeating tuple, or park-mates over DISJOINT types (q4),
terminate.** Checks: t10/t11/t15/q1/q6/q7 lone → terminate ✓;
q3 or+single-defeat → survivor is stable, no churn after its
one firing → terminate ✓; q4 two rules disjoint types →
foreign churn at DIFFERENT nodes → terminate ✓; 4036/q2 or +
both-defeat, 4442 two rules same type, 930 two tuples same
rule → shared-node relay → LOOP ✓. The engine's t15 lane
already revives on foreign LEFT-death with actor exclusion —
the missing arm is foreign RIGHT-churn revival.

## Round 4 predictions (REGISTERED BEFORE CELLS RUN)

- **q9_two_tuples_disjoint_defeat** (930's core: `T0($x : f0)
  not LK(a == $x) -> insertLogical(LK($x, 1))`, T0(7) + T0(9)):
  PREDICT oracle FIRE-LIMIT LOOP (high) — each tuple's derived
  LK never blocks the other, but its insert/retract churns the
  shared node → mutual revival.
- **q10_external_churn_revive** (q7 + epoch 1 inserting STATED
  LK(99, 5) — join-ineligible foreign churn): PREDICT the
  parked tuple revives at epoch 1 → ONE extra firing +
  re-park → terminates, firings 2 (med; if firings stay 1, the
  revival requires join-ELIGIBLE churn — record which).

## Round 4 MEASUREMENTS — both EXACT; the law confirmed

- q9: oracle FIRE-LIMIT LOOP ✓ (shared-node relay with disjoint
  per-tuple defeats).
- q10: oracle 2 firings, stated LK survives ✓ — external
  join-INELIGIBLE churn revives once; the tuple re-parks and
  terminates. The revival trigger = node-reaching churn, not
  release.

## THE PORT (D-330)

1. **tms_parked_not_churn** (new sweep, called at the END of
   on_insert and on_delete_ex): foreign churn of a fact
   reaching a parked rule's NOT position (constant alpha only —
   q10) unparks that rule's entries and re-activates each iff
   every positive still alpha-admits and EVERY not is OPEN
   (live-blocker scan through JoinEnv::allowed); blocked
   entries just lose the park (re-sync — the normal release
   re-activates later). Actor exclusion is TUPLE-level:
   `churn_actor` (a new Tms field, set by the post-fire drain
   around tms_on_terminal_del — current_act is cleared by
   then) falling back to current_act; a lone self-defeat sees
   only its own churn and stays parked (t10/t11/t15, q1/q6/q7
   byte-hold).
2. **The park-leak or-SIBLING gate is EAGER-only**: sd_b4
   (no-loop or-twin, fires once) keeps the sibling park + queue
   cancel; a LAZY or-sibling — the D-198-era "Family II"
   fenced runaway (sd_b3) — is left released, refires, and its
   teardown churn relays (4036/q2). Same-rule t21 blocked-list
   parks unaffected.

Post-port: q1-q10 ALL PASS; t10/t11/t12/t15 PASS; 930/4442/4036
PASS (error parity via D-013/j21 "fire limit" agreement — the
engine now runs to ITS 100000 like the oracle).

## The relay perf hunt (the port's gate)

The big members ground for MINUTES per run post-port (fz_123_1338:
>120s without reaching the limit; rate decayed 4100→1400
derivations/s) — unacceptable: the xfail drift gate re-runs every
banked cell engine-side. Chase (timers + per-rule split + gdb
child-sampling; perf/valgrind unavailable): t_eval dominant and
super-linear with FLAT structure sizes everywhere dumped; the hot
frame = phreak::do_join_node at one PC across samples. THE
QUADRATIC: `Node::kill_child` flags children dead and removes
child_ix but never prunes the id from the per-key `by_left`/
`by_right` index lists — the relay's IMMORTAL left tuples
accumulate one dead id per cycle and every child walk re-scans
all-ever. THE FIX: amortized dead-id compaction at kill_child
(retain live when dead*2 > len && len >= 8 — the D-297 threshold
pattern; same-batch create_child before_* position anchors stay
intact below threshold). MEASURED: 10k/20k/40k/100k fires =
0.24/0.48/0.98/2.48s — LINEAR; was 26s at 40k. Instruments kept:
SEINE_FIRE_LIMIT env in BOTH runners (diagnostic override,
certified 100_000 when unset — the oracle fire-trail dump rides
the same env).

## Round 5: the dyn-salience refinement + FULL SWEEP

First full sweep: 21/22 PASS; fz_777_6662 resisted — its R2 is
a DYN-SALIENCE or-twin, and the gate lumped dyn with no-loop as
"eager" (parks). The oracle RELAYS dyn-salience or-twins; the
certified sd_b4 basis is NO-LOOP only. Gate narrowed to
`!no_loop` → 6662 PASSES; t10/t21/t20a and the relay members
hold. **ALL 22 MEMBERS PASS (error-vs-error parity)** — the
oracle-runaway class is CLOSED: the engine oscillates exactly
where Drools does and reaches its own fire limit in ~2.5s
(linear post kill_child compaction).

## THE PRODUCT DECISION (Bryan) + the diagnosis enrichment

Bryan's call on review: keep the port; REFRAME as detection
(the settle was never a stable model — the shapes are Russell
loops, no stable assignment exists; the note reads "your rules
were contradictory"); ENRICH the fire-limit error with the
relay diagnosis (Tms.self_defeats at park-own → "; self-
defeating insertLogical relay: rule(s) R0 derive facts that
falsify their own 'not' support" — the certified "fire limit"
prefix verbatim, all 22 stay green); record as a DELIBERATE
D-entry with fenced diff expectations ("expect_error": true —
a new lint contract for certified error-parity cells); the
compatibility note goes in LOUD. A stabilizing-TMS mode stays
a possible opt-in divergence, never the default.
