# RD×2 engine-anomaly instrumentation predictions
# (logged BEFORE the instrumented runs)

The anomaly (D-205/D-208/D-209 flags, instrument-don't-armchair):
on r1/d1 the ENGINE fires the deleter ×2 and the belief never
materializes (oracle ×3 with the unstage); on d2 the engine fires
×3 with no unstage; on b1 (single stated) the engine unstages fine
(RD×2 matching the oracle's kill count — b1's divergence is only
the act-survival). The D-205 hypothesis list: (H2) pending_vals
None at the last stated delete; (H3) the had_justified no-op eating
it. This round adds instrumentation and pins the live path.

## The code-read hypothesis (pre-registered as H1, primary)

tms_route_delete_ex (engine.rs ~9875-9885): a stated delete that
does NOT empty the stated list falls PAST the unstage gate into the
branch tail — which runs `e.beliefs.clear()` unconditionally (the
"stated-only key dies with its handles / tms_e6" line). **The clear
is mis-scoped: it executes on NON-LAST stated deletes too.** At the
LAST stated's delete the unstage gate then reads
`!e.beliefs.is_empty()` == false and never takes pending_vals.
- Predicts r1/d1 (2 stateds): kill#1 plain + beliefs wiped; kill#2
  gate-fail, plain; NO materialize; RD×2. ✓ observed counts.
- Predicts d2 (3 stateds): kills #1/#2 wipe+plain, #3 gate-fail;
  RD×3 no unstage. ✓ observed.
- Predicts b1 (1 stated): kill#1 empties stated with beliefs intact
  → unstage fires. ✓ observed.
Code-read status of the old hypotheses: H2's "never set" is refuted
on paper (tms_insert_logical sets pending_vals whenever
justified=None and it is unset — r1's shape qualifies); H3 refuted
on paper (had_justified is only set on materialize/justified-birth
— never in this shape). The instrument settles them empirically:
the code-read could miss ANOTHER mutation site between the two
deletes (tms_eager_break, the drain machinery, a second
route-delete caller) — that possibility is exactly why this is an
instrumentation round and not a fix.

## The instrumentation (env-gated, house style, corpus-inert)

New SEINE_TMS_DEBUG tags (stderr, gated off by default — the
existing D-197+ tag precedent):
- `TMS key[stated-note]` in tms_note_stated: fact + key.
- `TMS key[logical]` in tms_insert_logical: key, need_insert,
  pending_vals set?, beliefs len after.
- `TMS key[route-del]` in tms_route_delete_ex at ENTRY: fact, rhs,
  and the entry state (stated list, beliefs len, pending_vals
  is_some, justified, had_justified); plus one tag per BRANCH taken
  (justified-path / dump3-noop / unstage / plain) and a
  `beliefs-cleared` tag when the tail clear fires with a NON-EMPTY
  stated remainder (the mis-scope signature).
- `TMS key[materialize]` in tms_materialize.

## Per-hypothesis predicted trace signatures (cells d1, r1, b1, d2)

- **H1 (mis-scoped clear)**: r1 trace shows route-del(s1):
  stated=[s1,s2], beliefs=1, pending=Some → branch=plain +
  `beliefs-cleared (1 dropped, stated remainder=[s2])`; then
  route-del(s2): beliefs=0, pending=Some(!) → gate-fail →
  branch=plain; NO materialize line. b1: route-del(s1): stated=[s1]
  → unstage branch + materialize. The pending_vals=Some at kill#2
  is H1's fingerprint (H2 predicts None there).
- **H2 (pending lost)**: route-del(s2) shows pending=None with
  beliefs≥1 → the unstage take() fails. (Also would need the
  logical tag to show pending never set, or a mutation between.)
- **H3 (had_justified no-op)**: route-del(s2) shows
  branch=dump3-noop (and RD's second kill would be a no-op — but
  the engine's s2 DOES die [finals 0], so H3 is doubly refuted
  unless the trace surprises).
- **H4 (an unread actor)**: any beliefs/pending mutation line
  between the two route-del lines that is NOT the tail clear —
  then the code-read was incomplete; re-read from the trace.

Prediction: H1 confirmed end-to-end, HIGH confidence; H2/H3 traces
contradict their signatures; no H4 lines.

## Port-fix preview (NOT this slab)

If H1 confirms: the fix shape is scoping the clear to the
stated-empty case — but the port slab must ALSO carry the
D-208 activation-backfill key-split (d1/d2 need TWO keys where the
engine's tms_note_stated builds one) and the dynamic law's
act-survival; the fix lands there against model_ird.py with
validate-and-revert, not here.

## Receipts plan (an engine change, debug-only)

Gated-off byte-identity: make diff (corpus 11/1124/370 + drift 59),
lint, cells 39/39, witnesses 26/26, model 31/31, agenda_open ×19
byte-identical vs the session-start capture (ag_run1.txt). Census
(the panic net, per the every-engine-change discipline): SD 12×150
expect 0-div + 72 EXACT; ird 5×150 expect 150/150 clean + 86 EXACT.
Any census movement ⇒ the "debug-only" change was not — revert and
re-read.
