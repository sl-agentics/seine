"""Seine streaming-driver demo — the rev-3 'time kicker' design on the Python
surface, respecting the certified epoch contract (D-102 / D-242).

Architecture (from the doc):
  - single-writer engine step: advance(t) -> assert facts -> drain to quiescence
  - epoch record = advance(t) + the facts asserted before the next drain
  - heartbeat  = an epoch that advances with an empty fact set (drives expiry)
  - WAL the epoch stream; replay is byte-for-byte the production path
  - outputs stamped with the epoch AND their justification (the durable why-log),
    drained from the firings audit rather than diffed out of working memory

Domain: live temperature sensor feed -> overheat CEP (temporal correlation +
event expiry). Readings live 6s then age out; a heartbeat is what makes them
expire from time passing alone.

Authored by the external-review Claude Code session (rounds 21-25 of the
black-box campaign); the epoch-sequence twin is byte-checked against the
oracle as scenarios/demo/stream_driver.json.
"""
import seine_rs as s
import json
from collections import defaultdict

# ---------------------------------------------------------------- domain

@s.fact(event=s.Event(timestamp="ts", expires_ms=6000))
class Reading:
    ts: int
    sensor: int
    temp: int

@s.fact
class Alert:
    sensor: int
    kind: str

def build_rules():
    # temporal correlation: two HOT readings (>80) from the SAME sensor within 5s
    spike = s.Rule("double_spike")
    a = spike.when(Reading, Reading.temp > 80)
    spike.when(Reading, Reading.temp > 80, Reading.sensor == a.sensor,
               s.this_after(a, 1, 5000))
    spike.then_insert(Alert, sensor=a.sensor, kind="double_spike")
    return [spike]

# ------------------------------------------------------------- the driver

class StreamDriver:
    """Single-writer consumer of epoch records. Owns the Session exclusively.
    Outputs drained from the firings audit, stamped with epoch + justification."""

    def __init__(self):
        self.sess = s.Session(build_rules(), facts={Reading: [], Alert: []})
        self.sess.fire()          # certified: fire initial state first (arms epoch shape, D-242)
        self.wal = []             # write-ahead log of epoch records
        self.outputs = []         # stamped output events (the why-log)
        self.epoch_no = 0
        self.clock = 0            # absolute pseudo-clock (advance() takes a DELTA)

    def consume(self, advance_to, facts, label=""):
        # WAL first: durability before the state moves
        self.wal.append({"advance_to": advance_to, "facts": facts, "label": label})
        # single-writer engine step: advance -> assert -> drain.
        # advance() is a DELTA; convert the epoch's absolute time into a step.
        if advance_to is not None:
            delta = advance_to - self.clock
            if delta < 0:
                raise ValueError(f"epoch clock went backwards: {advance_to} < {self.clock}")
            if delta > 0:
                self.sess.advance(delta)
                self.clock = advance_to
        for row in facts:
            self.sess.insert_row(Reading, row)
        res = self.sess.fire()
        live = len(res.facts["Reading"].to_pylist())   # readings still in WM (shows expiry)
        self._drain(res, advance_to)
        self.epoch_no += 1
        return live

    def _drain(self, res, clock):
        by_firing = defaultdict(list)
        for row in res.firings.to_pylist():
            by_firing[row["seq"]].append(row)
        alerted = set()
        for _, rows in sorted(by_firing.items()):
            if rows[0]["rule"] != "double_spike":
                continue
            why = [json.loads(r["values_json"]) for r in rows if r["type"] == "Reading"]
            sensor = why[0]["sensor"]
            if sensor in alerted:              # one alert per sensor per epoch
                continue
            alerted.add(sensor)
            self.outputs.append({
                "epoch": self.epoch_no, "clock": clock,
                "detail": f"double_spike sensor={sensor}",
                "why": " + ".join(f"R(t={w['ts']},{w['temp']}°)" for w in why),
            })

# --------------------------------------------------------- scripted feed

def scripted_stream():
    # (advance_to_ms, [Readings], label) — heartbeats carry no facts.
    return [
        (0,     [Reading(ts=0,     sensor=1, temp=70)], "warmup"),
        (1000,  [Reading(ts=1000,  sensor=1, temp=85)], "sensor1 spike #1 (hot)"),
        (2000,  [],                                      "heartbeat"),
        (3000,  [Reading(ts=3000,  sensor=1, temp=88)], "sensor1 spike #2 (<5s) -> ALERT"),
        (4000,  [Reading(ts=4000,  sensor=2, temp=60)], "sensor2 normal reading"),
        (7500,  [],                                      "heartbeat (spike #1 @1s expires)"),
        (9500,  [],                                      "heartbeat (spike #2 @3s expires)"),
        (11000, [Reading(ts=11000, sensor=1, temp=90)], "sensor1 hot again (spikes long gone)"),
    ]

def run(stream, tag):
    d = StreamDriver()
    print(f"\n=== {tag} ===")
    print(f"{'epoch':>5} {'clock':>6} {'live':>5}   event")
    for advance_to, facts, label in stream:
        live = d.consume(advance_to, facts, label)
        print(f"{d.epoch_no-1:>5} {advance_to:>6} {live:>5}   {label}")
    print(f"  --- output events ({len(d.outputs)}), stamped with epoch + why ---")
    for o in d.outputs:
        print(f"    [epoch {o['epoch']} @ t={o['clock']}] ALERT {o['detail']}")
        print(f"        why: {o['why']}")
    return d

if __name__ == "__main__":
    print("seine_rs", s.__version__, "  streaming driver demo")
    print("  ('live' = readings currently in working memory; drops as events expire)")
    prod = run(scripted_stream(), "LIVE PRODUCTION RUN")
    # replay from the WAL must reproduce the outputs bit-for-bit
    replay = [(e["advance_to"], e["facts"], e["label"]) for e in prod.wal]
    rep = run(replay, "REPLAY FROM WAL")
    same = prod.outputs == rep.outputs
    print(f"\n=== DETERMINISM: replay outputs == production outputs?  {same} ===")
    print(f"    WAL = {len(prod.wal)} epoch records, "
          f"{sum(1 for e in prod.wal if not e['facts'])} of them heartbeats")
