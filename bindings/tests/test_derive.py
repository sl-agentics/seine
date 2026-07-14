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
# battery vectors (reviewer-supplied, commit 8fecbaf), verbatim.
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
    drop a <=radius pair — sweep a lat/lon grid against ground truth.
    Grid capped at |lat| 89.5: above that sits the KNOWN pole-band gap
    of the pinned D-250 geometry (see test_pole_band_antipodal_gap)."""
    import itertools
    lats = [-89.5, -45.0, 0.0, 40.0, 89.5]
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


def test_pole_band_antipodal_gap_is_demo_identical():
    """KNOWN LIMIT of the pinned D-250 geometry, reproduced exactly
    (bit-compatibility with the demo's polars stage is this arc's
    contract): in the narrow pole band — mean |lat| between ~89.89
    (below which over-the-pole pairs exceed the radius) and ~89.93
    (above which the lon threshold saturates to 180) — a
    near-antipodal-lon pair can be INSIDE the radius over the pole yet
    outside the cos(lat)-scaled lon threshold. Here: true dist ~22239m
    < 25km, wrapped dlon 179.99 deg > threshold ~128.7 deg -> no
    emission, in the demo and the kernel alike. Flagged in D-251 as a
    candidate future-round cell; a geometry change is a joint
    demo+kernel+battery decision, not a kernel-side fix."""
    rows = {"icao": ["A", "B"], "lat": [-89.9, -89.9],
            "lon": [-179.99, 0.0]}
    assert haversine_ref(-89.9, -179.99, -89.9, 0.0) < RADIUS_M
    assert len(derive.pair_candidates(rows, id="icao")) == 0


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
    as the independent vectorized reference implementation."""

    BBOX_M = 25_000.0
    DEG_M = 111_320.0
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
        lat_thresh = self.BBOX_M / self.DEG_M
        raw_dlon = (pl.col("lon_a") - pl.col("lon_b")).abs() % 360.0
        wrapped_dlon = pl.min_horizontal(raw_dlon, 360.0 - raw_dlon)
        coslat = (((pl.col("lat_a") + pl.col("lat_b")) / 2.0)
                  .radians().cos().clip(1e-6))
        lon_thresh = pl.min_horizontal(
            pl.lit(180.0), self.BBOX_M / (self.DEG_M * coslat)
        )
        cand = (
            a.join(b, how="cross")
            .filter(pl.col("icao_a") < pl.col("icao_b"))
            .filter((pl.col("lat_a") - pl.col("lat_b")).abs() < lat_thresh)
            .filter(wrapped_dlon < lon_thresh)
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
