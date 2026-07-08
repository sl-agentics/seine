# hang-backlog — pre-existing engine NON-TERMINATION repros (E1-hardening)

Scenarios here **hang the engine** (spin in `next_activation`/`fire_all`, a
non-termination the fire limit can't catch). They are NOT gated or linted (this
dir is outside every Makefile/lint glob). **Do not run them through `make diff`
or `lint-probes`** — use `timeout N cargo run -q -p seine-harness -- run <f>`.

- `pre_existing_temporal_delete_hang.json` — a temporal-join + delete + advance
  shape (flushed by the CEP EP fuzz, D-116; bisected to HEAD → pre-existing,
  NOT entry-point-related). E1-hardening: temporal + external-delete
  non-termination, same family as the temporal-join-order latents.
