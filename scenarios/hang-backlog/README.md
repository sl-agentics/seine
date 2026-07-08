# hang-backlog — pre-existing engine NON-TERMINATION repros (E1-hardening)

Scenarios here drive the engine into a **non-terminating agenda cycle** — a
re-add loop in `next_activation` (TMS deferred drains / agenda scan) the fire
limit can't catch. Since D-117 the engine no longer HANGS on them: a per-call
step **spin-guard** (`AGENDA_SPIN_LIMIT`) trips and the engine ERRORS instead.
So they now surface as a divergence (guard-error vs oracle-success), documenting
the underlying bug without wedging.

They are NOT gated or linted (this dir is outside every Makefile/lint glob) —
they'd FAIL the gate (a real divergence) and are slow (~18s to trip the guard).
Run with `cargo run -q -p seine-harness -- run <f>`.

- `pre_existing_temporal_delete_hang.json` — a temporal-join + delete + advance
  + TMS (`insertLogical`) shape (flushed by the CEP EP fuzz, D-116; bisected to
  HEAD → pre-existing, NOT entry-point-related). The spin is the TMS
  `exp_deferred`/`deferred` re-add drain in `next_activation` (D-080/D-106
  envelope). E1-hardening ROOT-CAUSE fix pending; the D-117 guard only
  CONTAINS it (turns the hang into a catchable error).
