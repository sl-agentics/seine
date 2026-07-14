# The derivation plane — dataframe math upstream of the certified match

**Status: LANDED (D-249 design; D-251 kernels; D-252 demo swap). The
Rust/arrow-rs kernel library ships as `seine_rs.derive` (haversine,
pair_candidates, closing), certified by
bindings/tests/test_derive.py; the demo's DerivationStage runs on it.
Zero engine changes throughout.**

The pitch line: **Drools semantics in the match, dataframe semantics in
the data.** Seine never grows an `eval`/Java escape hatch — the seam
that makes upstream Drools unverifiable. Computation that Drools smuggles
*into* the match (arithmetic predicates, method calls, `eval`) lives
*outside* the match here, in a columnar derivation stage that produces
honest **fields**; the certified subset then constrains on those fields
with grammar it already has.

## Two planes, two oracles

| | Match plane | Derivation plane |
|---|---|---|
| What | RETE/PHREAK, TMS, temporal joins, agenda, queries | Vectorized pure functions over Arrow columns |
| Grammar | The frozen certified subset — **never grows arithmetic** | Anything expressible as `columns -> columns` |
| Oracle | Pinned Drools 9.44.0.Final+p1, byte-for-byte | Reference implementation + property tests |
| Hard part | Interleavings, epochs, belief revision — already certified | Nothing: pure functions on columns have no interleavings |

The asymmetry is the point. Cross-checking `f(column) == reference(column)`
is the easiest certification in this system; interleaving semantics are
the hardest and they are *done*. Keeping the planes separate means the
hard, finished thing never reopens.

## The epoch contract (extends D-102/D-242, does not bend it)

```
raw epoch record --> DERIVE (columnar, deterministic) --> derived facts
                 --> advance -> assert -> fire            (the certified step)
```

- Derivation runs **inside the epoch, upstream of assertion**. It is a
  deterministic, declared function of (raw batch, driver state), so the
  WAL stores **raw** epochs and replay re-derives identically — the
  stream_driver determinism guarantee extends unchanged.
- Derivation may keep state across epochs (e.g. previous positions for
  closing-rate) **only inside the derivation stage**, recomputed on
  replay. It never reads working memory: the data flow is one-way into
  the match plane. (Reading results back into derivation is a
  driver-level loop — legal, but it happens *between* epochs, visibly.)
- Derived facts are ordinary facts: events with timestamps if temporal,
  expiring like anything else. The match plane cannot tell they were
  computed.

## Pair generation (the genuinely hard part)

Cross-fact math (proximity, closing rate) means choosing which pairs to
compute. All-pairs is O(n²); the answer is the standard columnar shape:

1. a cheap vectorized **candidate pass** (bounding box / grid cell over
   position columns) prunes to plausible pairs;
2. exact math (haversine etc.) runs only on candidates;
3. candidates emit `Pair` facts; **the engine's certified temporal
   machinery does the rest** (persistence-of-convergence is `this_after`
   over successive `Pair` events, or TMS if alerts should self-retract).

## Certification battery (derivation side)

- **Reference cross-check**: every kernel tested against an independent
  implementation (geodesic references for haversine, numpy/polars for
  aggregates) on fixed vectors including the ugly ones (antimeridian,
  poles, zero distance, near-antipodal).
- **Property tests**: symmetry `d(a,b) == d(b,a)`, identity `d(a,a) == 0`,
  triangle-inequality spot checks, unit sanity — fixed seeds, no flake.
- **Determinism**: same input batch, same output batch, bit-for-bit —
  which is what lets the WAL store raw epochs.
- The derivation battery is **separate from `make diff`** and never
  gates on the Drools oracle: Drools has no opinion about column math.

## Declaration shape (prototype-level, Python)

```python
pairs = derive_pairs(
    aircraft_batch,                       # Arrow/polars columns
    candidate=bbox_within(0.1),           # cheap vectorized prune
    dist=haversine("lat", "lon"),         # exact math on candidates
    closing=decreasing("dist"),           # stateful across epochs
)
# -> Pair(ts, a, b, dist, closing) facts, asserted like any others

rule = Rule("convergence")
p = rule.when(Pair, Pair.dist < 5000, Pair.closing == True)
rule.then_insert(Alert, ...)              # grammar unchanged: two field constraints
```

The demo (demo/adsb_convergence.py) composes this stage from the
`seine_rs.derive` Rust arrow-rs kernels (it prototyped the same shape
in polars first; that implementation survives as the independent
vectorized cross-check inside the derive battery). Either way the
declaration is data-plane API — the Rule grammar above is today's
certified subset, untouched.

## What this forecloses, on the record

- `eval` / computed predicates in constraints: **WONT, permanently** —
  superseded by this design rather than deferred (extends the D-231
  RHS-arithmetic reasoning to the LHS escape-hatch family).
- Constraint arithmetic (D-061, `age + 1 > $x`) stays on the roadmap on
  its own merits (declarative *filtering* sugar), but proximity-class
  math never becomes its burden.
