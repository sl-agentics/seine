#!/usr/bin/env python3
"""D-359 verifier: the lazy TERM-QUEUE composition law.

The D-357 accumulation law seen at a first-built Term sink of a
MULTI-SINK join (single-sink joins fuse the term into the join's
segment and compose pure insertion-lex — engine already correct
there). The term-surface variant B' = B with ALL drains FIFO (ext
included; p359b's S-block pinned it): per batch, leftIns rows
(a in N, FIFO) walking [N FIFO ++ O blocks most-recent-first,
block-internal FIFO], then rightIns-born (b in N FIFO, a in O
most-recent-first); batches FIFO; equal static salience -> the
queue order IS the firing order (dead pairs dropped).
"""
import sys

def build_batch(N, O_blocks):
    O = [f for blk in O_blocks for f in blk]
    rows = [(a, b) for a in N for b in N + O]
    rins = [(a, b) for b in N for a in O]
    return rows + rins

def termq(batches, alive):
    B = []
    for N, O_blocks in batches:
        B.extend(build_batch(N, O_blocks))
    return [p for p in B if p[0] in alive and p[1] in alive]

def check(name, predicted, observed):
    ok = predicted == observed
    print(f"{name}: {'PASS' if ok else 'FAIL'} ({len(observed)} pairs)")
    if not ok:
        for i, (p, o) in enumerate(zip(predicted, observed)):
            if p != o:
                print(f"  first divergence at [{i}]: predicted {p} observed {o}")
                break
        if len(predicted) != len(observed):
            print(f"  length {len(predicted)} vs {len(observed)}")
    return ok

ok = True

# m123's R0 (oracle; merged G, S separate; h1 deleted).
S, G = [1, 2], [5, 6, 7, 8]
alive = {2, 5, 6, 7, 8}
pred = termq([(S, []), (G, [S])], alive)
obs = [(2,2),
       (5,5),(5,6),(5,7),(5,8),(5,2),(6,5),(6,6),(6,7),(6,8),(6,2),
       (7,5),(7,6),(7,7),(7,8),(7,2),(8,5),(8,6),(8,7),(8,8),(8,2),
       (2,5),(2,6),(2,7),(2,8)]
ok &= check("m123 R0", pred, obs)

# p359b (subnet present, no delete, all alive).
alive = {1, 2, 5, 6, 7, 8}
pred = termq([(S, []), (G, [S])], alive)
obs = [(1,1),(1,2),(2,1),(2,2),
       (5,5),(5,6),(5,7),(5,8),(5,1),(5,2),(6,5),(6,6),(6,7),(6,8),(6,1),(6,2),
       (7,5),(7,6),(7,7),(7,8),(7,1),(7,2),(8,5),(8,6),(8,7),(8,8),(8,1),(8,2),
       (1,5),(2,5),(1,6),(2,6),(1,7),(2,7),(1,8),(2,8)]
ok &= check("p359b R0", pred, obs)

# p359e (h2 deleted instead; no wave).
alive = {1, 5, 6, 7, 8}
pred = termq([(S, []), (G, [S])], alive)
obs = [(1,1),
       (5,5),(5,6),(5,7),(5,8),(5,1),(6,5),(6,6),(6,7),(6,8),(6,1),
       (7,5),(7,6),(7,7),(7,8),(7,1),(8,5),(8,6),(8,7),(8,8),(8,1),
       (1,5),(1,6),(1,7),(1,8)]
ok &= check("p359e R0", pred, obs)

# ablations
def expect_fail(name, pred, obs):
    bad = pred == obs
    print(f"  ablation {name}: {'still fits (BAD)' if bad else 'refuted (good)'}")
    return not bad

print("ablations:")
ab = True
# ext-LIFO S (the WAVE-surface direction) must fail the term surface
alive = {1, 2, 5, 6, 7, 8}
p = termq([([2, 1], []), (G, [[2, 1]])], alive)
obs_b = [(1,1),(1,2),(2,1),(2,2),
         (5,5),(5,6),(5,7),(5,8),(5,1),(5,2),(6,5),(6,6),(6,7),(6,8),(6,1),(6,2),
         (7,5),(7,6),(7,7),(7,8),(7,1),(7,2),(8,5),(8,6),(8,7),(8,8),(8,1),(8,2),
         (1,5),(2,5),(1,6),(2,6),(1,7),(2,7),(1,8),(2,8)]
ab &= expect_fail("ext-LIFO S (p359b)", p, obs_b)
# fully merged (no S split) must fail p359b
p = termq([([1, 2, 5, 6, 7, 8], [])], alive)
ab &= expect_fail("S merged into G (p359b)", p, obs_b)
# pure insertion-lex must fail p359b (it fits the single-sink cells)
p = sorted(termq([(S, []), (G, [S])], alive))
ab &= expect_fail("pure-lex (p359b)", p, obs_b)

print(f"ablations all refuted: {ab}")
sys.exit(0 if ok and ab else 1)
