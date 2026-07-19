# PINS — the CEP residuals round (D-342; Bryan: "do the ND/NE
# unblock-landing round" — the ledger item resolved to the three
# xf_cep_* witnesses; the actual D-317 item was closed by D-321/322)

2026-07-19. Witness re-measurement (3x, all stable, shapes match
the bank): xf_cep_not_mid_release_join_order ORDER (engine pairs
E0@0 with E2@-45 first, oracle E2@-3 first);
xf_cep_not_chain_heaptie ORDER (engine E0@4 first, oracle E0@3);
xf_cep_c_del_churn_exists_rule VALUE (2 vs 3 firings).

## Witness 1: not_mid_release_join_order — the decode

Rule: $a:E0() not E1(before[0,100] $a) $c:E2(before[0,50] $a).
E0@5 is E1@1-blocked forever; E0@0 is never E1-blocked but its
temporal not defers the unblock to the WINDOW CLOSE (D-134 §3B);
advance(1000) closes it; the released left joins both E2s
(-3, -45 ∈ [-50, 0]). FIRING SET 0-div; the fork is pair ORDER.

The banked D-134 comment names the engine mechanism: the
pop-time do_join_node release scans partners DESCENDING-TS
(D-101) then PREPENDS into staging — ONE net flip → -45 first.
The oracle's measured -3-first implies Drools' release-time
propagation reaches the terminal WITHOUT the net flip (direct
per-emission creation order = scan order; FIFO terminal).

CONFOUND in the banked witness: arrival order [-3,-45] and
descending-ts [-3,-45] COINCIDE. The grid splits them.

## The grid (predictions REGISTERED before any cell runs;
## oracle 3x per cell)

- nm1_arrival_split: base facts insert E2@-45 BEFORE E2@-3
  (arrival [-45,-3]; descending-ts [-3,-45]). PREDICT (med-high,
  scan = descending-ts per the D-101 certified node law, direct
  emission): oracle fires -3 FIRST — same as the banked witness,
  insertion-order-invariant. Engine: -45 first (desc scan + the
  prepend flip; also insertion-invariant). A -45-first oracle
  here instead means scan = ARRIVAL (D-101 doesn't govern the
  release walk) — then the banked witness's fork is the flip
  alone with arrival scan.
- nm2_three_partners: E2@-3, E2@-45, E2@-20 (that insertion
  order). PREDICT (med-high): oracle descending [-3,-20,-45];
  engine the FULL REVERSAL [-45,-20,-3] (one batch flip, not an
  adjacent swap). Under the arrival-scan alternative: oracle
  [-3,-45,-20].
- nm3_ctl_blocked: E1@-2 added (blocks E0@0: 0-(-2)=2 ∈ [0,100]).
  PREDICT (high): NO release firings either side (the blocked
  left never releases; E0@5 stays blocked too) — the control
  that the release lane is what we are measuring.
- nm4_two_lefts: E0@0 and E0@-10 both released (no blockers for
  either; E2@-3 ∈ [-50,0] pairs E0@0; for E0@-10: E2 ∈ [-60,-10]
  → -45 pairs; -3 does NOT). Each left pairs exactly one E2 —
  pins the PER-LEFT release grouping/order without the
  within-left question (close times differ: 0+100 vs -10+100).
  PREDICT (med): both sides fire (E0@-10,-45) then (E0@0,-3)?
  — close-time order (E0@-10 closes at 90 < 100)... register:
  oracle close-time ascending; engine SAME (the §3B deferral
  release machinery is close-time ordered, certified); the cell
  is acontrol for per-left order. Low confidence on tie details;
  record what lands.

## nm grid MEASUREMENTS (2026-07-19, oracle 3x stable each)

- nm1: oracle [(0,-45),(0,-3)] — **the med-high descending-ts
  prediction MISSED (recorded); the registered alternative HIT**:
  the release walk scans right partners in ARRIVAL (memory)
  order, insertion-sensitive. Engine [(0,-3),(0,-45)] =
  reverse-arrival (also insertion-sensitive — the D-134 §6
  comment's "descending-ts" attribution is STALE on both sides).
- nm2: oracle [(0,-3),(0,-45),(0,-20)] = ARRIVAL exactly
  (descending would interleave -20); engine [(0,-20),(0,-45),
  (0,-3)] = the FULL flat reversal.
- nm3 ctl: no firings either side (hit).
- nm4 (cell-design slip recorded: E0@0 pairs BOTH E2s, -45 ∈
  [-50,0]): oracle [(0,-3),(0,-45),(-10,-45)] — per-left order =
  DESCENDING CLOSE-TIME (E0@0 closes 100 > 90), the certified
  §3B model order, then within-left arrival. Engine
  [(-10,-45),(0,-45),(0,-3)] = the flat reversal of the oracle's
  entire release batch.

# THE LAW (witness 1, D-342 round 1)

**The oracle's §3B deferral release reaches the agenda DIRECT:
per-left in the certified target order (descending close-time,
then creation), each left's downstream-join pairs in right-
memory ARRIVAL order — the flat sequence T×R. The engine's
batched release compounds ONE uncompensated staging flip at the
mid-join hop: the agenda gets the flat REVERSAL of T×R.** The
existing drain_pending_fires compensation ("push reverse(T)")
is tuned for the not-TERMINAL shape (k=0 join hops, certified
by model_not_infer); every mid-join shape (k=1) lands reversed.

## The port design (GATED — its own slab, model-first)

Composition analysis (all three measured cells + the k=0
certified lane compose exactly):
- k=0: push reverse(T) → not-trg prepend → terminal head-first
  = T ✓ (certified, must not move).
- k=1 today: trg T → join walk head-first → emissions T×R →
  terminal prepend → head-first = flat reverse ✗.
Candidate mechanism (correct-by-construction for per-left order
at ANY k, and within-left at k≤1): SINGLETON release injection
(drain_pending_fires feeds ONE due deferral per eval rescan in
T order — kills the batch compensation and the parity math) +
the release eval's TERMINAL walk consumes its staged batch
.rev() (identity for k=0 singletons — the certified lane is
byte-safe by construction; = arrival for the k=1 within-left
pair batch). Blast surface: eval COUNTS change (D-333 laziness,
agenda_open, dirty/queued churn) — model_not_infer/defer must
be extended with the mid-join hop and re-validated BEFORE the
engine port; counter-set = the pr_cep_not_* graduates, D-140/
151/158 unblock shadows, D-102 held drains, the TJ corpus.
k>=2 chains are UNMEASURED (add an nm5 probe in the port slab).

## Witnesses 2 + 3 (this round's scope pass)

- xf_cep_not_chain_heaptie RE-MEASURED: oracle
  [(3,131),(4,131),(4,106),(3,106)] — the 231-close tie fires
  3-then-4 while the 206-close tie fires 4-then-3 IN ONE RUN.
  The D-134 "internally inconsistent" claim re-confirms verbatim
  on today's oracle: no per-tie-local deterministic rule exists;
  only whole-scheduler java.util.PriorityQueue heap emulation
  (offer/poll history) could reproduce it. Post-D-323/328
  doctrine distinction recorded: those emulated DOCUMENTED Java
  contracts (List.remove, toString); PQ equal-element order is
  contractually UNSPECIFIED — the fz_42_84 identity-hash class.
  RULING STANDS: stays banked, accepted-undefined. (Engine is
  internally CONSISTENT: 4-then-3 at both ties.)
- xf_cep_c_del_churn_exists_rule RE-MEASURED: oracle NE,CH,NE vs
  engine NE,CH — byte-for-byte the D-138 record. The law was
  fully decoded then (an explicit event-witness delete at an
  exists/not force-evaluates at DELETE-time; the RHS variant
  needs that eval RE-ENTRANTLY, inside execute_rhs, so the
  delete's child-teardown lands before the same-RHS reinsert
  re-blocks). No new probing needed; the port (re-entrant-safe
  delete-time eval) is GATED — its own slab.

## Round disposition

Witness 1: law PINNED this round (port gated, model-first,
design above). Witness 2: closed as accepted-undefined (ruling
re-confirmed). Witness 3: law complete since D-138 (port gated).
LEDGER CORRECTION recorded: the "expiration ND/NE unblock-
landing round (3 witnesses banked)" ledger line was STALE — the
D-317 item was closed by D-321 (the law) + D-322 (the port,
nine pr_ndne_* graduations); these three cep witnesses are
older D-134/D-138 classes and were mislabeled in the ledger.

# D-343: THE PORT SLAB (Bryan: "do the not_mid release-order port")

## Model validation (the D-342 design's step 1)

model_not_infer fuzz 400 seed 343001: 2 divergences, BOTH
chain_not/ORDER = the PQ-tie class (two tuples sharing the not
anchor, cross-left same-close-time). **not_mid: ZERO** — the
model's (-ft, crt) sort IS the oracle spec (crt = D-125 creation
= within-left right-arrival). The D-131 fence conflated the two
classes; only chain_not's cross-tuple tie is undefined.

## nm5 (k=2 hops) — predictions REGISTERED before the cell runs

Shape: $a:E0() not E1(before[0,100] $a) $c:E2(before[0,50] $a)
$d:E3(before[0,30] $a); E0@0 released at the advance; two E2s
(arrival -3, -45), two E3s (arrival -7, -25); all in-window.
Candidate compositions for the flat agenda order:
- H1/H2 (mid hops flip, terminal does not — equivalently
  reverse-scan everywhere + all hops flip): [c2-batch first]:
  (0,-45,-7),(0,-45,-25),(0,-3,-7),(0,-3,-25).
- H3 (pure DFS, no flips anywhere): (0,-3,-7),(0,-3,-25),
  (0,-45,-7),(0,-45,-25).
PREDICT H1/H2 (med): Phreak stages mid hops uniformly even in
the release lane; k=1's measured arrival order is the terminal
hop's difference (either the activation-creation order or a
reverse-scan — indistinguishable at k=1, and equivalent at all
k). H1/H2 ⟹ the D-342 port design (singleton injection +
release-eval terminal .rev()) is EXACT for all k.

## nm5 MEASUREMENT: BOTH candidates MISSED (recorded) — the miss
## decodes the TRUE law

Oracle 3x: [(0,-3,-7),(0,-45,-7),(0,-3,-25),(0,-45,-25)] —
neither H1/H2 (c2-major) nor H3 (DFS). It is the D-125
PER-ARRIVAL CREATION ORDER: d1's arrival pairs the held lefts
in memory order (c1,c2), then d2's. Engine:
[(0,-3,-25),(0,-3,-7),(0,-45,-25),(0,-45,-7)] = the uniform
per-hop staged composition.

# THE LAW (revised, final): §3B is a FIRING deferral — Drools
# propagates the not's child EAGERLY (downstream tuples and
# activations materialize in the ordinary D-125 per-arrival
# creation order); the window close RELEASES the held
# activations in their creation order, anchors ordered by the
# certified advance-batch rule (descending close-time, then
# creation). No re-propagation happens at release. All five nm
# cells + the banked witness compose exactly. Flat key check:
# sort by the tuple's fid-set DESCENDING, lex ASCENDING ==
# creation order on the whole insert-only population (nm1
# (3,2)<(4,2); nm4 (3,2)<(4,2)<(5,4); nm5 (3,1,0)<(3,2,0)<
# (4,1,0)<(4,2,0)) — the do_texists max-FactId reconstruction
# generalized. The singleton+.rev() D-342 design is DEAD (it
# implements H1/H2); the probe killed it before the port.

## The port design (revised): ONE SORT at the terminal
- drain_pending_fires keeps its batch + push compensation
  UNTOUCHED (the k=0 certified lane composes as today) and
  additionally records a RELEASE RANK per released left in T
  order (desc close-time, then creation).
- The release eval's TERMINAL batch (not_releasing) sorts by
  (anchor-prefix rank, fid-desc-lex) — for k=0 the batch is the
  anchor tuples themselves in T order already (stable sort ⇒
  byte-identical); for k>=1 it lands the creation order.
- Conservative arm: tuples with no ranked prefix keep position
  (mixed batches unsorted unless fully ranked).
- Churn caveat recorded: the fid-lex key equals the replay only
  for insert-only downstream histories (updates re-keying
  memories could split them) — the fresh not_mid fuzz axis and
  the byte gate patrol it.

## nm6 (the last confound): anchor insertion order vs close-time
E0@-10 inserted BEFORE E0@0 (fids swap; close 90 < 100). PREDICT
(high, the certified advance-batch model): oracle fires the
E0@0 group FIRST (descending close-time beats creation/fid
order): [(0,-3),(0,-45),(-10,-45)]. A creation-order-only oracle
would fire (-10,-45) first.

## nm6 MEASUREMENT: the high-confidence desc-close prediction
## MISSED (recorded) — the law simplifies further

Oracle 3x: [(0,-3),(-10,-45),(0,-45)] — PURE CREATION ORDER,
INTERLEAVED across anchors (desc-close would group the a0 pair).
Counterfactual replay: (a0,-3) born at E2@-3's arrival (fid4);
E2@-45's arrival (fid5) scans j1 left memory [a-10, a0] →
(a-10,-45) then (a0,-45). Exact. Engine
[(-10,-45),(0,-45),(0,-3)] = today's batched re-propagation.

# THE LAW (FINAL, all 6 nm cells + the banked witness):
# a not-MID release fires held downstream activations in PURE
# D-125 creation order (fid-desc-lex over the tuple's elements;
# within one completing arrival, left-memory scan order),
# ft-independent and anchor-interleaved. The k=0 lane (not as
# the LAST CE: not_partner, chain_not) keeps the certified
# arc-A order (descending close-time then creation) — measured
# separately in D-134 and untouched here; the two lanes differ
# in Drools because k=0 activations are only born AT the close
# (one hop) while not-mid tuples pre-exist with creation seats.

## The port (final design): for a NOT-MID-CLASS rule (temporal
## not followed by >=1 positive pattern — a static per-rule
## property), the release eval's terminal ins-batch sorts by
## fid-desc-lex. k=0 rules never sort (arc-A + heaptie safe).

## D-343 port receipts

Model validation: model_not_infer 400 @ 343001 (2 div, both
chain_not PQ-tie) + 200 @ 343002 (0 div) — not_mid model==oracle
throughout. THE PORT: one sort at the terminal consume
(engine.rs, evaluate_rule_inner): release evals (`releasing`) of
not-MID-class rules (temporal Not followed by a Positive — a
static pattern check) sort the ins batch by fid-desc-lex before
consume_term_ins. k=0 rules never enter the arm (arc-A +
heaptie byte-safe); self-join equal-multiset ties keep batch
order (stable sort, recorded caveat); churn caveat stands (the
key equals the replay on insert-only downstream histories).

POST-PORT: all 6 nm cells + the banked witness MATCH the oracle
(diff 7/7 incl. nm3 inert). Byte gate vs pre-port HEAD:
2506/2510 — the 4 diffs are EXACTLY nm1/nm2/nm4 + the witness
(nm5/nm6 post-commit-authored, nm3 inert-identical); ZERO
certified movers; the heaptie bank byte-identical. GRADUATIONS
(7): pr_nm_cep_not_mid_release (the witness) + pr_nm_nm1..nm6.
Bank 18→17→18 (one NEW pre-existing quarantine: fz_342002_1206,
TMS × setFocus × insertLogical, plain facts — bisected
byte-identical pre-port; seed re-run clean). Battery: make diff
11/1526/414 + drift 18 identical; lint 2385/0/0; cargo 74;
pytest 260; demo True; SD census 71 EXACT; agenda_open x10
identical x3; model_ird 31/31; IRD 0-div x5; fuzz 2x2000 seeds
342001/342002 + cep 3x300 seeds 342901-903 CLEAN; NEXT seeds
344001+. Arc scorecard (D-342+343): 13 predictions, 10 hits, 3
recorded misses — and BOTH port-shaping misses (nm5 killing the
singleton+.rev() design, nm6 killing desc-close grouping) were
the decisive measurements.
