"""ADS-B convergence WITHOUT the bespoke kernels — the composability
proof for the general surface.

The three hand-written kernels decompose into certified primitives:

  pair_candidates -> THE MATCH PLANE. Pairing is a self-join; the
      `a < b` dedup is one constraint (`icao > $i1`), and the copies
      into PairRaw are certified RHS insert args. No candidate PRUNE is
      needed at all — the prune existed to keep O(n²) haversine calls
      off the hot path, but the expression layer vectorizes the exact
      distance, so we compute it on every pair and `filter()` instead
      (the prune's completeness contract makes the kernel's output a
      subset of ours; agreement is checked on the within-radius set).
  haversine -> THE EXPRESSION LAYER, using the calculator row —
      written in the kernel's exact operation order, so the meters are
      BIT-IDENTICAL, not approximately equal.
  closing -> caller-owned state (the same dict contract the kernel
      documents) + one expression: dist < prev, null -> False.

The kernel pipeline (kernel_stage in test_derive.py) is re-composed
here verbatim as the comparison baseline.
"""
import math

import pytest

import seine_rs
from seine_rs import derive
from seine_rs.derive import col, with_columns
from seine_rs.derive import filter as dfilter

EARTH_R = 6_371_000.0
RADIUS_M = 25_000.0
TTL_MS = 60_000

# the round-27/28 battery vectors, verbatim (test_derive.py)
VECTORS = [
    (40.0, -0.117, 40.0, 0.117, 19932, True),
    (40.0, 0.00, 40.0, 0.04, 3407, True),
    (40.0, 179.98, 40.0, -179.98, 3407, True),
    (89.9, 0.0, 89.9, 1.0, 194, True),
    (89.95, 0.0, 89.95, 3.0, 291, True),
    (0.0, 179.98, 0.0, -179.98, 4452, True),
    (40.0, 0.0, 40.0, 0.0, 0, True),
    (40.0, 0.0, 40.0, 0.5, 42704, False),
    (40.0, 0.0, 40.0, 1.0, 85394, False),
    (90.0, 0.0, 90.0, 180.0, 0, True),
    (89.99, 0.0, 89.99, 180.0, 2224, True),
    (-89.9, -179.99, -89.9, 0.0, 22239, True),
    (89.876, 0.0, 89.876, 130.0, 24996, True),
    (89.5, 0.0, 89.5, 180.0, 111195, False),
    (0.0, 0.0, 0.0, 0.224578, 24972, True),
]


def haversine_ref(lat1, lon1, lat2, lon2):
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = math.radians(lat2 - lat1), math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * EARTH_R * math.asin(math.sqrt(a))


# ------------------------------------------------- the kernel baseline

def kernel_stage(state, ts, rows):
    cand = derive.pair_candidates(
        {"icao": [r["icao"] for r in rows],
         "lat": [r["lat"] for r in rows],
         "lon": [r["lon"] for r in rows]},
        id="icao", radius_m=RADIUS_M)
    withd = derive.haversine(cand, lat1="lat_a", lon1="lon_a",
                             lat2="lat_b", lon2="lon_b", out="dist")
    out = derive.closing(state, ts, withd, key="key", dist="dist",
                         ttl_ms=TTL_MS)
    return {r["key"]: (r["dist"], r["closing"]) for r in out.to_pylist()}


# --------------------------------------------- the kernel-free pipeline

PAIR_DRL = (
    'rule "MakePair"\n'
    "when\n"
    "    Position($i1 : icao, $la1 : lat, $lo1 : lon)\n"
    "    Position($i2 : icao, icao > $i1, $la2 : lat, $lo2 : lon)\n"
    "then\n"
    "    insert(new PairRaw($i1, $i2, $la1, $la2, $lo1, $lo2));\n"
    "end\n"
)
PAIR_SCHEMAS = {
    "PairRaw": {"ia": "String", "ib": "String", "lat_a": "f64",
                "lat_b": "f64", "lon_a": "f64", "lon_b": "f64"},
}


def expr_pairs(rows):
    """a<b pairing via the match plane (a self-join is what a RETE
    network is for). Returns the PairRaw table, or None for < 2 rows."""
    res = seine_rs.run(
        PAIR_DRL,
        {"Position": {"icao": [r["icao"] for r in rows],
                      "lat": [r["lat"] for r in rows],
                      "lon": [r["lon"] for r in rows]}},
        schemas=PAIR_SCHEMAS,
    )
    return res.facts.get("PairRaw")


def expr_haversine(pairs):
    """The haversine formula as ONE expression, in the kernel's exact
    operation order (bit-identical meters — same libm, same fp order):
        dp = (lat2 - lat1).to_radians() / 2;  sp = dp.sin()
        h  = sp*sp + cos(p1)*cos(p2)*sl*sl
        d  = 2*R*asin(sqrt(h)), rounded half away, as i64
    """
    sp = ((col("lat_b") - col("lat_a")).radians() / 2.0).sin()
    sl = ((col("lon_b") - col("lon_a")).radians() / 2.0).sin()
    h = (sp * sp
         + col("lat_a").radians().cos() * col("lat_b").radians().cos() * sl * sl)
    d = 2.0 * EARTH_R * h.sqrt().asin()
    return with_columns(
        pairs,
        key=col("ia").concat("|").concat(col("ib")),
        dist=d.round().cast("i64"),
    )


def expr_closing(state, ts, table):
    """The kernel's closing contract with caller-owned state, the
    comparison as an expression: dist < prev, null (unseen/TTL'd
    out) -> False. Sweep-first, monotonic-ts, row-order updates —
    the same epoch hygiene the kernel documents."""
    cutoff = ts - TTL_MS
    for k, (_, t) in list(state.items()):
        assert t <= ts, "epochs must be monotonic"
        if t < cutoff:
            del state[k]
    rows = table.to_pylist()
    if not rows:
        return {}
    prev = [state.get(r["key"], (None, 0))[0] for r in rows]
    flagged = with_columns(
        {"key": [r["key"] for r in rows],
         "dist": [r["dist"] for r in rows],
         "prev": prev},
        closing=(col("dist") < col("prev")).fill_null(False),
    ) if any(p is not None for p in prev) else None
    out = {}
    for i, r in enumerate(rows):
        closing = (flagged.to_pylist()[i]["closing"] if flagged is not None
                   else False)
        state[r["key"]] = (r["dist"], ts)
        out[r["key"]] = (r["dist"], closing)
    return out


def expr_stage(state, ts, rows):
    """positions -> match-plane pairing -> expression haversine ->
    caller-state closing. No bespoke kernel anywhere."""
    pairs = expr_pairs(rows)
    if pairs is None or len(pairs) == 0:
        return {}
    return expr_closing(state, ts, expr_haversine(pairs))


# ------------------------------------------------------------ agreement

def test_haversine_expression_is_bit_identical():
    """The expression-layer distance equals the kernel's EXACTLY (same
    fp operation order, same libm) on every battery vector."""
    for lat1, lon1, lat2, lon2, approx, _ in VECTORS:
        t = expr_haversine(
            {"ia": ["A"], "ib": ["B"], "lat_a": [lat1], "lat_b": [lat2],
             "lon_a": [lon1], "lon_b": [lon2]})
        got = t.to_pylist()[0]["dist"]
        k = derive.haversine(
            {"lat1": [lat1], "lon1": [lon1], "lat2": [lat2], "lon2": [lon2]}
        ).to_pylist()[0]["dist_m"]
        assert got == k, (lat1, lon1, lat2, lon2, got, k)
        assert abs(got - haversine_ref(lat1, lon1, lat2, lon2)) <= 1


def test_match_plane_pairing_is_a_lt_b_cross_join():
    rows = [{"icao": c, "lat": 40.0, "lon": float(i)}
            for i, c in enumerate("ABCD")]
    pairs = expr_pairs(rows).to_pylist()
    keys = sorted(f"{p['ia']}|{p['ib']}" for p in pairs)
    assert keys == ["A|B", "A|C", "A|D", "B|C", "B|D", "C|D"]
    for p in pairs:
        assert p["ia"] < p["ib"]


def test_vector_agreement_with_kernel_stage():
    """On every battery vector: pairs the kernel emits carry the SAME
    integer distance in both pipelines; every within-radius pair (the
    completeness contract's set) appears in both."""
    for lat1, lon1, lat2, lon2, approx, must_emit in VECTORS:
        rows = [{"icao": "A", "lat": lat1, "lon": lon1},
                {"icao": "B", "lat": lat2, "lon": lon2}]
        kern = kernel_stage({}, 0, rows)
        expr = expr_stage({}, 0, rows)
        assert "A|B" in expr  # the expression pipeline computes ALL pairs
        for key, (kd, kc) in kern.items():
            assert expr[key] == (kd, kc), (key, expr[key], (kd, kc))
        if must_emit:
            assert "A|B" in kern  # the kernel battery's own guarantee


def test_closing_sequence_agreement():
    """The TTL feed from the kernel battery, both pipelines in
    lockstep: same flags, same state, epoch for epoch."""
    feed = [(0, 0.10), (5000, 0.05), (10_000, 0.06), (80_000, 0.04)]
    ks, es = {}, {}
    for ts, dlon in feed:
        rows = [{"icao": "A", "lat": 40.0, "lon": 0.0},
                {"icao": "B", "lat": 40.0, "lon": dlon}]
        kern = kernel_stage(ks, ts, rows)
        expr = expr_stage(es, ts, rows)
        assert kern == expr, (ts, kern, expr)
        assert ks == es, (ts, ks, es)  # caller-state parity


def test_end_to_end_alerts_without_kernels():
    """The full demo shape, kernel-free: positions -> pairing (match
    plane) -> haversine + filter (expressions) -> closing -> the
    convergence RULE fires on the computed fields."""
    alert_drl = ('rule "Converging"\n'
                 "when\n"
                 "    Pair($k : key, dist < 5000, closing == true)\n"
                 "then\n"
                 "end\n")
    state = {}
    alerts = []
    feed = [(0, 0.10), (5000, 0.05), (10_000, 0.03)]
    for ts, dlon in feed:
        rows = [{"icao": "A", "lat": 40.0, "lon": 0.0},
                {"icao": "B", "lat": 40.0, "lon": dlon}]
        staged = expr_stage(state, ts, rows)
        pair_rows = {"key": list(staged.keys()),
                     "dist": [v[0] for v in staged.values()],
                     "closing": [v[1] for v in staged.values()]}
        res = seine_rs.run(alert_drl, {"Pair": pair_rows})
        for f in res.firings.to_pylist():
            alerts.append((ts, f["values_json"]))
    # epoch 0: far, not closing. epoch 1: 4441m and closing -> ALERT.
    # epoch 2: 2665m, still closing -> ALERT.
    assert len(alerts) == 2 and alerts[0][0] == 5000 and alerts[1][0] == 10_000
    d1 = haversine_ref(40.0, 0.0, 40.0, 0.05)
    assert f'"dist":{round(d1)}' in alerts[0][1]


def test_filter_keeps_the_radius_contract():
    """dfilter(dist <= radius) over all-pairs equals the kernel's
    candidates∩radius set — the prune is now just a filter."""
    rows = [{"icao": "A", "lat": 40.0, "lon": 0.0},
            {"icao": "B", "lat": 40.0, "lon": 0.04},
            {"icao": "C", "lat": 40.0, "lon": 1.0},
            {"icao": "D", "lat": 40.0, "lon": 0.117}]
    within = dfilter(expr_haversine(expr_pairs(rows)),
                     col("dist") <= int(RADIUS_M))
    expr_keys = sorted(r["key"] for r in within.to_pylist())
    kern = kernel_stage({}, 0, rows)
    kern_keys = sorted(k for k, (d, _) in kern.items() if d <= RADIUS_M)
    assert expr_keys == kern_keys
    assert "A|C" not in expr_keys  # ~85 km apart
