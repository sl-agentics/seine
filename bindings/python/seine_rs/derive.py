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
    ``lat2``/``lon2``; degrees; ints widen exactly; NaN/inf raise —
    a NaN would otherwise cast to 0 meters, the strongest possible
    false convergence signal). Returns the input columns plus
    ``out``: Int64 meters, rounded half away from zero —
    bit-compatible with the retired polars stage.
    """
    return _haversine(data, lat1=lat1, lon1=lon1, lat2=lat2, lon2=lon2,
                      out=out)


def pair_candidates(data, id="id", lat="lat", lon="lon", radius_m=25_000.0):
    """Candidate pairs from one position table (``id``/``lat``/``lon``).

    ``a < b`` dedup over the cross join, then a SOUND metric-space
    prune whose contract is COMPLETENESS: no pair whose true haversine
    distance is <= ``radius_m`` is ever dropped. A prune, not the
    exact test — false positives are expected; run :func:`haversine`
    on the output for true distances. With theta = radius_m/EARTH_R:
    |dlat| <= theta; pairs reachable across a pole (colatitude sum <=
    theta) admit regardless of longitude; otherwise the wrapped lon
    delta is bounded by the spherical-cap limit
    asin(sin theta / cos(max |lat|)), skipped entirely when the cap
    reaches a pole. Comparisons are inclusive (a coincident pair
    admits even at radius_m=0).

    Preconditions: ids must be UNIQUE (the ``a < b`` dedup means a
    duplicated id never pairs with itself — such pairs are silently
    absent) and coordinates FINITE (NaN/inf raise; they would
    otherwise decay into false convergence signals downstream).

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
    function of the raw epoch sequence. Epoch ``ts`` must be
    MONOTONIC: a ``ts`` earlier than any timestamp held in ``state``
    raises (silently computing flags against future-stamped state is
    the alternative; the error replays deterministically). Appends
    ``out`` (bool): true iff the key was seen within the TTL horizon
    at a strictly greater distance. Rows update the state in row
    order. An entry aged exactly ``ttl_ms`` still counts; one epoch
    older is swept.
    """
    return _closing(state, ts, data, key=key, dist=dist, ttl_ms=ttl_ms,
                    out=out)
