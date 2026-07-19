# PINS — D-344: the del_churn re-entrant eval port (Bryan: "do
# the del_churn re-entrant eval port"; the D-138 fenced RHS
# variant, witness xf_cep_c_del_churn_exists_rule)

2026-07-19. The law is D-138's, complete: an explicit delete of
an EVENT witness at an exists/not node evaluates at DELETE-TIME
(the del's 0-support crossing lands before the reinsert), where
the engine's RHS delete defers to the fire drain and the exists
node's ins-before-del phase order never crosses zero →
coalesce (NE,CH vs oracle NE,CH,NE). The port = mirror the
delete_fact D-138 force-eval block into execute_rhs's Delete
arm (same scope: event-typed victim, rules with a not/exists CE
over the victim's type, in_stream_flush saved/set/restored).
RHS context is never the expiration drain.

## The RHS grid (predictions REGISTERED before any cell runs;
## oracle 3x; mirrors D-138's external discriminators)

- rc1 = the banked witness (del-then-ins): oracle NE,CH,NE
  (already 3x-measured this session); engine pre-port NE,CH.
- rc2_ins_first (RHS insert E0"y" BEFORE delete E0"z"):
  support never crosses 0 (1→2→1). PREDICT (high): NE,CH both
  sides — coalesce is CORRECT here (D-138's external ins-first
  control held the same).
- rc3_not_ce (NN: not E0() P(); base has E0"z" → NN silent;
  CH churns): the del's release creates NN's activation
  mid-CH-fire, the reinsert re-blocks and CANCELS it before it
  can fire (CH still executing). PREDICT (med-high): CH only,
  both sides (engine pre-port coalesces to the same visible
  outcome — a control that the port must NOT break).
- rc4_del_only (RHS deletes E0"z", no reinsert): support 0
  stays; NE already fired; no new activation. PREDICT (high):
  NE,CH both sides.
- rc5_two_witnesses (base has E0"z" AND E0(30,"w")): del z =
  2→1, ins y = 1→2 — no crossing. PREDICT (high): NE,CH both
  sides (the exists edge never fires).

## Grid MEASUREMENTS + THE PORT (2026-07-19)

ALL FOUR grid predictions HIT (oracle 3x each): rc2 NE,CH / rc3
CH / rc4 NE,CH / rc5 NE,CH — engine pre-port already correct on
every control; the fork confined to the del-then-ins 0-crossing
(rc1, the witness). THE PORT: the D-138 force-eval block
mirrored into execute_rhs's Delete arm (event-typed victim,
rules with a not/exists CE over the victim's type,
in_stream_flush saved/set/restored; RHS is never the expiration
drain; no exclusion of the firing rule — mirrors the external
scope exactly). The witness flips to NE,CH,NE; grid 5/5 PASS.

## D-344 receipts

Byte gate vs pre-port HEAD: 2512/2513 — the ONE diff is the
witness itself; zero certified movers. FIVE graduations:
pr_dc_cep_c_del_churn_exists_rule + pr_dc_rc2..rc5; bank 18→17.
Battery: make diff 11/1531/414 + drift 17 identical; lint
2390/0/0; cargo 74; pytest 260; demo True; SD census 71 EXACT;
agenda_open x10 identical x3; model_ird 31/31; IRD 0-div x5;
fuzz 2x2000 seeds 344001/344002 + cep 3x300 seeds 344901-903
ALL CLEAN; NEXT seeds 345001+. The D-138 arc is now FULLY
CLOSED (external D-138 + RHS D-344); the xf_cep_* residual set
is reduced to the heaptie alone (accepted-undefined).
