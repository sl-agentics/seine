"""seine_rs.derive — the derivation plane's Rust kernels.

The two-plane contract (docs/derivation-plane.md): Drools
semantics in the match, dataframe semantics in the data. These are
pure columnar functions over Arrow data — anything ``run()`` accepts
(``__arrow_c_stream__`` tables or dicts of column lists) in, a
``seine_rs.Table`` out — producing honest FIELDS upstream of
assertion; the certified match grammar never grows arithmetic. Their
oracle is an independent pure-python reference + the property battery
in bindings/tests/test_derive.py; the Drools oracle has no opinion
about column math.

Epoch contract: derivation runs inside the epoch, upstream of
assertion, as a deterministic function of (raw batch, caller-owned
state) — the WAL stores RAW epochs and replay re-derives identically.

The v1 kernel set (ADS-B-driven; geometry per the design doc's
round-27 hardening):

- ``haversine`` — great-circle distance columns -> Int64 meters.
- ``pair_candidates`` — cross-join + metric-space candidate prune
  over one position table (wrapped lon delta, cos(lat)-scaled
  threshold saturating to latitude-only at the poles).
- ``closing`` — TTL'd decreasing-distance flag; state is the CALLER's
  dict, swept by epoch timestamp on every call.
"""
from seine_rs._native import (
    derive_closing as _closing,
    derive_haversine as _haversine,
    derive_pair_candidates as _pair_candidates,
)

__all__ = ["EARTH_R", "closing", "haversine", "pair_candidates"]

EARTH_R = 6_371_000.0  # meters — the kernels' sphere radius


def haversine(data, lat1="lat1", lon1="lon1", lat2="lat2", lon2="lon2",
              out="dist_m"):
    """Columnar great-circle distance on the EARTH_R sphere.

    ``data`` needs four numeric columns (named by ``lat1``/``lon1``/
    ``lat2``/``lon2``; degrees; ints widen exactly). Returns the input
    columns plus ``out``: Int64 meters, rounded half away from zero —
    bit-compatible with the demo's polars stage.
    """
    return _haversine(data, lat1=lat1, lon1=lon1, lat2=lat2, lon2=lon2,
                      out=out)


def pair_candidates(data, id="id", lat="lat", lon="lon", radius_m=25_000.0):
    """Candidate pairs from one position table (``id``/``lat``/``lon``).

    ``a < b`` dedup over the cross join, then the METRIC-space prune
    (docs/derivation-plane.md, exactly): |dlat| < radius_m/111320; the lon delta WRAPS
    across the antimeridian and its threshold scales by cos(mean lat)
    (clipped to 1e-6, capped at 180 deg) — saturating to a
    latitude-only prune at the poles. A prune, not the exact test:
    run :func:`haversine` on the output for true distances.

    Output columns: ``{id}_a, {lat}_a, {lon}_a, {id}_b, {lat}_b,
    {lon}_b, key`` (``"{a}|{b}"``), in cross-join order (a-major).
    """
    return _pair_candidates(data, id=id, lat=lat, lon=lon, radius_m=radius_m)


def closing(state, ts, data, key="key", dist="dist_m", ttl_ms=60_000,
            out="closing"):
    """Stateful decreasing-distance flag keyed by ``key``.

    ``state`` is the CALLER's dict (``key -> (dist, epoch_ts)``) —
    hold it in your driver and rebuild it on WAL replay; nothing hides
    in module globals. Entries older than ``ttl_ms`` are swept FIRST
    on every call (pass the raw epoch timestamp as ``ts``), so call
    once per epoch even when the batch is empty: eviction stays a pure
    function of the raw epoch sequence. Appends ``out`` (bool): true
    iff the key was seen within the TTL horizon at a strictly greater
    distance. Rows update the state in row order.
    """
    return _closing(state, ts, data, key=key, dist=dist, ttl_ms=ttl_ms,
                    out=out)
