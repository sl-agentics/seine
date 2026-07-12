# hang-backlog — pre-existing engine NON-TERMINATION repros (E1-hardening)

Scenarios here drive the engine into a **non-terminating agenda cycle** — a
re-add loop in `next_activation` (TMS deferred drains / agenda scan) the fire
limit can't catch. Since D-117 the engine no longer HANGS on them: a per-call
step **spin-guard** (`AGENDA_SPIN_LIMIT`, overridable via `SEINE_SPIN_GUARD`
for recon — the verdict is limit-independent, D-175) trips and the engine
ERRORS instead. So they surface as a divergence (guard-error vs
oracle-success), documenting the underlying bug without wedging.

They are NOT gated or linted (this dir is outside every Makefile/lint glob) —
they'd FAIL the gate (a real divergence) and are slow (~18s to trip the guard
at the default limit). Run with `cargo run -q -p seine-harness -- run <f>`.

**The directory is currently EMPTY: the entire known D-117 family was cured by
the D-175 teardown cause split, landed at D-176.** Where everything went:

- `pre_existing_temporal_delete_hang.json` (the original D-116/D-117 repro),
  `spin_c3_delpartner.json`, `spin_c4_noE1s.json`, `spin_c6_noEP.json`,
  `spin_deps_k1.json` → `scenarios/regressions/` (live, engine==oracle).
- `spin_deps_extdel.json`, `spin_deps_delpartner.json` →
  `probes_pending/cep/tj_upd/tju_spin_deps_{extdel,delpartner}.json` as
  `open_divergence` pins: they TERMINATE post-fix but under-fire RN — the
  D-106 halt-fine-structure corner (the pick's D-101/cf5x17 static return vs
  Drools' pre-fire reopen), which needs its own halt-matrix arc.

The dir's charter stands for future cycles the guard catches: bank the repro
here, keep it out of the gates, and note it in DECISIONS. A hypothetical
still-uncured shape: an all-alive-marked act draining at its own justifier's
post-fire drain (reachable in principle, no witness yet — D-175 §6).
