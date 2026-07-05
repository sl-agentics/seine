#!/usr/bin/env python3
"""8-puzzle solved by recursive backward chaining on the Seine engine.

The canonical Prolog goal-search, run on the certified Drools-subset
engine (DECISIONS.md D-049..D-057):

  - the STATE SPACE is facts: MoveF(src, dst) edges over board states
    (9-char strings, 0 = blank) within the search ball, GoalF(state),
    and a structural depth chain Dec(d, d-1) + ZeroF(0) — the subset has
    no arithmetic, so the depth bound decrements through facts, exactly
    like arithmetic-free Prolog;

  - the SEARCH is a recursive query (Phase Q1: recursion + unification +
    backtracking over or-branches):

        query reach(String $s, long $d)
            ( GoalF($s;) and ZeroF($d;) )
            or
            ( Dec($d, $d1;) and MoveF($s, $n;) and reach($n, $d1;) )
        end

    reach($s, $d) proves "the goal is EXACTLY $d moves from $s". The
    evaluator backtracks through every move sequence; exact depth makes
    every proof a geodesic (a length-d walk to a state d away cannot
    detour), so a unique-shortest-path scramble yields exactly one proof
    per step;

  - the SOLUTION PATH is extracted by forward rules that PULL the query
    as a condition (Phase Q2, the query-as-condition bridge):

        rule Step1 when
            P0($s : state)
            MoveF(src == $s, $n : dst)
            ?reach($n, 4;)
        then insert(new P1($n)); end

    each firing commits one move whose successor still reaches the goal
    in the remaining budget; the chained P1..Pd facts ARE the path.

Run:            python3 demo/eight_puzzle.py [--moves N] [--seed N]
Differential:   python3 demo/eight_puzzle.py --diff   (needs the oracle)

Scramble range: 5-6 moves. At 7 the move ball exceeds 96 states and the
demo refuses: a query hash index above 96 distinct keys crosses the
resize boundary that is fenced OUT of the certified subset (D-051) —
the guard fails loudly rather than run off the certified map.

The frozen default instance also lives in the differential corpus as
scenarios/demo/eight_puzzle.json — certified against real Drools on
every `make diff`.
"""

import argparse
import json
import os
import random
import subprocess
import sys
from collections import deque

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOAL = "123456780"

# D-051: a query hash index resizes above 96 distinct keys — out of the
# certified subset. The move ball must stay under it.
MAX_INDEX_KEYS = 96


def neighbors(state):
    """Legal successor states (slide a tile into the blank)."""
    i = state.index("0")
    r, c = divmod(i, 3)
    out = []
    for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        nr, nc = r + dr, c + dc
        if 0 <= nr < 3 and 0 <= nc < 3:
            j = nr * 3 + nc
            s = list(state)
            s[i], s[j] = s[j], s[i]
            out.append("".join(s))
    return out


def bfs_dist(src):
    dist = {src: 0}
    q = deque([src])
    while q:
        s = q.popleft()
        for n in neighbors(s):
            if n not in dist:
                dist[n] = dist[s] + 1
                q.append(n)
    return dist


def geodesic_count(s, d, dist_to_goal):
    """Number of length-d walks from s to GOAL (= geodesics when
    d == dist_to_goal[s])."""
    if d == 0:
        return 1 if s == GOAL else 0
    return sum(
        geodesic_count(n, d - 1, dist_to_goal)
        for n in neighbors(s)
        if dist_to_goal.get(n, 99) <= d - 1
    )


def pick_scramble(moves, seed):
    """Random-walk `moves` from GOAL until the endpoint is exactly
    `moves` away with a UNIQUE shortest path (one proof per step)."""
    rng = random.Random(seed)
    dist_to_goal = bfs_dist(GOAL)
    while True:
        s, prev = GOAL, None
        for _ in range(moves):
            opts = [n for n in neighbors(s) if n != prev]
            prev, s = s, rng.choice(opts)
        if dist_to_goal[s] == moves and geodesic_count(s, moves, dist_to_goal) == 1:
            return s


def build_scenario(scramble, depth):
    dist_from_scramble = bfs_dist(scramble)
    # Edges the exact-depth search can traverse: walks leave states at
    # most depth-1 moves from the scramble.
    srcs = sorted(s for s, d in dist_from_scramble.items() if d <= depth - 1)
    assert len(srcs) < MAX_INDEX_KEYS, "move ball exceeds the D-051 index wall"
    moves = [(s, n) for s in srcs for n in neighbors(s)]

    types = [
        {"name": "MoveF", "fields": [{"name": "src", "type": "String"},
                                     {"name": "dst", "type": "String"}]},
        {"name": "GoalF", "fields": [{"name": "state", "type": "String"}]},
        {"name": "Dec", "fields": [{"name": "from", "type": "i64"},
                                   {"name": "to", "type": "i64"}]},
        {"name": "ZeroF", "fields": [{"name": "z", "type": "i64"}]},
    ]
    for k in range(depth + 1):
        types.append({"name": f"P{k}", "fields": [{"name": "state", "type": "String"}]})
    types.append({"name": "Solution", "fields": [{"name": "state", "type": "String"}]})

    facts = [{"type": "MoveF", "fields": {"src": s, "dst": n}} for s, n in moves]
    facts.append({"type": "GoalF", "fields": {"state": GOAL}})
    facts += [{"type": "Dec", "fields": {"from": d, "to": d - 1}} for d in range(1, depth + 1)]
    facts.append({"type": "ZeroF", "fields": {"z": 0}})
    facts.append({"type": "P0", "fields": {"state": scramble}})

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

    return {
        "name": "eight_puzzle",
        "types": types,
        "facts": facts,
        "drl": drl,
        # the same search, standalone (Phase Q1): is the scramble exactly
        # `depth` moves out? and what reaches the goal in one move?
        "queries": [
            {"call": "reach", "args": [scramble, depth]},
            {"call": "reach", "args": [None, 1]},
        ],
    }


def board(state):
    rows = [state[i * 3:(i + 1) * 3].replace("0", "·") for i in range(3)]
    return [" ".join(r) for r in rows]


def print_path(path):
    boards = [board(s) for s in path]
    for chunk in range(0, len(boards), 6):
        group = boards[chunk:chunk + 6]
        sep = "    " if chunk + 6 >= len(boards) else "    "
        for line in range(3):
            print(sep.join(b[line] for b in group))
        print()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--moves", type=int, default=5, help="scramble length (default 5)")
    ap.add_argument("--seed", type=int, default=11, help="scramble seed")
    ap.add_argument("--diff", action="store_true",
                    help="differential-check the scenario against the Drools oracle")
    args = ap.parse_args()

    scramble = pick_scramble(args.moves, args.seed)
    scenario = build_scenario(scramble, args.moves)
    out_path = os.path.join(REPO, "target", "eight_puzzle_demo.json")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(scenario, f, indent=1)

    n_moves = sum(1 for f in scenario["facts"] if f["type"] == "MoveF")
    print(f"8-puzzle: scramble {scramble} is exactly {args.moves} moves from {GOAL}")
    print(f"state space handed to the engine: {n_moves} MoveF edges; "
          f"depth bound as Dec facts {args.moves}..1; goal + zero facts")
    print()

    cmd = ["cargo", "run", "-q", "-p", "seine-harness", "--",
           "diff" if args.diff else "run", out_path]
    res = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
    if args.diff:
        print(res.stdout.strip())
        sys.exit(0 if res.returncode == 0 else 1)
    if res.returncode != 0:
        print(res.stdout, res.stderr)
        sys.exit(1)
    result = json.loads(res.stdout)["result"]

    # Path = the chain of Pk facts the Step rules derived.
    path = [scramble]
    for k in range(1, args.moves + 1):
        nxt = [f["fields"]["state"] for f in result["facts"] if f["type"] == f"P{k}"]
        assert len(nxt) == 1, f"expected a unique step-{k} state, got {nxt}"
        path.append(nxt[0])
    assert path[-1] == GOAL

    print("the Step rules pulled ?reach per candidate move and committed the path:")
    print()
    print_path(path)
    print("firing log (each Step firing = one backward-chaining proof):")
    for f in result["firings"]:
        if f["rule"] == "Solved":
            print(f"  Solved: {path[-1]}")
            continue
        pk = next(m["fields"] for m in f["matches"] if m["type"].startswith("P"))
        mv = next(m["fields"] for m in f["matches"] if m["type"] == "MoveF")
        print(f"  {f['rule']}: {pk['state']} -> {mv['dst']}   (?reach({mv['dst']!r}, ...) proved)")
    q0 = result["queries"][0]
    print(f"\nstandalone Q1 check: reach({scramble!r}, {args.moves}) "
          f"has {len(q0['rows'])} proof(s) — the unique geodesic")


if __name__ == "__main__":
    main()
