# TmsDump lens — gt13 results (read against tmslens-predictions.md)

_2026-07-12. Instrument: SdDump + the D-194 BeliefSet lens (this
sitting). gt13 ×3 JVM launches: identity-normalized full dumps 3/3
identical; TMS lines raw-identical 3/3 (they carry no identity tags).
Firing sequence with the instrument attached == the banked oracle
sequence (instrument non-perturbing on this shape). Instrument
cleanliness check passed: at PRE-FIRE the pending line still shows
`Insert@1 Insert@2` AFTER getFactHandles() ran ⇒ the lens does not
flush the propagation queue; the factory/entry-point fallback never
executed (TMS reached through the live BeliefSet only)._

## The trace (belief layer, per firing)

- PRE-FIRE: P@1,P@2 key=- ; keys -; pending [Insert@1 Insert@2]
  (external inserts ride the queue; drained by F0).
- F2 = RJ(P1): store ALREADY holds LK1@3 JUSTIFIED bs[n=1 wma=-]
  dep{RJ act=false q=false tup=(P1@1)}. Pending [Update@1 Insert@3]
  — the RHS pair queued in RHS order (upd first: mutfirst).
- F3 = RJ(P2): **LK1@3 GONE — store handle, key, everything; zero
  pending residue.** LK2@4 JUSTIFIED n=1 dep{tup=(P2@2)}. Pending
  [Update@2 Insert@4]. Beta side: RJ's join rtm lost P1 (the staged
  right-delete was CONSUMED); RO2's join ltm EMPTY (LK1 never
  entered), rtm now [P2,P1] (add-at-head).
- F4/F5 = RO2 ×2 on LK2(2): pending EMPTY (Update@2/Insert@4 drained
  post-run), yet **LK2@4 still n=1, dep INTACT, justifier match
  act=false q=false** — the zombie-justifier window. Beta: RJ's join
  rtm STILL physically holds P2 (its right-delete staged, unconsumed
  — right staging is invisible to the PATH left-staged lists); RO2's
  rtm reordered [P1,P2] (gt9 tail-relocation on P2's update),
  reversed scan ⇒ pairing P2-then-P1 (dossier reconfirmed).
- FIRE-BOUNDARY: **LK2@4 GONE** (store+key); RJ's join rtm EMPTY
  (staged delete consumed post-RO2); RO2's ltm cleared; WM = P-only
  == model_sd finals.

## Verdict on the pre-registered readings

- Common ground CONFIRMED: both deps attach at insertLogical exec
  (n=1 at the inserting firing's own dump). **H2 (mutfirst gates
  attachment) FALSIFIED. H3's sub-question: LK1's dep DID attach.**
- **H1 falsified in its WM/queue form**: the session queue drains
  BETWEEN the justifier's own consecutive firings (F2→F3), not at
  run end. What holds is the agenda-layer fact (observers gaining
  staged work mid-run do not preempt the run) — that layer is the
  D-106-adjacent interposer arc, NOT this lens's lane.
- P-A's mechanism + P-C's endpoint, neither verbatim — the trace
  picks a FIFTH reading:

  ⇒ **EVAL-CONSUMPTION LANDING (dump-grounded): the amut
  update-break's dep-teardown lands when the JUSTIFIER'S network
  eval consumes the staged break — NOT at the WM action's drain.**
  (a) Break drained mid-run (justifier has more firings): consumed
  at its between-firings eval ⇒ teardown + inline logical retract
  complete before the next firing; the LK's own queued network
  Insert then nets out — observers NEVER see it (LK1: no RO2/RO1
  activations, no belief residue at F3).
  (b) Break drained after the run's last firing: the staged
  right-delete sits unconsumed while higher-salience observers run
  (belief n=1, zombie justifier) ⇒ the LK is OBSERVABLE (RO2 fired
  twice); teardown lands at the justifier's NEXT EVAL after that —
  by the boundary on gt13.
- P-D dead (n was 1, never 0-with-pending); P-B dead (no callback
  ever pending at F4/F5 — the teardown had not LANDED, rather than
  landed-but-undrained).

## The remaining fork (method law — splitter built BEFORE the pin)

gt13 cannot distinguish WHERE (b)'s "next eval" is: the justifier
ITEM'S NEXT POP (salience 5's turn after RO2@7 — the parsimonious
sibling of the D-189 lazy-drop-at-pop law) vs a QUIESCENCE-time
forced consumption (a new landing site). Splitter = gt14: gt13 + an
obs_lk RO3 at salience 3 (strictly BELOW RJ). Pop-landing ⇒ RJ's
item pops at 5 before RO3's run ⇒ LK2 dies first ⇒ RO3's pending
activation is cancelled ⇒ **RO3 never fires**. Quiescence-landing ⇒
**RO3 fires once on LK2(2)** before the final drain. Both predict
RO3 never fires on LK1 (netted out mid-run). Prior: pop-landing
HIGH (sibling law parsimony). Oracle 3×.

## gt14 verdict (run after the above was written)

gt14 ×3: 3/3 identical (identity-normalized; TMS lines raw). Firing
sequence == gt13's — **RO3 NEVER FIRED** despite LK2(2) live and
believed (n=1) through RO2's whole run. Belief trajectory identical
to gt13 (LK1 dead by F3; LK2 zombie-justifier through F5; boundary
clean). **POP-LANDING CONFIRMED; quiescence-landing dead.**

⚖ THE LAW (pinned): **the amut update-break's dep-teardown lands at
the justifier's next NETWORK EVAL — (a) mid-run breaks: consumed at
the between-firings eval; the LK dies before the justifier's next
firing, its queued network Insert nets out, observers never see it;
(b) last-firing breaks: the staged delete waits while strictly-
higher-salience observers run (they fire on the zombie-justified
LK), and lands at the justifier item's NEXT POP — strictly-lower-
salience observers never fire on it.** Sibling of the D-189
lazy-drop-at-pop law (same landing site, different break source).

## Model port (same sitting)

model_sd.py amut=set_break routing: an EAGER justifier's last-firing
break now rides drops[] (pop-landing) instead of eager_pend[]
(loses-head landing); mid-run breaks keep eager_pend[] (the existing
loses-head machinery nets the LK out before the next firing — net
order already correct). Lazy routing untouched (no dump evidence).
Gates: validator 39/39 HELD; gt13 + gt14 shapes reproduce the oracle
exactly (RO2 ×2 in gt9 pairing order, RO3 silent, finals P-only);
populations seeds 7001/7002/6001/6003 + fresh 7004 → see below.

## Population verdicts + the COMPOSITE lane (x51; an overreach caught)

Edit-1 (pure lane only) populations: 7001 149/150, 7002 149/150,
6001 149/150, 6003 147/150 — **the lead-NL quartet RESOLVED as a
belief-staging cluster** (x17, x131, x128 clean; x90 DEMOTED to a
pure member-order residue: firing set now correct, only the
deleter-run P-order reversed — reclassified into the continuation-
order family). Survivors = exactly the mf-lazy-trail trio + nb-trail
tails + demoted x90. Fresh 7004: 146/150, 4 REAL — **all four
bisected PRE-EXISTING** (HEAD model output bit-identical on each):
x67/x131 order-class, x108 extra-continuation-firing, and **x51 a
FINALS divergence** = the same pop-landing law in the COMPOSITE lane
(breaks=True + amut=set_break, mutfirst): oracle has del_join@5 fire
×4 on the zombie-justified LK then delete every P; the old
loses-head landing killed the LK before the higher-salience run.

Composite extension, round 1 (NO mutfirst gate) — **FALSIFIED by
fuzz**: 7002 dropped 149→145 (x26/x58/x71/x95 over-fired observers/
deleters on a window the oracle does not show). All four regressors
are ILFIRST; x51 is MUTFIRST. Mechanism (the D-193 intervening-
action fold law's belief-layer sibling): the staged RHS pair drains
in RHS order — ilfirst lets the insert's not-break reach the tuple
FIRST (D-076 eager cascade at propagation ⇒ no window, even for
strictly-higher observers); mutfirst stages the update's join-break
first (lazy eval-consumption ⇒ the pop window). breaks=False shapes
have no race ⇒ the pure lane stays mutfirst-independent (gt13 was
mutfirst; base seeds already verified). Round 2 = the elif gated on
mutfirst: validator 39/39, x26/x58/x71/x95 back to certified
behavior, x51 exact, gt13/gt14 exact. Full five-seed populations
(v3) below.

## Final scoreboard (v3, mutfirst-gated model)

7001 149/150 (x103), 7002 149/150 (x68), 6001 149/150 (x90,
demoted), 6003 147/150 (x0/x41/x88), 7004 147/150 (x67/x108/x131,
all bisected pre-existing) = **741/750 (98.8%)**. Engine census
steady 47/56/45/54/55. **Every survivor is continuation/member-order
class; ZERO belief-staging divergences remain. The lead-NL cluster
is CLOSED; the composite window (x51 witness → banked as
gt15_composite_mutfirst.json) closed out-of-sample.** Gates at
close: make diff green (xfail drift 75/75 identical), lint
1716/0/0 (gt14+gt15 live), validator 39/39. Remaining open =
2 clusters: mf-lazy-trail continuation order (x68/x41/x88 + kin
x90/x67/x108/x131 — the justifier/deleter POST-CHURN continuation
order) and nb-trail tails (x103/x0) — both named for one SdDump
run each in the D-194 handoff (§clusters 2-3); the TmsDump lens is
NOT their instrument (order-class, not belief-class).
