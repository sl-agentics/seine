# I-RD population predictions (logged BEFORE any fuzz runs)

Deliverable: tools/fuzz_tms_ird.py — arc-local (gen.rs walls STAY
UP), the fuzz_tms_sd.py pattern: draw random compositions from the
model_ird vocabulary, generate DRL + the model spec from ONE draw,
score model-vs-oracle (the 0-div gate; raw mismatches oracle-3×
flake-filtered) and engine-vs-oracle (the census = the port
baseline). Fresh-seed protocol: comparison seeds 7001/7002/6001/
6003 then a NEVER-USED seed (9001).

## The grammar (v1)

Types T0/T1 only (the m6/m7 iso cells pin the self-join law on this
vocabulary; T2/T3 was 2442 fidelity). Facts: T0(true) ×1-2; 0-1
initial stated T1 per payload value; auto-added arm/trigger facts.
Payload values {v, w} (collisions create mixed keys and dep folds);
arm/trigger values disjoint from payloads. Rules 3-6 per scenario:
JL (brk ∈ {none-heavy, update, modify, delete}; selfjoin rare),
ST, DEL, OBS, MIDT (T1-premised justifier — makes REBIRTH reachable
by pure salience order: JL@high births, DEL@mid kills the key,
MIDT@low re-justifies fresh), KILLT0 (arm-gated premise killer),
RU (arm-gated premise updater).

CONFOUND CONTROLS (deliberate, logged):
- ALL rule saliences DISTINCT (drawn without replacement) — the
  pick-order/equal-salience layer is the SD arc's territory; the
  I-RD population must test the TMS laws, not the pick machinery.
  Same-rule multi-tuple order (FIFO) remains exercised.
- NO rule alpha matches T0(f0==false) — update-driven NEW
  activation is a general engine feature outside the I-RD laws and
  outside the model's scope; excluded by construction, documented.
- Arms/triggers never collide with payloads; DEL targets payloads
  only.

## Two model changes required by the grammar (flagged, not silent)

1. rhs_update: the D-206 assert "update invalidates a queued act"
   becomes CANCEL THE ACT. Justification: any grammar containing RU
   floods this corner; the behavior (an alpha-breaking update
   cancels queued unfired acts) is the D-076 eager-unmatch family,
   certified in the engine corpus for years — but it is NOT
   ird-cell-pinned, so it enters the model as an **IMPORTED
   commitment**, pre-registered here as an at-risk axis: if REAL
   mismatches cluster on RU shapes, the import is wrong for this
   vocabulary — build cells, do not tune.
2. MIDT gains a bf1 parameter (belief f1 value; default False keeps
   b2 byte-identical) so MIDT beliefs can share keys with JL
   beliefs (f1=true). Parameterization only — no semantic content.
Both changes must keep the D-206 validator at 22/22 and the
mutation matrix intact before any fuzz runs.

## Pre-registered predictions

1. **The 0-div claim**: model_ird matches the oracle on EVERY
   simulable generated case (0 REAL after 3× flake filter) across
   all five seeds — the three laws + their composition rules
   generalize. At-risk axes (mismatch clusters would land here):
   - the LAZY-BREAK SLOT (the ⚖-flagged underdetermined pick):
     self-join cases with observers BETWEEN the justifier's
     salience and the bottom discriminate at-justifier-salience vs
     end-of-agenda — the population runs the straddle cell the
     cells never had. If a cluster appears: re-pin the slot from
     the oracle output (that is a PIN, not a failure of the laws).
   - the IMPORTED update-cancel commitment (RU shapes).
   - PARTIAL dep breaks (two T0s, one killed: belief survives on
     n-1 deps) — cells never exercised partial breaks.
   - FIFO generalization under multi-rule agendas.
2. **Corner counts**: the three remaining assert-unreachable
   corners are all REACHABLE in this grammar and will fire:
   (a) stated-delete on a JUSTIFIED-born mixed key (JL@high +
   ST@lower + DEL), (b) belief-delete with stated siblings,
   (c) a break emptying a PENDING belief's deps (ST-born key + JL
   + premise kill/update). Predict >0 occurrences of at least (a)
   and (c) per 150-case seed. These are counted + one witness each
   banked per seed — they are NOT 0-div failures; they are the
   NEXT CELL ROUND's worklist. Predict (b) rarer (needs the DEL act
   to land on the belief handle specifically).
3. **The census**: engine-vs-oracle divergence substantially above
   the SD census rate (~3-7%/seed) because the grammar aims at the
   pinned divergence surfaces (dynamic law, r1 event, in-flush
   self-break, rebirth). Bracket: 10-40% of cases per seed. This
   number is the I-RD port baseline, not a gate.
4. **Oracle stability**: TMS-bar flake filtering stays quiet (the
   cells were 3×-stable throughout; predict 0-2 flaky cases per
   seed, quarantine-class if any).

## Gate meaning

0 REAL on all five seeds ⇒ the laws generalize; the port slab opens
with model_ird.py as the executable target and the census as its
baseline. ANY REAL mismatch ⇒ a law gap: minimize (drop rules/
facts), read the dump, build the discriminating cell BEFORE the
port — never patch the model from a single fuzz case.
