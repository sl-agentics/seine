"""ADS-B convergence — the TWO-PLANE pattern (docs/derivation-plane.md, D-249).

The problem that motivated it: proximity alerting needs haversine math,
and the certified match grammar has no arithmetic — by design. Drools
would smuggle the math into the match with eval()/Java, the exact seam
seine refuses to inherit. The Arrow-native answer:

  DERIVATION PLANE (this file's DerivationStage, seine_rs.derive —
      the Rust/arrow-rs kernels that replaced the polars prototype,
      D-251/D-252; zero extra dependencies):
      vectorized candidate pruning + haversine + closing-rate over
      position columns -> Pair facts with honest FIELDS.
      Its oracle: a pure-python reference implementation + property
      checks (run at import; see _selfcheck) — NOT the kernels
      themselves; the battery in bindings/tests/test_derive.py is the
      full certification, including agreement with the retired polars
      stage (kept there as the independent vectorized cross-check).

  MATCH PLANE (the certified subset, unchanged):
      Pair(dist < 5000, closing == true)          -- plain field constraints
      + this_after persistence over successive Pair events
      + expiry aging the pairs out.
      Its oracle: the pinned Drools, exactly as always. The epoch
      sequence this demo produces is byte-checked as
      scenarios/demo/adsb_convergence.json.

Drools semantics in the match, dataframe semantics in the data. The
driver extends demo/stream_driver.py's epoch contract by one stage:
raw epoch -> derive -> assert -> fire; the WAL stores RAW epochs and
replay re-derives, so determinism covers the whole pipeline.
"""
import math

import seine_rs as s

# ------------------------------------------------------------ match plane

@s.fact(event=s.Event(timestamp="ts", expires_ms=15000))
class Pair:
    ts: int
    key: str
    dist: int      # meters, derived (haversine)
    closing: bool  # derived (distance decreasing vs previous epoch)

@s.fact
class Alert:
    kind: str
    key: str
    dist: int

def build_rules():
    # the pitch line: constrain on derived values as ordinary fields
    conv = s.Rule("converging")
    p = conv.when(Pair, Pair.dist < 5000, Pair.closing == True)  # noqa: E712
    conv.then_insert(Alert, kind="converging", key=p.key, dist=p.dist)

    # persistence-of-convergence = the CERTIFIED temporal machinery over
    # derived events: two closing sub-5km pairs within 10s
    sus = s.Rule("sustained")
    p1 = sus.when(Pair, Pair.dist < 5000, Pair.closing == True)  # noqa: E712
    p2 = sus.when(Pair, Pair.dist < 5000, Pair.closing == True,  # noqa: E712
                  Pair.key == p1.key, s.this_after(p1, 1, 10000))
    sus.then_insert(Alert, kind="sustained", key=p2.key, dist=p2.dist)
    return [conv, sus]

# ------------------------------------------------------- derivation plane

EARTH_R = 6_371_000.0  # meters

def _haversine_ref(lat1, lon1, lat2, lon2):
    """Independent pure-python reference — the derivation plane's oracle."""
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = math.radians(lat2 - lat1), math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * EARTH_R * math.asin(math.sqrt(a))

class DerivationStage:
    """Columnar candidate pass + exact math + cross-epoch closing state,
    composed from the seine_rs.derive Rust kernels. Deterministic
    function of (raw batch, own state); never reads WM.

    The candidate prune is METRIC-space and COMPLETE (round-27 and
    round-28 findings, implemented inside derive.pair_candidates): the
    longitude delta WRAPS across the antimeridian, its bound is the
    spherical-cap limit (skipped when the radius cap reaches a pole),
    and over-the-pole pairs admit by colatitude-sum reachability — no
    pair whose true distance is inside the radius is ever dropped.
    Closing state is THIS
    object's plain dict (the kernels hold no state), entries carry the
    epoch timestamp, and derive.closing sweeps anything older than
    STATE_TTL_MS before comparing: a pair reappearing after a long gap
    does not compare against a stale distance. Eviction is a pure
    function of the raw epoch sequence, so WAL-replay determinism is
    unchanged."""

    BBOX_M = 25_000.0          # candidate radius, meters
    STATE_TTL_MS = 60_000      # closing-state freshness horizon

    def __init__(self):
        self.prev_dist = {}    # pair key -> (distance, epoch ts)

    def derive(self, ts, rows):
        if len(rows) < 2:
            # the TTL sweep is part of the epoch function even on a
            # degenerate batch (same hygiene derive.closing applies)
            cutoff = ts - self.STATE_TTL_MS
            self.prev_dist = {k: (d, t) for k, (d, t) in self.prev_dist.items()
                              if t >= cutoff}
            return []
        cand = s.derive.pair_candidates(
            {"icao": [r["icao"] for r in rows],
             "lat": [r["lat"] for r in rows],
             "lon": [r["lon"] for r in rows]},
            id="icao", radius_m=self.BBOX_M)
        withd = s.derive.haversine(cand, lat1="lat_a", lon1="lon_a",
                                   lat2="lat_b", lon2="lon_b", out="dist")
        out = s.derive.closing(self.prev_dist, ts, withd,
                               key="key", dist="dist",
                               ttl_ms=self.STATE_TTL_MS)
        return [{"ts": ts, "key": r["key"], "dist": r["dist"],
                 "closing": r["closing"]} for r in out.to_pylist()]

def _selfcheck():
    """The derivation battery in miniature, GROUND-TRUTH-DRIVEN (round-27
    hardening): every input pair whose TRUE distance is within the
    candidate radius MUST emit a Pair with the reference distance —
    the assert is unconditional, so a candidate-geometry miss (the
    antimeridian / cos-lat findings) turns red instead of silently
    passing a kernel-only symmetry check."""
    stage = DerivationStage()
    # (lat1, lon1, lat2, lon2, must_emit) — the round-27 battery vectors
    # (reviewer-supplied), each with an explicit ground-truth emission flag
    cases = [
        (40.0, -0.117, 40.0, 0.117, True),     # the demo's opening separation
        (40.0, 0.00, 40.0, 0.04, True),        # benign control, ~3407m
        (40.0, 179.98, 40.0, -179.98, True),   # antimeridian straddle, ~3407m
        (89.9, 0.0, 89.9, 1.0, True),          # polar lon-compression, ~194m
        (89.95, 0.0, 89.95, 3.0, True),        # extreme polar compression, ~291m
        (0.0, 179.98, 0.0, -179.98, True),     # antimeridian at the equator, ~4.5km
        (40.0, 0.0, 40.0, 0.0, True),          # identity (0m)
        (40.0, 0.0, 40.0, 0.5, False),         # ~42.7km: comfortably outside
        (40.0, 0.0, 40.0, 1.0, False),         # ~85km: outside
    ]
    for lat1, lon1, lat2, lon2, must_emit in cases:
        got = stage.derive(0, [
            {"icao": "A", "lat": lat1, "lon": lon1},
            {"icao": "B", "lat": lat2, "lon": lon2},
        ])
        ref = _haversine_ref(lat1, lon1, lat2, lon2)
        if must_emit:
            assert got, f"MISSED close pair (true dist {ref:.0f}m): ({lat1},{lon1})-({lat2},{lon2})"
            assert abs(got[0]["dist"] - ref) <= 1.0, (got, ref)
        else:
            assert not got, f"emitted far pair (true dist {ref:.0f}m)"
        sym = _haversine_ref(lat2, lon2, lat1, lon1)
        assert abs(ref - sym) < 1e-9          # symmetry
        stage.prev_dist.clear()
    assert _haversine_ref(40.0, 0.1, 40.0, 0.1) == 0.0  # identity
    # the reviewer's state-TTL vector: derive, close in, long gap, closer
    # still -> the post-gap closing MUST be False (stale prev evicted)
    stage = DerivationStage()
    def pos(dlon):
        return [{"icao": "A", "lat": 40.0, "lon": 0.0},
                {"icao": "B", "lat": 40.0, "lon": dlon}]
    first = stage.derive(0, pos(0.10))
    assert first and first[0]["closing"] is False
    mid = stage.derive(5000, pos(0.05))
    assert mid and mid[0]["closing"] is True, "in-horizon closing must hold"
    post_gap = stage.derive(605_000, pos(0.04))
    assert post_gap and post_gap[0]["closing"] is False, "stale prev_dist used after TTL"

_selfcheck()

# ------------------------------------------------------------- the driver

class AdsbDriver:
    """stream_driver's epoch contract + one derivation stage upstream."""

    def __init__(self):
        self.sess = s.Session(build_rules(), facts={Pair: [], Alert: []})
        self.sess.fire()                  # arm the certified epoch shape (D-242)
        self.derive = DerivationStage()
        self.wal = []                     # RAW epochs; replay re-derives
        self.outputs = []
        self.epoch_no = 0
        self.clock = 0
        self.derived_log = []             # per-epoch derived Pair rows (for the scenario twin)

    def consume(self, advance_to, raw_rows, label=""):
        self.wal.append({"advance_to": advance_to, "raw": raw_rows, "label": label})
        if advance_to > self.clock:
            self.sess.advance(advance_to - self.clock)
            self.clock = advance_to
        pairs = self.derive.derive(advance_to, raw_rows)
        self.derived_log.append(pairs)
        for row in pairs:
            self.sess.insert_row(Pair, row)
        res = self.sess.fire()
        for a in res.derived["Alert"].to_pylist():
            self.outputs.append(
                {"epoch": self.epoch_no, "clock": advance_to,
                 "kind": a["kind"], "key": a["key"], "dist": a["dist"]}
            )
        self.epoch_no += 1
        return pairs

def scripted_feed():
    """AC1/AC2 head-on at lat 40 (20km -> 12km -> 4.5km -> 3km); AC3 far."""
    def ac(icao, lon):
        return {"icao": icao, "lat": 40.0, "lon": lon}
    return [
        (0,     [ac("AC1", -0.1170), ac("AC2", 0.1170), ac("AC3", 5.0)], "20km apart"),
        (5000,  [ac("AC1", -0.0702), ac("AC2", 0.0702), ac("AC3", 5.1)], "12km, closing"),
        (10000, [ac("AC1", -0.0263), ac("AC2", 0.0263), ac("AC3", 5.2)], "4.5km -> converging"),
        (15000, [ac("AC1", -0.0176), ac("AC2", 0.0176), ac("AC3", 5.3)], "3km -> SUSTAINED"),
    ]

def run(feed, tag):
    d = AdsbDriver()
    print(f"\n=== {tag} ===")
    for advance_to, rows, label in feed:
        pairs = d.consume(advance_to, rows, label)
        desc = ", ".join(f"{p['key']} {p['dist']}m{' closing' if p['closing'] else ''}"
                         for p in pairs) or "no candidates"
        print(f"  t={advance_to:>6}  {label:<22} derived: {desc}")
    for o in d.outputs:
        print(f"    [epoch {o['epoch']} @ t={o['clock']}] {o['kind'].upper()} "
              f"{o['key']} at {o['dist']}m")
    return d

if __name__ == "__main__":
    print("seine_rs", s.__version__, "  ADS-B two-plane demo (derivation: seine_rs.derive)")
    prod = run(scripted_feed(), "LIVE")
    rep = run([(e["advance_to"], e["raw"], e["label"]) for e in prod.wal],
              "REPLAY FROM RAW WAL (re-derives)")
    print("\nDETERMINISM (raw WAL -> re-derive -> same alerts):",
          prod.outputs == rep.outputs)
