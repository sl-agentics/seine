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
