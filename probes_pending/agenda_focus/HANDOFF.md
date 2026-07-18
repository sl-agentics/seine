# HANDOFF — the setFocus × not-CE staged-flush lane: fine-structure
# grid, then THE PORT (cold start)

Written 2026-07-18 at the end of the D-311..D-318 session. Bryan's
directive for this slab: **"do the fine-structure probe grid, then
the port"** — the port gate is PRE-CLEARED, CONTINGENT on the grid
landing a coherent law. If the grid comes back incoherent or
oracle-nondeterministic, STOP AND REPORT instead of porting.

## THE LAW AS MEASURED (D-318 — read probes_pending/agenda_focus/
## PINS.md first, it is the full hunt record)

When a firing's RHS calls `drools.setFocus(G)` and G's rules contain
NOT-CE networks that receive the same RHS's staged inserts, the
ORACLE's focused-group evaluation FLUSHES staged propagation — fresh
higher-salience MAIN activations become visible to the next pick and
PREEMPT the current rule's remaining activation run. The ENGINE
always continues the run (fires all of the current rule's
activations first). Where the group has no not-CE over the staged
types (alpha-only, plain join, type-blind, dead, no-focus, no fresh
activation), BOTH engines continue — that agree-boundary is
graduated as pr_af_s1_ctl/s2_focus/s3_logical/s4_grouphit/
s5_nofresh/s7/s8/s11 (these 8 cells MUST stay byte-identical through
the port).

Fine structure, UNMAPPED (the grid's job): the group-not form
interleaves FULLY (xf_af_s9_groupnot: L,H,L,H,L,H); the simple-not
form PARTIALLY (xf_af_s10_simplenot: L,H,L,L,H,H — the flush
happens on the FIRST insert-evaluation, then stops: phreak
segment-linking territory). Canonical witnesses:
scenarios/xfail/xf_af_s9_groupnot.json, xf_af_s10_simplenot.json,
xf_af_min1681.json, plus the 4 original fuzz witnesses
(fz_313002_319, fz_315901_311, fz_316001_1681, fz_316002_1902 — all
setFocus-ablation-verified members).

## PHASE 1 — THE FINE-STRUCTURE GRID (predictions first in PINS.md,
## 3× stability, all cells diffable; divergent cells are EXPECTED —
## they map the oracle's law, and they graduate AFTER the port)

Axes to grid (build on the s9/s10 shapes — L sal -5 pushes "g" and
inserts X per firing; H sal 5 on X; GD in "g" holds the not-CE):

- **g1 scale**: 2/3/4/5 L-seeds on the s10 shape — does the partial
  interleave stay "one flush then none" (L,H,L,L,L,H,H,H) or
  something periodic? Names WHICH evaluation flushes (first-link
  only?).
- **g2 repeat-push**: pre-push "g" via a one-shot highest-salience
  rule BEFORE any L fires (D-106 relocate-or-push means later
  pushes RELOCATE) — does the flush still trigger per L firing, or
  only when the push actually changes the stack top?
- **g3 group fires**: the not-CE rule in "g" MATCHES (unblocked) —
  order of GD vs H vs remaining Ls (s4 covered the join-free
  matching case; this is the not-CE matching case).
- **g4 two fresh highs**: L's insert activates H1 (sal 5) and H2
  (sal 8) — does the preemption drain ALL higher activations
  (H2,H1) before returning to L, or one per flush?
- **g5 exists dual**: exists-CE instead of not in "g" — does exists
  flush too? (If YES, note for fuzz_cep: the D-317 `exists`-only
  observer fence for the ND/NE lane may need revisiting — but that
  fence is for a DIFFERENT, expiration-driven lane; do not touch it
  in this slab.)
- **g6 not-over-join**: the not wraps a join (`not(S(...) and
  X(...))` group form vs `not S(k == $v)` beta-correlated) — which
  network shapes flush?
- **g7 salience ties**: fresh H at the SAME salience as L — decl
  order vs continue (the pick law at equality).
- **g8 second rule same group**: two rules in "g", one not-CE one
  alpha — does the alpha rule's presence change the flush?
- **g9 mid-run push change**: L1 pushes "g", L2 pushes "g2" (both
  not-CE groups) — stack evolution × flush.

Add axes the first results demand (the s10 partial WILL suggest
follow-ups). The minimizer pattern from D-318 is in the session
record: greedy rule/fact/epoch deletion with a SEMANTIC-divergence
predicate (`'FAIL' in out and 'errored' not in out`) — rebuild it in
the job tmp dir if needed.

## PHASE 2 — THE PORT (contingent: coherent law only)

Engine map (verify each before editing — cold-session rule):
- The engine's pick: the firing loop continues the current rule's
  activation run (the D-258/D-259 "late-continue" lane — read those
  DECISIONS entries). The fix shape: when `focus_changed` was set
  during the RHS (engine.rs, CompiledAction::SetFocus sets
  `self.focus_changed = true` — the D-106 relocate-or-push site) AND
  the grid's flush condition holds (not-CE networks over staged
  types in the pushed group), force the staged flush + a full
  agenda re-pick instead of continuing.
- The flush machinery already exists: `stream_flush` / the staged
  propagation the RHS insert path uses (engine.rs ~10750 region,
  `stage_snapshot` + `on_insert` + `stream_flush`).
- The grid decides the CONDITION's precision (s10 says the flush is
  stateful — possibly "only when the group's segment first links").
  DO NOT over-model: if the grid shows a simple condition ("any
  not-CE over a staged type in the focused group → flush once per
  firing"), port that; epicycle rule applies (a condition needing a
  proxy variable isn't the mechanism).
- Walls NOT to touch: the D-089 group-CE justifier wall, the ?query
  justifier fence, the D-317 fuzz_cep exists-only fence (different
  lane: the expiration ND/NE unblock-landing split — still an open
  probe item, NOT this slab).

PORT GATES (the order-sensitive battery is the whole point):
- SD census 72 EXACT ×12 seeds (6,10,3,5,6,5,5,6,8,7,4,7) — THE
  order gate; a port that moves ANY cell fails.
- agenda_open ×15 byte-identical, both binaries + pre-edit worktree.
- Byte gate vs the pre-edit worktree: EXPECTED divergence = exactly
  the 7 xfailed witness cells (xf_af_s9/s10/min1681 + the 4 fz_*
  members) flipping toward the oracle; the 8 pr_af_* boundary cells
  and everything else byte-identical. After the port: diff the
  witnesses 3×, un-xfail the ones that PASS (bank DOWN from 63 via
  make xfail-rebank), graduate the canonicals to scenarios/probes/.
- Full battery otherwise: make diff (from 11/1315/414 + drift 63),
  lint (2149), cargo (13 suites), maturin FROM bindings/ + pytest
  (257), demo True, model_ird 31/31 (cd probes_pending/tms_envelope
  in a SUBSHELL — a stuck cwd once wrote DECISIONS.md there), IRD
  0-div ×5 (7001/7002/6001/6003/9001, tools/fuzz_tms_ird.py 150 N),
  SD (tools/fuzz_tms_sd.py 150 N ×12 seeds, timeout 900 each, never
  rebuild mid-census), fresh fuzz 2×2000 seeds 318001/318002
  (./target/debug/seine-harness fuzz 2000 N; finds → bisect vs the
  pre-edit worktree via engine-output compare — but note ORDER-lane
  fixes make output-bisection invalid where the fix legitimately
  changes order; use exact-shape control variants like D-315's, or
  setFocus-ablation membership tests), fuzz_cep 3×300 (seeds 318901+;
  no bank suppression — known banked cases re-report, check names).

## REPO STATE (2026-07-18)

- Local UNPUSHED: 88346d8 (D-316 probe) → 2ccf5a5 (D-316 lift) →
  fcaf322 (D-317 clock-fuzz wiring) → 02b0bc9 (D-318 hunt) → this
  handoff commit. Released + pushed through v0.4.41 (f5a934e); PyPI
  live. CHANGELOG "Unreleased" already carries: verbatim decimal
  ingestion, scale-sensitive TMS identity, decimal-literal walls,
  windowed logical aggregates. A "bump tag one point, and push to
  release" directive may arrive before or after this slab — the flow
  is in the release-review memory (CHANGELOG → versions →
  wheel from bindings/ → extract .so over tracked → fresh-venv
  sanity → commit "vX: version bump" → lightweight tag → push
  main+tag; publish-crates ALWAYS red = Bryan's crates.io TP;
  PyPI JSON lags ~2min).
- Standing ledger: crates.io TP config (Bryan); the expiration ND/NE
  unblock-landing probe round (D-317's named open item, 3 witnesses
  banked, fuzz_cep fence in place); ?query justifiers (honestly
  unprobed); windowed average_exact authoring fence (own ox/ex round
  if wanted).
- Oracle runs: `./target/debug/seine-harness oracle|run|diff <file>`;
  batch fine for non-cyclic; -Xss1g pinned. Venv: `.venv/bin/python`
  and `../.venv/bin/maturin` (PATH quirk).
- Gate discipline: this slab's port is PRE-CLEARED contingent on a
  coherent grid; anything beyond (new walls, other lanes) gates
  separately. Commit per green phase; NEVER push without Bryan's
  directive.
