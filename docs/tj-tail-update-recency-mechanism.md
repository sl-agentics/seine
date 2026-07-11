# D-165 — the tj-tail latents: UPDATE-RECENCY ordering (mechanism report)

_2026-07-11. Recon of the two quarantined fuzz_cep composition latents
`scenarios/xfail/cf933x385.json` + `scenarios/xfail/cf313x346.json`
(both PRE-EXISTING, both ORDER-class — full firing multisets verified
EQUAL engine-vs-oracle on HEAD; positional diffs only). Battery:
`probes_pending/cep/tj_tail/` (12 divergence cells + 6 control cells,
oracle 3×-deterministic). **This is a mechanism finding + fix plan; no
engine change is in this slab — the port is Bryan-gated.**_

## The one-line mechanism

**Drools orders re-enumeration by update recency; the engine does not
track updates in its ordering state.** Both witnesses — superficially a
10-rule salience mix (cf933x385) and a salience'd chain-rule composite
(cf313x346) — minimized to single-digit-line reproducers whose ONLY
active ingredient is a fact **update**. Every candidate seam in the
handoff (BfShadow×agenda partition, shadow×shadow P-type sharing,
tj_epoch drain, salience interleave, chain siblings) was a red herring:
the minimal cf313 case is ONE unsalienced rule; the minimal cf933 case
is `not E1() P()` + a `window:time` accumulate.

## Cell 1 — temporal-join partner enumeration (cf313x346)

Minimal: `CH2: $a:E0() $b:E1(this before[0,100] $a) $c:E2(this
after[0,100] $b)`; three E1s inserted 35y→5z→23z; E0 arrives; then an
epoch that UPDATES one E1 and inserts a fresh E2. The new E2's CH2
firings enumerate E1 partners:

- **oracle:** most-recently-UPDATED first (two updates stack
  most-recent-first — `tjt_two_upd`/`tjt_two_upd_rev`), then untouched
  facts in INSERTION order;
- **engine:** pure insertion order (the updated fact stays in its
  original slot).

Properties pinned by the battery:

| property | cell | result |
|---|---|---|
| durable across epochs (not batch-local) | `tjt_upd_sep_epoch` | DIVERGES |
| permanent — persists across MULTIPLE later arrivals | `tjt_two_arrivals` | DIVERGES |
| value-blind (values-identical update still hoists) | `tjt_upd_noop` | DIVERGES |
| idempotent (re-update = still one front slot) | `tjt_reupd` | DIVERGES |
| sequential updates stack most-recent-first | `tjt_seq_upd` | DIVERGES |
| scoped to the updated fact's own memory (E0 anchor update inert) | `tjt_upd_anchor` | control, MATCH |
| rightmost-pattern memory immune | `tjt_right_upd` | control, MATCH |
| updating the already-first fact is order-inert | `tjt_upd_first` | control, MATCH |

Second sub-seam in the same cell: when several facts are updated in one
epoch, their own re-fires emit **FIFO in update order oracle-side, LIFO
engine-side** (`tjt_two_upd` firings [2]-[3]).

## Cell 2 — event-not release order (cf933x385)

Minimal: `NE9: not E1() P()` (+ a `window:time(150)` accumulate over E1
that the minimizer could not drop — presence required, mechanism not
implicated in the order itself); P(1), then E1 + P(2); then one epoch
with `advance` (expires the blocker, releasing both Ps) and an update
of P(1).

- Baseline (no update, `tjt_933_noupd`): release order is
  recency-DESC — P(2) then P(1) — and MATCHES both trees (the D-134
  certified reversal).
- An update to a blocked-under P **in the expiry epoch** hoists it to
  release-FIRST oracle-side (`tjt_933_min`), on either side of the
  `advance` action (`tjt_933_upd_before_adv`). Engine order unmoved.
- Unlike cell 1 this is **epoch-local**: the same update in the epoch
  AFTER the expiry does not reorder (`tjt_933_split_epoch`, MATCH).
- Updating the already-first P is order-inert (`tjt_933_upd_p2`, MATCH).

## What it is NOT

- Not the BfShadow/PnShadow emit_rank composition (baseline orders
  match; shadows are per-rule-net and independent — verified in code).
- Not the D-136 tj_epoch drain, not salience partitioning, not
  chain-rule enumeration, not Allen inference (already ruled out at
  D-164).
- Not raw Drools memory-list placement: `TupleList.add` in
  drools-9.44-src is a TAIL-append, so the observed hoist-to-front is
  produced by some other internal seam (candidates: staged-update
  re-propagation order, RuleExecutor activation-list order). The
  INTERNAL Drools mechanism is deliberately left uncracked — the
  observable spec above is probe-pinned and deterministic; a BfDump
  graft can settle internals if the port needs ground truth.

## Engine seams for a port (pre-read, no change made)

1. Join cell: the partner scan sorts by `(left_sseq, left_seq)`
   (phreak.rs ~1376-1392); update's `re_add_left_tuple`/`re_add_right_tuple`
   (phreak.rs ~562/~640) move the tuple to list END but the scan's
   ORIGINAL stamps erase the move. A durable "front stamp" (e.g. a
   decreasing negative counter re-stamped on update, sorting ahead of
   all arrival stamps, most-recent-most-negative) reproduces the
   observed spec for all 12 cells; the FIFO-refire sub-seam is the
   update-batch drain order.
2. Event-not cell: the blocked-left release ordering at expiry
   (`pending_release` / D-134 §3B path) needs the same epoch-local
   updated-first hoist. ⚠ D-106 halt-model caveat applies near
   next_activation — checker-first.

Doctrine: model-check the spec in Python against a fuzz population
(update-heavy axis; the current fuzz_cep reaches these shapes only
rarely — consider the handoff's `SEINE_CEPMIX=1` idea narrowed to an
update-recency axis) BEFORE any Rust change, then the Bryan gate.

## Status

Both cf* witnesses stay QUARANTINED (xfail, name-keyed suppression);
their `_finding` fields now point here. The family spec is the
committed battery; graduation happens with the gated port.
