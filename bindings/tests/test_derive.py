"""The derivation-plane certification battery (docs/derivation-plane.md).

seine_rs.derive's oracle is NOT the Drools pin — Drools has no opinion
about column math. It is (a) an INDEPENDENT pure-python reference
implementation (kept python on purpose: never Rust checked against
Rust), (b) property tests (symmetry, identity, determinism), and
(c) the ground-truth-driven vector battery from D-250/commit 8fecbaf:
every vector carries an explicit must_emit flag and the assert is
UNCONDITIONAL — a candidate-geometry miss (antimeridian wrap, cos-lat
scaling) turns red instead of hiding behind a kernel-only check.

Agreement with the demo's polars stage (the prototype these kernels
replace) is asserted vector-for-vector and over the scripted feed, so
the scenario twin scenarios/demo/adsb_convergence.json stays alive.
"""
import math
from pathlib import Path

import pytest

import seine_rs as s
from seine_rs import derive

EARTH_R = 6_371_000.0
RADIUS_M = 25_000.0
TTL_MS = 60_000


def haversine_ref(lat1, lon1, lat2, lon2):
    """Independent pure-python reference — the plane's oracle.
    (Mirrors demo/adsb_convergence.py's _haversine_ref; stays python.)"""
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = math.radians(lat2 - lat1), math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * EARTH_R * math.asin(math.sqrt(a))


# (lat1, lon1, lat2, lon2, ~true_dist_m, must_emit) — the round-27
# battery vectors (reviewer-supplied, commit 8fecbaf) verbatim, plus
# the round-28 D1/D2 vectors (the over-the-pole and boundary-shell
# geometries the D-250 prune falsely dropped).
VECTORS = [
    (40.0, -0.117, 40.0, 0.117, 19932, True),    # demo opening separation
    (40.0, 0.00, 40.0, 0.04, 3407, True),        # benign control
    (40.0, 179.98, 40.0, -179.98, 3407, True),   # antimeridian straddle
    (89.9, 0.0, 89.9, 1.0, 194, True),           # polar lon-compression
    (89.95, 0.0, 89.95, 3.0, 291, True),         # extreme polar compression
    (0.0, 179.98, 0.0, -179.98, 4452, True),     # antimeridian at equator
    (40.0, 0.0, 40.0, 0.0, 0, True),             # identity
    (40.0, 0.0, 40.0, 0.5, 42704, False),        # comfortably outside
    (40.0, 0.0, 40.0, 1.0, 85394, False),        # outside
    # round 28 (D1): over-the-pole convergence geometry
    (90.0, 0.0, 90.0, 180.0, 0, True),           # pole identity, wrapped 180
    (89.99, 0.0, 89.99, 180.0, 2224, True),      # over the pole, cap zone
    (-89.9, -179.99, -89.9, 0.0, 22239, True),   # the D-251 pole-band gap
    (89.876, 0.0, 89.876, 130.0, 24996, True),   # deep band: great-circle
                                                 # undercuts the parallel arc
    (89.5, 0.0, 89.5, 180.0, 111195, False),     # over-pole but outside
    # round 28 (D2): the old 111320 constant's falsely-pruned shell
    (0.0, 0.0, 0.0, 0.224578, 24972, True),
]


def kernel_stage(state, ts, rows, radius_m=RADIUS_M, ttl_ms=TTL_MS):
    """The demo's DerivationStage.derive(), composed from the kernels:
    candidates -> haversine on candidates -> TTL'd closing. Returns the
    demo's row shape ({ts, key, dist, closing}) for comparison."""
    cand = derive.pair_candidates(
        {"icao": [r["icao"] for r in rows],
         "lat": [r["lat"] for r in rows],
         "lon": [r["lon"] for r in rows]},
        id="icao", radius_m=radius_m)
    withd = derive.haversine(cand, lat1="lat_a", lon1="lon_a",
                             lat2="lat_b", lon2="lon_b", out="dist")
    out = derive.closing(state, ts, withd, key="key", dist="dist",
                         ttl_ms=ttl_ms)
    return [{"ts": ts, "key": r["key"], "dist": r["dist"],
             "closing": r["closing"]} for r in out.to_pylist()]


# ------------------------------------------------- ground-truth battery


def test_battery_ground_truth_driven():
    """Every true-dist-inside-radius pair MUST emit with the reference
    distance (unconditional, D-250); comfortably-outside must not."""
    for lat1, lon1, lat2, lon2, approx, must_emit in VECTORS:
        got = kernel_stage({}, 0, [
            {"icao": "A", "lat": lat1, "lon": lon1},
            {"icao": "B", "lat": lat2, "lon": lon2},
        ])
        ref = haversine_ref(lat1, lon1, lat2, lon2)
        # the table's ~true_dist_m values are documentation-grade
        # approximations (flat-earth arithmetic), not the oracle
        assert abs(ref - approx) <= max(1.0, 0.005 * approx)
        if must_emit:
            assert got, (f"MISSED close pair (true dist {ref:.0f}m): "
                         f"({lat1},{lon1})-({lat2},{lon2})")
            assert abs(got[0]["dist"] - ref) <= 1.0, (got, ref)
            assert got[0]["key"] == "A|B"
            assert got[0]["closing"] is False  # fresh state
        else:
            assert not got, f"emitted far pair (true dist {ref:.0f}m)"


def test_haversine_reference_crosscheck_and_symmetry():
    """|kernel - reference| <= 1m on the vectors and a fixed grid
    including the ugly regions (antimeridian, poles, near-antipodal);
    d(a,b) == d(b,a) exactly; d(a,a) == 0."""
    pts = [(lat, lon) for lat in (-89.95, -60.0, -0.01, 0.0, 40.0, 89.9, 90.0)
           for lon in (-180.0, -179.98, -90.0, 0.0, 0.117, 90.0, 179.98)]
    pairs = [(a, b) for a in pts for b in pts]
    cols = {
        "lat1": [a[0] for a, b in pairs], "lon1": [a[1] for a, b in pairs],
        "lat2": [b[0] for a, b in pairs], "lon2": [b[1] for a, b in pairs],
    }
    fwd = [r["dist_m"] for r in derive.haversine(cols).to_pylist()]
    rev_cols = {"lat1": cols["lat2"], "lon1": cols["lon2"],
                "lat2": cols["lat1"], "lon2": cols["lon1"]}
    rev = [r["dist_m"] for r in derive.haversine(rev_cols).to_pylist()]
    for k, ((a, b), d, dr) in enumerate(zip(pairs, fwd, rev)):
        ref = haversine_ref(a[0], a[1], b[0], b[1])
        assert abs(d - ref) <= 1.0, (a, b, d, ref)
        assert d == dr, ("symmetry", a, b, d, dr)
        if a == b:
            assert d == 0, ("identity", a)


def test_haversine_triangle_inequality_spot():
    tri = [((40.0, 0.0), (40.0, 0.1), (40.1, 0.05)),
           ((0.0, 179.9), (0.0, -179.9), (0.1, 180.0)),
           ((89.0, 0.0), (89.5, 90.0), (89.9, -170.0))]
    def d(p, q):
        t = derive.haversine({"lat1": [p[0]], "lon1": [p[1]],
                              "lat2": [q[0]], "lon2": [q[1]]})
        return t.to_pylist()[0]["dist_m"]
    for p, q, r in tri:
        assert d(p, q) <= d(p, r) + d(r, q) + 2  # +2: two 1m roundings


# ------------------------------------------------------ candidate prune


def test_pair_candidates_dedup_and_order():
    """a<b dedup over the cross join, rows in a-major cross-join order,
    and duplicate ids never self-pair."""
    cand = derive.pair_candidates(
        {"icao": ["C", "A", "B", "A"],
         "lat": [40.0, 40.0, 40.0, 40.0],
         "lon": [0.02, 0.00, 0.01, 0.00]},
        id="icao")
    keys = [r["key"] for r in cand.to_pylist()]
    # cross-join a-major order, a<b only; the duplicate "A" appears
    # twice as the a-side but never pairs with itself
    assert keys == ["A|C", "A|B", "B|C", "A|C", "A|B"]


def test_pair_candidates_int_ids():
    cand = derive.pair_candidates(
        {"tag": [7, 3], "lat": [40.0, 40.0], "lon": [0.0, 0.01]}, id="tag")
    rows = cand.to_pylist()
    assert rows[0]["tag_a"] == 3 and rows[0]["tag_b"] == 7
    assert rows[0]["key"] == "3|7"


def test_pair_candidates_prune_is_superset_of_truth():
    """The prune may keep a >radius pair (it is a bbox), but must not
    drop a <=radius pair — sweep a lat/lon grid against ground truth,
    poles included (the round-28 D1 fix closed the pole-band gap the
    old grid had to step around)."""
    import itertools
    lats = [-90.0, -89.99, -89.9, -89.5, -45.0, 0.0, 40.0, 89.5, 89.9, 89.99, 90.0]
    lons = [-179.99, -90.0, 0.0, 0.2, 179.99]
    pts = [(la, lo) for la, lo in itertools.product(lats, lons)]
    for i, (la1, lo1) in enumerate(pts):
        for la2, lo2 in pts[i + 1:]:
            true_d = haversine_ref(la1, lo1, la2, lo2)
            got = derive.pair_candidates(
                {"icao": ["A", "B"], "lat": [la1, la2], "lon": [lo1, lo2]},
                id="icao")
            if true_d < RADIUS_M:
                assert len(got) == 1, (
                    f"prune dropped a {true_d:.0f}m pair: "
                    f"({la1},{lo1})-({la2},{lo2})")


def test_pole_band_gap_closed_and_superset_random_sweep():
    """The D-251 pole-band gap is CLOSED (round 28, D1/D2): the prune's
    contract is completeness — every pair with true haversine distance
    <= radius emits. Verified by a fixed-seed randomized sweep biased
    at the hard geometries (poles, antimeridian, boundary shell)."""
    import random
    rng = random.Random(2028)
    checked = inside = 0
    for _ in range(3000):
        mode = rng.randrange(4)
        if mode == 0:      # near-pole band, wide lons
            la1 = rng.choice([1, -1]) * (89.7 + rng.random() * 0.3)
            la2 = la1 + (rng.random() - 0.5) * 0.4
            lo1, lo2 = rng.uniform(-180, 180), rng.uniform(-180, 180)
        elif mode == 1:    # antimeridian straddle
            la1 = rng.uniform(-89.99, 89.99); la2 = la1 + (rng.random() - 0.5) * 0.4
            lo1 = 179.9 + rng.random() * 0.2
            if lo1 > 180: lo1 -= 360
            lo2 = -179.9 - rng.random() * 0.2
            if lo2 < -180: lo2 += 360
        elif mode == 2:    # boundary shell at random latitude
            la1 = rng.uniform(-89.0, 89.0); la2 = la1
            arc = RADIUS_M / 111_194.9266
            lo1 = rng.uniform(-180, 180)
            lo2 = lo1 + arc / max(math.cos(math.radians(la1)), 1e-6) * rng.uniform(0.9, 1.0)
        else:              # generic near pair
            la1 = rng.uniform(-90, 90); la2 = max(-90, min(90, la1 + (rng.random() - 0.5) * 0.5))
            lo1 = rng.uniform(-180, 180); lo2 = lo1 + (rng.random() - 0.5) * 0.7
        d = haversine_ref(la1, lo1, la2, lo2)
        got = derive.pair_candidates(
            {"icao": ["A", "B"], "lat": [la1, la2], "lon": [lo1, lo2]},
            id="icao")
        checked += 1
        if d <= RADIUS_M:
            inside += 1
            assert len(got) == 1, (
                f"prune dropped a {d:.0f}m pair: ({la1},{lo1})-({la2},{lo2})")
    assert inside >= 200, f"sweep too weak: only {inside} inside-radius pairs"


# ------------------------------------------------- round-28 pins


def test_haversine_named_vectors_bit_match_reference():
    """Reviewer pin: haversine bit-matches the pure-python reference
    (half-away-from-zero rounding) on the named vectors."""
    named = [
        (0.0, 0.0, 0.0, 180.0),          # antipodal
        (0.0, 0.0, 0.001, 0.0),          # ~111 m
        (89.999, 10.0, 89.999, -170.0),  # near-pole seam
        (51.5074, -0.1278, 48.8566, 2.3522),   # London-Paris
        (-33.8688, 151.2093, 37.7749, -122.4194),  # Sydney-SF
    ]
    for la1, lo1, la2, lo2 in named:
        t = derive.haversine({"lat1": [la1], "lon1": [lo1],
                              "lat2": [la2], "lon2": [lo2]})
        got = t.to_pylist()[0]["dist_m"]
        ref = haversine_ref(la1, lo1, la2, lo2)
        want = math.floor(ref + 0.5) if ref >= 0 else -math.floor(-ref + 0.5)
        assert got == want, (la1, lo1, la2, lo2, got, ref)


def test_closing_ttl_boundary_inclusive():
    """Reviewer pin (cross-plane consistency with the expires_ms
    convention): an entry aged EXACTLY ttl survives; one ms older is
    swept."""
    st = {"A|B": (1000, 0)}
    out = derive.closing(st, 60_000, {"key": ["A|B"], "dist_m": [900]})
    assert out.to_pylist()[0]["closing"] is True, "age==ttl must survive"
    st = {"A|B": (1000, 0)}
    out = derive.closing(st, 60_001, {"key": ["A|B"], "dist_m": [900]})
    assert out.to_pylist()[0]["closing"] is False, "age==ttl+1 must sweep"


def test_closing_within_batch_duplicate_keys_row_order():
    """Reviewer pin: duplicate keys within one batch see row-order
    state — [1000, 900, 950] -> [False, True, False]."""
    out = derive.closing({}, 0, {"key": ["k", "k", "k"],
                                 "dist_m": [1000, 900, 950]})
    assert [r["closing"] for r in out.to_pylist()] == [False, True, False]


def test_backwards_epoch_ts_raises():
    """Round-28 Q1: a ts earlier than stored state errors loudly
    instead of silently computing against future-stamped entries."""
    st = {"A|B": (100, 5000)}
    with pytest.raises(ValueError, match="backwards"):
        derive.closing(st, 4000, {"key": ["A|B"], "dist_m": [50]})


def test_nan_coordinates_rejected():
    """Round-28 D3: NaN/inf coordinates raise — a NaN would otherwise
    pass the prune and cast to dist_m=0, the strongest possible false
    convergence signal."""
    nan = float("nan")
    with pytest.raises(ValueError, match="non-finite"):
        derive.haversine({"lat1": [nan], "lon1": [0.0],
                          "lat2": [0.0], "lon2": [0.0]})
    with pytest.raises(ValueError, match="non-finite"):
        derive.pair_candidates(
            {"icao": ["A", "B"], "lat": [nan, 0.0], "lon": [0.0, 0.0]},
            id="icao")
    with pytest.raises(ValueError, match="non-finite"):
        derive.pair_candidates(
            {"icao": ["A", "B"], "lat": [0.0, 0.0],
             "lon": [float("inf"), 0.0]}, id="icao")


def test_radius_zero_inclusive_coincident_pair():
    """Round 28: the completeness contract is INCLUSIVE (true dist <=
    radius emits), deliberately flipping the pre-round-28 strict-<
    pin: at radius_m=0 a coincident pair now admits."""
    got = derive.pair_candidates(
        {"icao": ["A", "B"], "lat": [10.0, 10.0], "lon": [20.0, 20.0]},
        id="icao", radius_m=0.0)
    assert len(got) == 1
    far = derive.pair_candidates(
        {"icao": ["A", "B"], "lat": [10.0, 10.0], "lon": [20.0, 20.001]},
        id="icao", radius_m=0.0)
    assert len(far) == 0


def test_pair_candidates_empty_and_single():
    one = derive.pair_candidates(
        {"icao": ["A"], "lat": [40.0], "lon": [0.0]}, id="icao")
    assert len(one) == 0
    # empty candidate tables flow through the rest of the chain
    withd = derive.haversine(one, lat1="lat_a", lon1="lon_a",
                             lat2="lat_b", lon2="lon_b", out="dist")
    state = {"stale|key": (100, 0)}
    out = derive.closing(state, 605_000, withd, key="key", dist="dist")
    assert len(out) == 0
    assert state == {}, "TTL sweep must run even on an empty batch"


# ------------------------------------------------------- closing state


def test_closing_state_ttl_sequence():
    """The reviewer's state-TTL vector: t=0 far -> t=5000 closer
    (closing True) -> 600s gap, closer still (closing MUST be False:
    the stale prev was evicted, not compared against)."""
    state = {}
    def pos(dlon):
        return [{"icao": "A", "lat": 40.0, "lon": 0.0},
                {"icao": "B", "lat": 40.0, "lon": dlon}]
    first = kernel_stage(state, 0, pos(0.10))
    assert first and first[0]["closing"] is False
    mid = kernel_stage(state, 5000, pos(0.05))
    assert mid and mid[0]["closing"] is True, "in-horizon closing must hold"
    post_gap = kernel_stage(state, 605_000, pos(0.04))
    assert post_gap and post_gap[0]["closing"] is False, \
        "stale prev_dist used after TTL"


def test_closing_state_is_caller_owned_and_inspectable():
    state = {}
    derive.closing(state, 1000, {"key": ["A|B"], "dist_m": [500]})
    assert state == {"A|B": (500, 1000)}
    # equal distance is NOT closing (strictly decreasing)
    out = derive.closing(state, 2000, {"key": ["A|B"], "dist_m": [500]})
    assert out.to_pylist()[0]["closing"] is False
    # increasing is not closing either
    out = derive.closing(state, 3000, {"key": ["A|B"], "dist_m": [600]})
    assert out.to_pylist()[0]["closing"] is False
    out = derive.closing(state, 4000, {"key": ["A|B"], "dist_m": [599]})
    assert out.to_pylist()[0]["closing"] is True


def test_closing_replay_re_derives():
    """Same raw epoch sequence, fresh state -> identical outputs (the
    WAL-replay determinism the epoch contract requires)."""
    feed = [(0, 0.10), (5000, 0.05), (10_000, 0.06), (80_000, 0.04)]
    def run():
        st, out = {}, []
        for ts, dlon in feed:
            out.append(kernel_stage(st, ts, [
                {"icao": "A", "lat": 40.0, "lon": 0.0},
                {"icao": "B", "lat": 40.0, "lon": dlon}]))
        return out
    assert run() == run()


def test_determinism_byte_identical():
    """Same batch in, byte-identical batch out, repeated."""
    cols = {"lat1": [40.0, 89.9, 0.0], "lon1": [-0.117, 0.0, 179.98],
            "lat2": [40.0, 89.9, 0.0], "lon2": [0.117, 1.0, -179.98]}
    a = derive.haversine(cols).to_pylist()
    b = derive.haversine(cols).to_pylist()
    assert a == b


# --------------------- agreement with the retired polars prototype
# The demo's DerivationStage is kernel-backed since the D-252 swap, so
# the INDEPENDENT vectorized cross-check lives here: the retired
# polars stage, verbatim (dev-only dependency, never the wheel's).


class _PolarsStage:
    """demo/adsb_convergence.py's pre-swap DerivationStage, preserved
    as the independent vectorized reference implementation — its prune
    updated in lockstep to the round-28 sound geometry (it checks the
    SAME spec as the kernel, from an independent implementation)."""

    BBOX_M = 25_000.0
    STATE_TTL_MS = 60_000

    def __init__(self):
        import polars as pl
        self.pl = pl
        self.prev_dist = {}

    def derive(self, ts, rows):
        pl = self.pl
        cutoff = ts - self.STATE_TTL_MS
        self.prev_dist = {k: (d, t) for k, (d, t) in self.prev_dist.items()
                          if t >= cutoff}
        if len(rows) < 2:
            return []
        df = pl.DataFrame(rows)
        a = df.rename({c: f"{c}_a" for c in df.columns})
        b = df.rename({c: f"{c}_b" for c in df.columns})
        theta = self.BBOX_M / EARTH_R
        eps = 1e-12
        half_pi = math.pi / 2.0
        la, lb = pl.col("lat_a").radians(), pl.col("lat_b").radians()
        lat_ok = (la - lb).abs() <= theta + eps
        colat_sum = pl.min_horizontal(
            (half_pi - la) + (half_pi - lb), (half_pi + la) + (half_pi + lb))
        over_pole = colat_sum <= theta + eps
        phi_m = pl.max_horizontal(la.abs(), lb.abs())
        raw_dlon = (pl.col("lon_a") - pl.col("lon_b")).abs() % 360.0
        wrapped = pl.min_horizontal(raw_dlon, 360.0 - raw_dlon).radians()
        dmax = (math.sin(theta) / phi_m.cos()).clip(upper_bound=1.0).arcsin()
        lon_ok = (phi_m + theta >= half_pi) | (wrapped <= dmax + eps)
        cand = (
            a.join(b, how="cross")
            .filter(pl.col("icao_a") < pl.col("icao_b"))
            .filter(lat_ok & (over_pole | lon_ok))
        )
        if cand.height == 0:
            return []
        lat_a, lat_b = pl.col("lat_a").radians(), pl.col("lat_b").radians()
        dp = (pl.col("lat_b") - pl.col("lat_a")).radians() / 2.0
        dl = (pl.col("lon_b") - pl.col("lon_a")).radians() / 2.0
        h = dp.sin().pow(2) + lat_a.cos() * lat_b.cos() * dl.sin().pow(2)
        cand = cand.with_columns(
            (2.0 * EARTH_R * h.sqrt().arcsin()).round(0)
            .cast(pl.Int64).alias("dist"),
            (pl.col("icao_a") + "|" + pl.col("icao_b")).alias("key"),
        )
        out = []
        for key, dist in cand.select("key", "dist").iter_rows():
            prev = self.prev_dist.get(key)
            closing = prev is not None and dist < prev[0]
            self.prev_dist[key] = (dist, ts)
            out.append({"ts": ts, "key": key, "dist": dist,
                        "closing": closing})
        return out


def _load_demo():
    import importlib.util
    path = Path(__file__).resolve().parents[2] / "demo" / "adsb_convergence.py"
    spec = importlib.util.spec_from_file_location("adsb_demo", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)  # runs the demo's _selfcheck at import
    return mod


def test_agreement_with_polars_reference():
    """Kernels vs the retired polars prototype, row-for-row on every
    battery vector and on the demo's scripted feed with state parity —
    the cross-implementation check that kept the scenario twin alive
    through the swap."""
    pytest.importorskip("polars")
    demo = _load_demo()
    for lat1, lon1, lat2, lon2, _approx, _must in VECTORS:
        rows = [{"icao": "A", "lat": lat1, "lon": lon1},
                {"icao": "B", "lat": lat2, "lon": lon2}]
        assert kernel_stage({}, 0, rows) == _PolarsStage().derive(0, rows)
    ref = _PolarsStage()
    state = {}
    for ts, rows, _label in demo.scripted_feed():
        assert kernel_stage(state, ts, rows) == ref.derive(ts, rows), \
            f"divergence at t={ts}"
        assert state == ref.prev_dist, f"state divergence at t={ts}"


def test_demo_stage_matches_kernel_composition():
    """The demo's kernel-backed DerivationStage is wired exactly like
    kernel_stage (same kernels, same params) — a wiring check, cheap
    and polars-free."""
    demo = _load_demo()
    for lat1, lon1, lat2, lon2, _approx, _must in VECTORS:
        rows = [{"icao": "A", "lat": lat1, "lon": lon1},
                {"icao": "B", "lat": lat2, "lon": lon2}]
        stage = demo.DerivationStage()
        assert kernel_stage({}, 0, rows) == stage.derive(0, rows)
    stage = demo.DerivationStage()
    state = {}
    for ts, rows, _label in demo.scripted_feed():
        assert kernel_stage(state, ts, rows) == stage.derive(ts, rows)
        assert state == stage.prev_dist


def test_arrow_input_path_matches_dict_path():
    pl = pytest.importorskip("polars")
    rows = {"icao": ["A", "B", "C"], "lat": [40.0, 40.0, 89.9],
            "lon": [-0.117, 0.117, 1.0]}
    via_dict = derive.pair_candidates(rows, id="icao").to_pylist()
    via_arrow = derive.pair_candidates(pl.DataFrame(rows), id="icao").to_pylist()
    assert via_dict == via_arrow


# ------------------------------------------- feeding the match plane


def test_derived_batch_feeds_session_directly():
    """Kernel output is a seine_rs.Table — Session/run() ingest it with
    no pyarrow/polars in between (the zero-dep D-221 contract)."""
    cand = derive.pair_candidates(
        {"icao": ["AC1", "AC2"], "lat": [40.0, 40.0],
         "lon": [-0.0263, 0.0263]}, id="icao")
    withd = derive.haversine(cand, lat1="lat_a", lon1="lon_a",
                             lat2="lat_b", lon2="lon_b", out="dist")
    pair = derive.closing({}, 0, withd, key="key", dist="dist")
    expect = pair.to_pylist()[0]
    assert 0 < expect["dist"] < 5000  # scripted-feed epoch-2 geometry
    drl = """
    rule "converging"
    when
        $p : Cand(dist < 5000, closing == false)
    then
        insert(new Alert($p.getKey(), $p.getDist()));
    end
    """
    res = s.run(drl, {"Cand": pair},
                schemas={"Alert": {"key": "String", "dist": "i64"}})
    alerts = res.derived["Alert"].to_pylist()
    assert len(alerts) == 1
    assert alerts[0]["key"] == "AC1|AC2" and alerts[0]["dist"] == expect["dist"]
    assert abs(expect["dist"] - haversine_ref(40.0, -0.0263, 40.0, 0.0263)) <= 1


# ------------------------------------------------------- error paths


def test_missing_column_is_loud():
    with pytest.raises(ValueError, match="missing column"):
        derive.haversine({"lat1": [1.0], "lon1": [1.0], "lat2": [1.0]})
    with pytest.raises(ValueError, match="missing column"):
        derive.pair_candidates({"icao": ["A"], "lat": [1.0]}, id="icao")
    with pytest.raises(ValueError, match="missing column"):
        derive.closing({}, 0, {"dist_m": [1]})


def test_non_numeric_column_is_loud():
    with pytest.raises(TypeError, match="numeric"):
        derive.haversine({"lat1": ["x"], "lon1": [1.0],
                          "lat2": [1.0], "lon2": [1.0]})
    with pytest.raises(TypeError, match="utf8 or int64"):
        derive.pair_candidates(
            {"id": [1.5], "lat": [1.0], "lon": [1.0]})


def test_nulls_rejected_loudly():
    pl = pytest.importorskip("polars")
    df = pl.DataFrame({"lat1": [40.0, None], "lon1": [0.0, 0.0],
                       "lat2": [40.0, 40.0], "lon2": [0.1, 0.1]})
    with pytest.raises(ValueError, match="null"):
        derive.haversine(df)


def test_output_name_collision_is_loud():
    with pytest.raises(ValueError, match="already exists"):
        derive.haversine({"lat1": [1.0], "lon1": [1.0], "lat2": [1.0],
                          "lon2": [1.0], "dist_m": [0]})
    with pytest.raises(ValueError, match="already exists"):
        derive.closing({}, 0, {"key": ["k"], "dist_m": [1],
                               "closing": [True]})


def test_bad_state_shape_is_loud():
    with pytest.raises(ValueError, match=r"\(dist, epoch_ts\)"):
        derive.closing({"k": "oops"}, 0, {"key": ["k"], "dist_m": [1]})
