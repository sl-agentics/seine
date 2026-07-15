#!/usr/bin/env python3
"""8-puzzle by recursive backward chaining — WHEEL-NATIVE port.

A rewrite of demo/eight_puzzle.py that runs entirely on the installed
`seine_rs` wheel (Layer 1 raw DRL + queries), with NO cargo / seine-harness
/ Rust-repo dependency. The certified logic is identical to the original
(DECISIONS.md D-049..D-057):

  - STATE SPACE as facts: MoveF(src,dst) edges within the search ball,
    GoalF(state), a depth chain Dec(d,d-1) + ZeroF(0) (the subset has no
    arithmetic, so the depth bound decrements through facts);
  - SEARCH as a recursive query reach($s,$d) proving "goal is EXACTLY $d
    moves from $s" — recursion + unification + backtracking over or-branches;
  - SOLUTION PATH extracted by forward rules that PULL the query as a
    condition (?reach), each firing committing one move; the chained
    P1..Pd facts ARE the path.

Differences from the harness version:
  - types/facts/rules go straight into s.run() (schemas from @fact classes);
  - the Pk path chain is read from Result.derived instead of the harness JSON;
  - the firing log is reconstructed from the `firings` audit table
    (grouped by `seq`, matched facts read from `values_json`);
  - no --diff mode (that differential-checks against the Drools oracle via
    cargo — inherently a repo/harness capability, not on the wheel surface).

Solvability gate: before searching, the board is checked for the 3x3
parity invariant (inversion count must be even) — half of all 9!
configurations are unreachable from the goal, and there is no reason to
spend a backward-chaining search on one that provably has no solution.

Run:  python3 demo/eight_puzzle_wheel.py [--moves N] [--seed N]
      python3 demo/eight_puzzle_wheel.py --start 213456780   # unsolvable (odd parity)
      python3 demo/eight_puzzle_wheel.py --start 123456708   # solvable, 1 move
"""
import argparse
import dataclasses
import json
import random
from collections import deque

import seine_rs as s
from seine_rs import fact

GOAL = "123456780"


# ---- pure-python geometry (unchanged from the original) ----------------

def neighbors(state):
    """Legal successor states (slide a tile into the blank)."""
    i = state.index("0")
    r, c = divmod(i, 3)
    out = []
    for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        nr, nc = r + dr, c + dc
        if 0 <= nr < 3 and 0 <= nc < 3:
            j = nr * 3 + nc
            t = list(state)
            t[i], t[j] = t[j], t[i]
            out.append("".join(t))
    return out


def bfs_dist(src):
    dist = {src: 0}
    q = deque([src])
    while q:
        st = q.popleft()
        for n in neighbors(st):
            if n not in dist:
                dist[n] = dist[st] + 1
                q.append(n)
    return dist


def geodesic_count(st, d, dist_to_goal):
    """Number of length-d walks from st to GOAL (= geodesics when
    d == dist_to_goal[st])."""
    if d == 0:
        return 1 if st == GOAL else 0
    return sum(
        geodesic_count(n, d - 1, dist_to_goal)
        for n in neighbors(st)
        if dist_to_goal.get(n, 99) <= d - 1
    )


def pick_scramble(moves, seed):
    """Random-walk `moves` from GOAL until the endpoint is exactly `moves`
    away with a UNIQUE shortest path (one proof per step)."""
    rng = random.Random(seed)
    dist_to_goal = bfs_dist(GOAL)
    while True:
        st, prev = GOAL, None
        for _ in range(moves):
            opts = [n for n in neighbors(st) if n != prev]
            prev, st = st, rng.choice(opts)
        if dist_to_goal[st] == moves and geodesic_count(st, moves, dist_to_goal) == 1:
            return st


# ---- solvability gate (pure-Python parity invariant) -------------------

def inversions(state):
    """Count inversions in the tile sequence (blank excluded): pairs that
    appear out of natural order."""
    tiles = [int(c) for c in state if c != "0"]
    return sum(1 for i in range(len(tiles)) for j in range(i + 1, len(tiles))
               if tiles[i] > tiles[j])


def is_solvable(state):
    """3x3 (odd width): reachable from the goal IFF inversions are even.
    Exactly half of the 9! permutations qualify."""
    return inversions(state) % 2 == 0


def parse_board(text):
    if len(text) != 9 or sorted(text) != list("012345678"):
        raise SystemExit(f"--start must be a permutation of digits 0-8 (0=blank), got {text!r}")
    return text


# ---- fact types (schemas come from @fact classes) ----------------------

@fact
class MoveF:
    src: str
    dst: str

@fact
class GoalF:
    state: str

@fact
class Dec:
    d: int      # positional in DRL; name is free
    d1: int

@fact
class ZeroF:
    z: int


def make_state_class(name):
    """Dynamically declare a P-k / Solution single-field state fact."""
    cls = dataclasses.make_dataclass(name, [("state", str)])
    return fact(cls)


def build(scramble, depth):
    """Return (rules_drl, facts_map) for the wheel to run."""
    dist_from_scramble = bfs_dist(scramble)
    srcs = sorted(st for st, d in dist_from_scramble.items() if d <= depth - 1)
    moves = [(st, n) for st in srcs for n in neighbors(st)]

    P = [make_state_class(f"P{k}") for k in range(depth + 1)]
    Solution = make_state_class("Solution")

    facts = {
        MoveF: [MoveF(st, n) for st, n in moves],
        GoalF: [GoalF(GOAL)],
        Dec:   [Dec(d, d - 1) for d in range(1, depth + 1)],
        ZeroF: [ZeroF(0)],
        P[0]:  [P[0](scramble)],
        Solution: [],
    }
    for k in range(1, depth + 1):
        facts[P[k]] = []   # schema-only declaration (inserted by the Step rules)

    drl = (
        "query reach(String $s, long $d)\n"
        "    ( GoalF($s;) and ZeroF($d;) )\n"
        "    or\n"
        "    ( Dec($d, $d1;) and MoveF($s, $n;) and reach($n, $d1;) )\n"
        "end\n\n"
    )
    for k in range(depth):
        remaining = depth - k - 1
        drl += (
            f"rule Step{k + 1}\n"
            f"when\n"
            f"    P{k}($s : state)\n"
            f"    MoveF(src == $s, $n : dst)\n"
            f"    ?reach($n, {remaining};)\n"
            f"then\n"
            f"    insert(new P{k + 1}($n));\n"
            f"end\n\n"
        )
    drl += (
        f"rule Solved\nwhen\n    P{depth}($s : state)\nthen\n"
        f"    insert(new Solution($s));\nend\n"
    )
    return drl, facts, P


def board(state):
    rows = [state[i * 3:(i + 1) * 3].replace("0", "·") for i in range(3)]
    return [" ".join(r) for r in rows]


def print_path(path):
    boards = [board(st) for st in path]
    for chunk in range(0, len(boards), 6):
        group = boards[chunk:chunk + 6]
        for line in range(3):
            print("    ".join(b[line] for b in group))
        print()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--moves", type=int, default=5, help="scramble length (default 5); ignored with --start")
    ap.add_argument("--seed", type=int, default=11, help="scramble seed")
    ap.add_argument("--start", type=str, default=None,
                    help="explicit 9-digit board (0=blank), e.g. 213456780; overrides --moves/--seed")
    args = ap.parse_args()

    scramble = parse_board(args.start) if args.start is not None else pick_scramble(args.moves, args.seed)

    print("seine_rs", s.__version__, " 8-puzzle (wheel-native, DRL query + ?reach)")

    # --- solvability gate: runs BEFORE the engine sees the board ------
    inv = inversions(scramble)
    print(f"start {scramble}: {inv} inversion(s) -> {'even' if inv % 2 == 0 else 'odd'} parity")
    if not is_solvable(scramble):
        print(f"UNSOLVABLE: no move sequence reaches {GOAL} from this board "
              f"(odd inversion parity). Not attempting a search.")
        return
    print("SOLVABLE: proceeding to the backward-chaining search.\n")

    # exact search depth: the board's true distance from the goal
    depth = bfs_dist(GOAL)[scramble] if args.start is not None else args.moves
    if depth == 0:
        print(f"{scramble} is already the goal — 0 moves.")
        return

    drl, facts, P = build(scramble, depth)
    n_moves = len(facts[MoveF])
    print(f"scramble {scramble} is exactly {depth} moves from {GOAL}")
    print(f"state space: {n_moves} MoveF edges; depth bound Dec {depth}..1; goal + zero")
    print()

    res = s.run(drl, facts)

    # Reconstruct ONE shortest path from the derived Pk sets. Every state
    # in Pk is exactly depth-k from goal and on some geodesic, so greedily
    # following a neighbour into the next level always reaches the goal —
    # robust even when the board has several shortest solutions.
    levels = [{scramble}] + [{r["state"] for r in res.derived[f"P{k}"].to_pylist()}
                             for k in range(1, depth + 1)]
    path, cur = [scramble], scramble
    for k in range(1, depth + 1):
        nxts = [n for n in neighbors(cur) if n in levels[k]]
        assert nxts, f"search produced no successor at step {k} (derived P{k}={levels[k]})"
        cur = nxts[0]
        path.append(cur)
    assert path[-1] == GOAL, f"path did not reach goal: {path}"

    print("the Step rules pulled ?reach per candidate move and committed the path:")
    print()
    print_path(path)

    # firing log, reconstructed from the audit table (grouped by seq)
    by_seq = {}
    for row in res.firings.to_pylist():
        by_seq.setdefault(row["seq"], []).append(row)
    print("firing log (each Step firing = one backward-chaining proof):")
    for seq, rows in sorted(by_seq.items()):
        rule = rows[0]["rule"]
        if rule == "Solved":
            continue
        pk = next(json.loads(r["values_json"]) for r in rows if r["type"].startswith("P"))
        mv = next(json.loads(r["values_json"]) for r in rows if r["type"] == "MoveF")
        print(f"  {rule}: {pk['state']} -> {mv['dst']}   (?reach({mv['dst']!r}, ...) proved)")
    print(f"  Solved: {GOAL}")

    # standalone Q1 check via session.query (recursion + backtracking)
    sess = s.Session(drl, facts)
    sess.fire()
    q = sess.query("reach", scramble, depth)
    gc = geodesic_count(scramble, depth, bfs_dist(GOAL))
    kind = "the unique geodesic" if gc == 1 else f"{gc} distinct shortest solutions"
    print(f"\nstandalone Q1: reach({scramble!r}, {depth}) has {len(q)} proof(s) — {kind}")


if __name__ == "__main__":
    main()

