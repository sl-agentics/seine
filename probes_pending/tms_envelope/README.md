# D-080 TMS envelope arc — recon home (opened D-186, 2026-07-11)

Arc plan + the full scoping read: `~/.claude/plans/tms-envelope-arc.md`.

- `triage-2026-07-11.md` — the arc's baseline: fresh 10-replicate
  triage of scenarios/xfail/ on HEAD 687b8ae with the +p1 oracle
  (`tools/triage_xfail.py --runs 10 --cache target/triage_cache_d080arc`).
  Supersedes docs/xfail-triage.md (the D-087 record, kept) as the
  comparison base. Movement vs D-087: exactly 3 witnesses, engine-side
  only, bisect-attributed — fz_42_4442 at D-091 `f70b189` (dirty-flag
  lifecycle port), fz_123_2674 + fz_42_7619 at D-101 `bb6eb6d`
  (drain-at-link slab). Oracle byte-stable on all 68.

The envelope = exactly the 68 TMS witnesses (45 VALUE + 22 RUNAWAY +
1 NONDET). Law-read buckets (candidate classifications, method-law
caveat — see the plan §3): L-SD self-defeat landing ×13 (11 under /
2 over), L-MB mutation-break landing ×18 (16 over / 2 mixed), I-RD
mixed-key kill path ×12 (6/6 mixed), I-ST static bookkeeping ×1
(fz_7_9902), compound ×1 (fz_7_9550). Runaways = identity-law mirror,
fenced-by-nature; nondet fenced-by-nature.

THE RESIDUE (the arc's work): R1 the (cloud × belief-loss) landing
rows — the fifth-law-shaped hole, evaluation-lifecycle region, ⚠
D-106-adjacent; R2 the static stated/justified bookkeeping model
(outside the identity law as written).

Ladder cells land here as `sd_*` / `mb_*` / `rd_*` when probing opens.
