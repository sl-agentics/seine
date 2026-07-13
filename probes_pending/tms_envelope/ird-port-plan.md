# THE I-RD PORT PLAN (predictions first; Bryan-gated open, 2026-07-12)

Baseline: engine-vs-oracle 86/750 on the ird census (12/14/21/25/14);
SD census 72; drift bank 59. Target: the model_ird laws translated,
red-first per family, full receipts per family, graduation at the end.

## Family F1 — the static key model

ENGINE CHANGES (minimal-touch):
1. Topology (⚖ activation-backfill): tms gains `activated` +
   `pre_stated` + `orphans` (+ `unstage_born` for F2). tms_note_stated
   pre-activation → pre_stated only (NO key, NO by_fact — a
   pre-activation singleton key is observationally identical to
   keyless); tms_activate (at the first tms_insert_logical) backfills
   LAST-per-value into the mapped entry; post-activation notes join
   the mapped entry (current behavior).
2. The r1-event in tms_route_delete_ex: an ORPHAN delete no-ops (new
   top check); an rhs stated-delete of a pending-mixed key (pending
   Some AND beliefs non-empty) → kill the named handle, ORPHAN the
   remaining stateds, take pending_vals (unstage), the KEY DIES.
   Externals (rhs=false) keep the old path (dump8: no
   materialization). The mis-scoped `beliefs.clear()` is scoped to
   the key-dying (stated-empty) case.
3. The L6-event + pending-clear in tms_drop_act_deps ONLY (the
   9700 refire-supersede epilogue stays UNTOUCHED — corpus-pinned
   refire shapes, unreachable in the ird envelope): beliefs-empty →
   justified: retract + ORPHAN stateds + key dies; pending:
   pending=None, key survives (pure stated); neither: the existing
   fz_42_1395 removal.
4. tms_materialize: the unstaged handle is fully TMS-DROPPED (no
   entry update when the key died, no stale by_fact) — matches the
   oracle dumps (@5 leaves the map); mark unstage_born (F2's flag).
5. The c5/justified-delete path and the dump3 had_justified no-op:
   UNTOUCHED (c5 converges today via that approximation).

RED-FIRST targets (engine≠oracle now, must converge): cells d1, d2,
r1, l6; witnesses fz_7_9902, fz_7_8757. BOUNDARIES (must not move):
b1's kill-count (RD×2 — the r1-event's 0-sibling case), b2, x1, l5
(same observables via orphans instead of had_justified survivors),
c2-c5, l1-l4, r2, a1/a2/c1, m0, all m/s cells, the 39 sd cells, the
FULL corpus byte-gate (esp. tms_w1/w5 multiplicity, tms_e6, the
dump pins, pr_tms_t20*).

PRE-REGISTERED RISKS: (a) scoping the external-delete clear to
key-dying changes external multi-stated-pending behavior — unpinned;
make diff arbitrates, fallback = rhs-scoped clear only; (b) tms_w1/w5
under the keyless pre-activation encoding — byte-gate arbitrates;
(c) the SD census must hold 72 EXACT (LK stateds are rare in that
grammar but not absent).

EXPECTED CENSUS MOVEMENT: the ird 86 drops by the F1 share (d1/d2
class + rebirth/orphan class); SD 72 EXACT.

## Family F2 — the dynamic law (act survival)

The unstage-born handle's delete must not cancel queued acts (they
fire later with the dead handle's values). Site recon after F1
(next_activation's per-rule queues; the cancel is either at delete
propagation into the queue or a pop-time liveness check — the fix
is the unstage_born exemption at that site). ⚠ the stale-value fire
hazard: firing a dead handle's act reads store values — is_alive
gating exists throughout the eval path; red-first will surface any
panic (the census is the net). RED-FIRST: b1, b2; witnesses
fz_7_4048, fz_123_7219, fz_42_6368 (the surviving-act deltas).
BOUNDARIES: everything else — the exemption keys on unstage_born
handles ONLY, a set that is EMPTY in every non-ird corpus shape.

## Family F3 — the in-flush self-break landing

Narrow tms_eager_break's current_act exclusion: lazy ONLY when the
justifying rule's LHS has ≥2 positive patterns on the broken fact's
type (⚖ rule-shape, s2); single-pattern same-batch self-breaks land
eagerly like foreign ones. RED-FIRST: m1, m2, m5; witnesses
fz_777_2956, fz_7_1591, fz_7_5988. BOUNDARIES: m3/m6/m7 (self-join
stays lazy), s1/s2/s3, fz_42_2442 (regressions, byte-gate),
pr_tms_t20d + pr_tms_selfbreak_flush (named in the engine comment
as certified), m4 (foreign eager — already right).

## Receipts + graduation protocol (per family, then final)

Per family: red-first capture → implement → target cells converge →
boundaries hold → make diff (corpus byte-gate; the xfail drift gate
will flag EXACTLY the family's witnesses — verify the moved set
matches the family map, then xfail-rebank with the D-entry note) →
lint/cells/witnesses/model → agenda_open ×19 → SD + ird censuses
(background; SD 72 EXACT every family) → commit.
FINAL: triage_xfail (10× oracle stability) → graduate every PASS
(git mv to regressions + rebank + D-entry), re-run the ird census
for the closing number, the port-slab summary entry.
