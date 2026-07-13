# RD×2 engine-anomaly instrumentation results
# (predictions: ird-anomaly-predictions.md)

Seven SEINE_TMS_DEBUG-gated probes added to the TMS key path
(stated-note / logical / route-del entry-state / dump3-noop /
unstage / plain-with-clear / materialize — engine.rs, house tag
style, corpus-inert). Traces on b1 / r1 / d1 / d2
(scratch anomaly_trace.txt; the signatures below are verbatim).

## H1 CONFIRMED END-TO-END — the mis-scoped beliefs.clear()

The r1 trace, exactly the pre-registered H1 signature:
```
route-del f1 rhs=true stated=[f1,f2] beliefs=1 pending=true
route-del/plain f1 beliefs-cleared=1 stated-remainder=[FactId(2)] pending-still=true
route-del f2 rhs=true stated=[f2] beliefs=0 pending=true
route-del/plain f2 beliefs-cleared=0 stated-remainder=[] pending-still=true
```
- Kill#1 (a NON-LAST stated delete) falls past the unstage gate
  into the plain tail, which runs `e.beliefs.clear()`
  unconditionally — **1 belief dropped with a non-empty stated
  remainder** (the mis-scope fingerprint; the tms_e6 comment
  "stated-only key dies with its handles" only contemplated the
  last-handle case).
- Kill#2 (the LAST stated) reaches the unstage gate with
  `beliefs=0` — the `!e.beliefs.is_empty()` conjunct fails — and
  **pending_vals is STILL Some** (the belief values sit unused).
  No materialize line ever fires.
- d1: identical two-kill signature. d2: the clear at kill#1, then
  two gate-fails — RD×3, no unstage. b1 (control): the single
  stated's kill reaches the gate with beliefs=1 →
  `route-del/unstage` + `materialize`, and the materialized
  handle's own kill takes the justified path — the machinery is
  correct whenever the gate is reached intact.

H2 (pending_vals lost) — REFUTED empirically: pending=true at
every kill#2/#3. H3 (had_justified no-op) — REFUTED: every branch
is plain, never dump3-noop; had_justified=false throughout. H4 (an
unread actor) — NONE: no entry mutation appears between the
route-del lines.

## Second confirmation riding the same traces

`stated-note` registers ALL same-value stateds on ONE key
(r1/d1/d2: f1 and f2 note the same key) — the engine has no
ACTIVATION-BACKFILL split (D-208's law: pre-activation stateds get
per-handle keys, last one mapped). Both engine gaps behind d1/d2
are now trace-confirmed from the engine side: (1) the mis-scoped
clear kills the unstage; (2) the one-key model diverges from the
oracle's key topology whenever stateds precede the first logical.

## The pinned port map (for the port slab — nothing landed here)

1. Scope `e.beliefs.clear()` to the stated-empty case (the tms_e6
   intent); the unstage gate then sees the beliefs at the last
   stated's kill. Covers the b1/r1/d1/d2 kill-count axis.
2. The activation-backfill key topology (tms_note_stated → an
   activation-time backfill with a value-map; per-key events).
3. The dynamic law's act-survival (the terminal-drain cancel
   exemption for unstage-born handles + the stale-value fire
   hazard flagged at D-203).
4. The r1 orphan/whole-key-death event + x1 undeletability against
   the engine's current had_justified approximation.
All against model_ird.py (31 cells / 13 mutation rows / the
750-case clean population), engine baseline 86/750,
validate-and-revert, Bryan gates.

## Receipts

Instrumentation gated off by default: corpus 11/1124/370 + drift
59 byte-identical, lint 1769/0/0, cells 39/39, witnesses 26/26,
model 31/31, agenda_open ×19 byte-identical vs the session-start
capture. Censuses (the every-engine-change discipline):

SD census 12×150: 0-div on all seeds, divergents
6/10/3/5/6/5/5/6/8/7/4/7 = **72 EXACT** (the baseline). ird census
5×150: **150/150 clean ×5, corners none, 86 EXACT**. The
instrumentation is behavior-inert on every axis; the change is
safe to keep as the permanent TMS key-path debug facility.
