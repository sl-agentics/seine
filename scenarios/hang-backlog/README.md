# hang-backlog — pre-existing engine NON-TERMINATION repros (E1-hardening)

Scenarios here drive the engine into a **non-terminating agenda cycle** — a
re-add loop in `next_activation` (TMS deferred drains / agenda scan) the fire
limit can't catch. Since D-117 the engine no longer HANGS on them: a per-call
step **spin-guard** (`AGENDA_SPIN_LIMIT`) trips and the engine ERRORS instead.
So they now surface as a divergence (guard-error vs oracle-success), documenting
the underlying bug without wedging.

They are NOT gated or linted (this dir is outside every Makefile/lint glob) —
they'd FAIL the gate (a real divergence) and are slow (~18s to trip the guard).
Run with `cargo run -q -p seine-harness -- run <f>` (recon: `SEINE_SPIN_GUARD=100000`
turns the trip into milliseconds; the verdict is limit-independent, D-175).

- `pre_existing_temporal_delete_hang.json` — a temporal-join + delete + advance
  + TMS (`insertLogical`) shape (flushed by the CEP EP fuzz, D-116; bisected to
  HEAD → pre-existing, NOT entry-point-related). The spin is the TMS
  `exp_deferred`/`deferred` re-add drain in `next_activation` (D-080/D-106
  envelope). ROOT CAUSE CRACKED + fix validated at D-175 (the teardown cause
  split — validate-and-revert, Bryan-gated); this scenario PASSES engine==oracle
  with that fix in-tree and graduates at the landing.
- `spin_c3_delpartner.json` / `spin_c4_noE1s.json` / `spin_c6_noEP.json` —
  D-175 spin-family cells (delete-the-unmarked-partner / no-E1s / no-entry-point);
  each spins on HEAD, passes with the D-175 fix → live pins at landing. c3 is the
  round-3 kill of the identity-model law (necessity direction).
- `spin_deps_extdel.json` / `spin_deps_delpartner.json` — the deps-carrying
  (insertLogical) witness shapes; oracle 3×: [TJ1, RN, TJ1, RL] (eager belief
  drop at the delete's propagation, mid-mark-window). Spin on HEAD; with the
  D-175 fix they TERMINATE but under-fire RN ([TJ1,TJ1,RL]) — the residual
  halt-fine-structure corner (D-106/cf5x17): open_divergence pins at landing.
- `spin_deps_k1.json` — a previously-unknown LATENT family member: k=1
  justifier, no temporal join. Spins on HEAD; byte-identical with the D-175 fix
  (oracle 3×: [J1, RN, J1, RL]) → live pin at landing.
