# PINS — the four filed decimal pin candidates (D-315; Bryan: "do
# the four filed pin candidates")

Predictions registered 2026-07-18 BEFORE any cell ran. These are the
corners the D-313 fuzz axes deliberately AVOIDED; every cell here is
ordinary DRL/JSON, so unlike the averageExact grid they diff
directly. Where a prediction says DIVERGENCE, the oracle decides the
landing (doctrine) unless the divergence is our data-plane contract
vs the oracle's POJO verbatim-ness — that split is exactly what p1
measures.

## p1 — string-scale ingestion

Our D-098 ingestion rescales JSON strings to the declared (p,s)
(exact widening, HALF_UP narrowing — pin J, which aligned with
DUCKDB's DECIMAL(p,s) cast). The oracle's setTyped is
`new BigDecimal(v.asText())` — VERBATIM, the string's own scale.

- **p1_subscale**: "1.1" into decimal(10,2). PREDICT DIVERGENCE:
  oracle "1.1" (scale 1), engine "1.10".
- **p1_narrow**: "1.005" into decimal(10,2). PREDICT DIVERGENCE:
  oracle "1.005" (scale 3, verbatim), engine "1.01" (HALF_UP — a
  LOSSY rescale the oracle never performs).
- **p1_super**: "1.100" into decimal(10,2). PREDICT DIVERGENCE:
  oracle "1.100", engine "1.10".
- Landing if predicted: JSON/py-Decimal ingestion goes VERBATIM
  (precision-checked, no rescale) — the match plane's oracle is
  Drools; Arrow-COLUMN ingestion keeps the declared scale (columns
  are uniform-scale by construction; the derive plane's oracle is
  DuckDB, whose cast rescales). The declared s stays the Arrow/
  column contract, not a WM-value normalizer.

## p2 — int-JSON into a decimal field

`{"a": 3}` for decimal(10,2). PREDICT DIVERGENCE: oracle
`new BigDecimal("3")` → "3" (scale 0); engine I64→Dec rescale →
"3.00". Landing with p1: verbatim (scale 0).

## p3 — decimal eq-literals × the oracle's alpha hash groups (D-029)

Drools folds ≥3 same-field `==` literal alpha constraints into a
HASHED group (alphaNodeHashingThreshold=3) keyed by equals() —
BigDecimal.equals is SCALE-SENSITIVE. Our engine evaluates compareTo.

- **p3_trip**: THREE rules `M(a == 1) / (a == 2) / (a == 3)` over
  decimal a; facts "1.00", "2", "3.0" (scales 2/0/1). PREDICT
  (medium confidence — this is the genuinely unpinned cell): the
  oracle HASH-MISSES where literal-vs-value scales differ (how the
  DRL int literal coerces to the key decides which scale the group
  holds — possibly BigDecimal("1") scale 0, so only "2" hits), while
  the engine compareTo-matches all three. If the oracle instead
  compareTo-matches everything, the hash group normalizes and there
  is NO quirk (low-med).
- **p3_ctl**: TWO rules only (below threshold — no hash group) same
  facts. PREDICT MATCH both sides (plain MVEL compareTo — the
  D-308/D-309 certified surface).
- Landing per measurement: if the oracle hash-misses, the quirk gets
  a loud FENCE (≥3 same-field decimal eq-literals = compile error
  naming the scale-sensitive hashing) — modeling a hash-layout
  artifact is epicycle territory; if it matches, no action + the
  fuzzer keeps drawing freely. Either way fz_313902_761 gets
  re-examined against the mechanism.

## p4 — cross-scale runtime decimals into fields (the D-313 coerce
## change, now oracle-diffed) + RHS literal error parity

- **p4_ins**: sum over decimal(10,4) source → insert into a
  decimal(38,2) field. PREDICT MATCH: both sides store the runtime
  scale-4 value verbatim (the D-313 own-scale fix, previously
  certified only via regression cells).
- **p4_set**: `modify($b){ setV($s) }` same cross-scale flow.
  PREDICT MATCH (same coerce path).
- **p4_lit**: `insert(new M(2.5, ...))` into a decimal field —
  engine-only convenience (d098 pin). PREDICT ERROR-vs-SUCCESS
  DIVERGENCE: the oracle's generated POJO ctor takes BigDecimal and
  javac rejects the double literal. Landing if predicted: the engine
  WALLS bare numeric literals as decimal ctor/setter args (error
  parity; steering to bindings/ingestion), d098's engine-only pin
  updated.

## MEASUREMENTS (2026-07-18, same day; all diffs 3× stable)

- **p1 (all three) + p2: DIVERGENT AS PREDICTED** — oracle verbatim
  ("1.1", "1.005", "1.100", "3"), engine rescaled. LANDED: ingestion
  is VERBATIM (coerce's Str→Dec and I64→Dec arms keep the value's own
  scale, precision still enforced); the pin-J rescale contract now
  lives only at the Arrow COLUMN boundary. The lossy half-up narrow
  ("1.005" → 1.01) is GONE — it silently destroyed data the oracle
  preserved. d098 ingest pins updated to the measured semantics.
- **p3: NO QUIRK** — the oracle fires R1/R2/R3 across scales
  2/0/1 identically at and below the hash threshold; alpha
  eq-literal matching is compareTo on both sides. The medium-
  confidence hash-miss prediction MISSED (the good kind of miss).
  fz_313902_761 is NOT this mechanism — it stays xfailed as an
  unexplained agenda-order latent.
- **p4_ins / p4_set: MATCH** — the D-313 own-scale storage flow is
  now oracle-diffed (previously certified only via regression cells);
  GRADUATED with the rest (pr_dp_*, 8 cells).
- **p4_lit: ERROR PARITY LANDED** — the oracle javac-rejects numeric
  literals into BigDecimal ctors AND setters; the engine's D-098-era
  convenience conversion is now a loud wall on both sites.
  **p4_strlit (added same-day): string literals ALREADY error on
  both sides** ("wrong type" vs "constructor N(String) undefined") —
  parity held, no change. Both cells stay pending engine_fenced as
  the error-parity record.
- Fuzzer unlocked: DEC_POOL gains mixed scales ("1.1", "3", "-0.5",
  "2.500") + occasional int-JSON draws for dec fields. Smoke 500
  flushed fz_315901_311 — bisected PRE-EXISTING via an exact-scale
  control variant (diverges identically; pre-slab engine
  bit-identical on the control): a setFocus × or-branch-delete ×
  salience agenda-order latent (the fz_313002_319-adjacent family) →
  xfail, bank 54.

- **THE FIFTH FINDING** (battery seed 315001): TMS equality keys are
  SCALE-SENSITIVE (BigDecimal.equals) — KeyVal::D's
  trailing-zero normalization was a pin-J composition, falsified the
  moment mixed scales could flow; fixed to raw (u, s). Two
  regression tripwires banked; one more collect-order family member
  xfailed (bank 55).
