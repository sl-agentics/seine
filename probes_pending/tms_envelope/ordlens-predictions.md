# Order-cluster dumps (gt16/gt17) — PREDICTIONS, logged pre-run

_2026-07-12, the D-195 follow-on (Bryan: "the two order clusters, one
SdDump run each, then 0-div"). Written BEFORE either dump ran. Cores:
gt16 = sdp7002x68 verbatim (J lazy trail set_break mutfirst
breaks=True @-5 decl-first; D no-loop @-5; P×4), gt17 = sdp7001x103
verbatim (J lazy trail set_break ILFIRST breaks=FALSE @-5; obs_join
@5; D no-loop @-5 decl-last; P×3). Banked divergences:_

- gt16: oracle [J1, D4, **J3**, D1, D2, D3] (J fires twice, SKIPS P2
  forever; D tail ascending) vs model [J1, D4, J2, D1, J3, D3, D2].
- gt17: 12 identical firings, then D tail — oracle **[D1, D2, D3]**
  (ascending) vs model [D3, D2, D1].

## Cluster-wide observations driving the readings (from the 9-case
## census, before any dump)

- x88 (amut=del nb, obs_join@7): the oracle observer GLIMPSES each
  generation — gt12's "del = eager dep-cascade" was read from a
  BELOW-salience observer geometry, so eager-cascade vs pop-landing
  were confounded there; the D-195 eval-consumption landing likely
  governs the del lane too (window for strictly-higher only).
- x90 (eager J, D@-5): D runs [3,2,1] = raw phys scan; x131 (eager,
  P×2): D runs [1,2] = one reversal. Parity fits the D-195 split:
  mid-run LKs NET OUT (no fold), only the LAST LK folds ⇒ exactly
  one reversal... x90 contradicts naive parity (needs [1,2,3]) —
  UNLESS the eager-lead fold doesn't reach D's group at all (x90 J
  is LEAD+mutfirst ⇒ churns=False in the current model — yet the
  model got [1,2,3] via D's t0 staging while the oracle scanned
  phys). The t0-staging-vs-phys question is a dump observable.

## Pre-registered readings

**gt17 (the cleaner cell — no breaks, no folds on D's node):**
- P-17: D consumes its t0 staged-insert list FIFO ⇒ [1,2,3]. The
  model's miss is the pending_fold PROXY ("unshared lazy set_break J
  churns any del_not group") cross-contaminating a group the J does
  not share a node with (the set_break alpha broke sharing). Predict
  the dump shows D's staged lists intact/FIFO through all three
  generations and NO reversal of its phys. FALSIFIER: D's staging
  shows replacement or the phys reverses ⇒ the churn is real and the
  model's error is elsewhere (then the t0-owner law itself is in
  question on this shape).
- P-17-obs: per-generation obs pairing [1,2,3]/[2,1,3]/[3,2,1]
  re-verifies gt9 relocation — no new claim.

**gt16 (breaks=True composite, lazy):**
- P-16-D: same as P-17 — D's tail [1,2,3] = staged-FIFO/insertion
  order surviving; the model's [3,2] tail inversion is fold
  contamination again.
- P-16-J (THE OPEN QUESTION — J fires 3-not-2 after the first fold,
  then NEVER fires P2): candidate readings, no pin —
  (i) staged-without-dirty (the banked t10-leak analog): the
  unbreak's re-add for J's P2 tuple lands in J's staging but J's
  item is NOT re-queued; D's tuples (independent staging) fire and
  delete P2 before any J eval consumes it. Predict: J's PATH staged
  lists show the P2 re-add sitting unconsumed at D's firings.
  (ii) the re-add order after the fold is a phys head→tail scan of
  survivors ([3,2] after P4's delete and P1's alpha-break) and J's
  whole-continuation consume takes P3 ONLY (single-tuple consume),
  the P2 re-add belonging to the NEXT unbreak — which D's deletes
  starve. (iii) queue-tie inversion (J@-5 decl-first loses the head
  to D) — heavily counter-certified; if the dump shows this, the
  queue-head discipline needs a caveat row.
  Weight: (i) ≥ (ii) ≫ (iii).

## Discipline

Oracle 3× per core (3 JVM launches, identity-normalized diff; TMS
lines raw). Predictions for the ENCODING phase: the fix is expected
to REMOVE a proxy (the del_not-existence churn condition), not add
one (epicycle stop). Any encoding validated by: validator 39/39,
gt13/gt14/gt16/gt17 exact, all five seeds + fresh 7005 → 0-div.

## gt18 addendum (pre-run; the 7006x34 corner)

x7006x34 = x131's eager-lead-ilfirst composite shape at THREE facts:
oracle D-tail [1,2,3], the per-landing fold encoding predicts [3,2,1]
(three folds, odd parity). Banked as gt18_eager3_ilfirst. Candidate
readings, no pin: (i) the eager mid-run ins+del cycles NET OUT in
D's staging when D never evaluates between them (⇒ no folds at all;
D consumes t0 order — but gt16's D processed its fold under a
seemingly identical no-eval-between window, so (i) needs a
lazy-vs-eager staging difference to be real); (ii) all three cycles
process at D's one eval and the RTN-level dedup nets the
intermediate activations (⇒ effective LAST-cycle re-add scan
[1,2,3] with ltm at [1,2,3] post-odd-reversal... the ltm parity in
the dump decides); (iii) neither. The dump reads: D's ltm order at
its first firing, the PATH staged lists, and (via ×3) stability.

## gt19 addendum (pre-run; the 7002x56 corner + the del-composite pair)

The v6 gate (8 seeds): 1197/1200. x7007x79/x98 = the del+breaks
eager+mutfirst composite missing the D-195 last-key pop re-route
(obs_lk@10 glimpses the final generation's LK once) — mechanical
extension, no new law. x7002x56 (eager ilfirst TRAIL composite @10,
del_not@0, 1 generation, P×2): oracle D-tail [2,1]; the eager-no-fold
encoding predicts t0 [1,2]. Banked as gt19. This exposes the
deleter-ACTIVATION-ORDER source dimension: witnesses now span
memory-scan-like [2,1] (x56), update-order [1,2,3] (gt17/gt18),
fold-scan [3,2,1]/[4,1,2,3] (x90/gt16). Candidate readings, no pin:
(i) D's t0 staging is consumed at CYCLE-START (its ltm is live
before any firing — gt16 F0 showed this) and x56's D memory-scans
phys [2,1] because the single ilfirst cycle nets at D and only P1's
update re-adds — order [old-P2, refreshed-P1]; gt17/gt18 then read
as ALL-updated ⇒ update-order; but x90 [3,2,1] (all updated,
mutfirst) contradicts pure update-order ⇒ (ii) the surviving staged
ops' interleave with the fold decides, and only the dump can say.
Read: D's ltm order + PATH staged-left lists at each firing.
