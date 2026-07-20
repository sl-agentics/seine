#!/usr/bin/env python3
"""D-357 verifier: the mass-unblock wave-order law (oracle side).

Candidate law (extracted from the full m123 + xf_fz_141421_123 wave
tables, D-356b continuation): the oracle never parks the post-setup
join tuples at the subnet not -- they accumulate as staged lefts at
the R4-subnet join (lazy segment schedule: that segment evaluates only
when R4's executor pops, post-delete). The wave order is the staged
ACCUMULATION order; the delete's rightDel phase precedes leftIns, so
the blocker is gone when the staged lefts flow through. Then the
agenda's dyn-salience STABLE sort.

Batch structure = join1's flush schedule: one batch per evaluation of
a join1-sharing rule with pending staged facts (plus the t=0 initial
sweep). Within a batch with new-fact list N (drain order: external
batches LIFO, RHS batches FIFO, merged batches concatenated in firing
order) over old facts O (as per-batch blocks, most-recent-batch-first,
block-internal in that batch's drain order):

  B_batch = [ (a,b) for a in N for b in N ++ O ]      (leftIns rows)
         ++ [ (a,b) for b in N for a in O ]           (rightIns-born)

B = concat of batches (FIFO). Wave = stable_sort(B, key=-dynsal).

The script hardcodes the two cells' timelines (facts + batch
boundaries read from the certified pre-wave firing logs) and checks
the predicted wave against the observed oracle wave EXACTLY.
"""
import sys

def build_batch(N, O_blocks):
    """N: new facts in drain order. O_blocks: older batches,
    most-recent-first, each in its drain order. Returns pair list."""
    O = [f for blk in O_blocks for f in blk]
    rows = [(a, b) for a in N for b in N + O]
    rins = [(a, b) for b in N for a in O]
    return rows + rins

def wave(batches, alive, dynsal):
    """batches: list of (N, O_blocks). alive: set of surviving facts.
    Returns stable-salience-sorted pair wave (dead pairs dropped)."""
    B = []
    for N, O_blocks in batches:
        B.extend(build_batch(N, O_blocks))
    B = [p for p in B if p[0] in alive and p[1] in alive]
    return sorted(B, key=lambda p: -dynsal(p))

def check(name, predicted, observed):
    ok = predicted == observed
    print(f"{name}: {'PASS' if ok else 'FAIL'} ({len(observed)} pairs)")
    if not ok:
        for i, (p, o) in enumerate(zip(predicted, observed)):
            if p != o:
                print(f"  first divergence at wave[{i}]: "
                      f"predicted {p} observed {o}")
                break
        if len(predicted) != len(observed):
            print(f"  length: predicted {len(predicted)} "
                  f"observed {len(observed)}")
    return ok

F0 = {}
def sal(p):
    return F0[p[0]] - F0[p[1]]

# ---- m123 ----------------------------------------------------------
# T1 handles: 1=(-4,beta) deleted, 2=(2,ab); R1 gen-1: 5=beta,6=a;
# gen-2: 7=beta,8=a. Ext batch drains LIFO: [2,1]. No join1-sharing
# rule evaluates between R1's firings (R0 sal -10 pops post-wave) ->
# G1+G2 MERGE into one batch, drain = RHS-FIFO per firing, firing
# order: [5,6] ++ [7,8]. t=0 sweep flushes S alone.
F0 = {1: -4, 2: 2, 5: 11, 6: 11, 7: 11, 8: 11}
S = [2, 1]
G12 = [5, 6, 7, 8]
m123_batches = [(S, []), (G12, [S])]
m123_alive = {2, 5, 6, 7, 8}
m123_pred = wave(m123_batches, m123_alive, sal)

m123_obs = [
    (5,2),(6,2),(7,2),(8,2),                                  # sal 9
    (2,2),(5,5),(5,6),(5,7),(5,8),(6,5),(6,6),(6,7),(6,8),    # sal 0
    (7,5),(7,6),(7,7),(7,8),(8,5),(8,6),(8,7),(8,8),
    (2,5),(2,6),(2,7),(2,8),                                  # sal -9
]

ok1 = check("m123", m123_pred, m123_obs)

# ---- xf_fz_141421_123 ----------------------------------------------
# T1 handles: 1=(9,b), 3=(10,alpha), 4=(2,ab), 6=(-4,beta) deleted;
# gen-1: 21=beta,22=a; gen-2: 23=beta,24=a. Ext LIFO: [6,4,3,1].
# R3 (shares join1) evaluates in the gb passes between R1's firings
# -> S, G1, G2 are three separate batches.
F0 = {1: 9, 3: 10, 4: 2, 6: -4, 21: 11, 22: 11, 23: 11, 24: 11}
S = [6, 4, 3, 1]
G1 = [21, 22]
G2 = [23, 24]
w_batches = [(S, []), (G1, [S]), (G2, [G1, S])]
w_alive = {1, 3, 4, 21, 22, 23, 24}
w_pred = wave(w_batches, w_alive, sal)

w_obs = [
    (21,4),(22,4),(23,4),(24,4),                              # sal 9
    (3,4),                                                    # sal 8
    (1,4),                                                    # sal 7
    (21,1),(22,1),(23,1),(24,1),                              # sal 2
    (3,1),(21,3),(22,3),(23,3),(24,3),                        # sal 1
    (4,4),(3,3),(1,1),                                        # sal 0
    (21,21),(21,22),(22,21),(22,22),
    (23,23),(23,24),(23,21),(23,22),(24,23),(24,24),(24,21),(24,22),
    (21,23),(22,23),(21,24),(22,24),
    (1,3),(3,21),(3,22),(3,23),(3,24),                        # sal -1
    (1,21),(1,22),(1,23),(1,24),                              # sal -2
    (4,1),                                                    # sal -7
    (4,3),                                                    # sal -8
    (4,21),(4,22),(4,23),(4,24),                              # sal -9
]

ok2 = check("xf_fz_141421_123", w_pred, w_obs)

# ---- probe cells (D-357 rounds 1-2; oracle 3x stable each) ---------
# p357a: m123 with base T1(11,ab) — all pairs sal 0, merged batches.
F0 = {1: -4, 2: 11, 5: 11, 6: 11, 7: 11, 8: 11}
p357a_pred = wave([([2, 1], []), ([5, 6, 7, 8], [[2, 1]])], m123_alive, sal)
p357a_obs = [
    (2,2),(5,5),(5,6),(5,7),(5,8),(5,2),(6,5),(6,6),(6,7),(6,8),(6,2),
    (7,5),(7,6),(7,7),(7,8),(7,2),(8,5),(8,6),(8,7),(8,8),(8,2),
    (2,5),(2,6),(2,7),(2,8),
]
ok3 = check("p357a", p357a_pred, p357a_obs)

# p357b/c: + R3b (T1(f1=="beta") x T1(f1=="a")) — DIFFERENT alpha
# constraints -> its own beta node, NOT join1: no batch split.
# Registered predictions MISSED (assumed a split); the law with the
# correct partition ([S],[G12]) fits both EXACTLY.
F0 = {1: -4, 2: 2, 5: 11, 6: 11, 7: 11, 8: 11}
p357b_pred = wave([([2, 1], []), ([5, 6, 7, 8], [[2, 1]])], m123_alive, sal)
ok4 = check("p357b (merged partition)", p357b_pred, m123_obs)
F0 = {1: -4, 2: 11, 5: 11, 6: 11, 7: 11, 8: 11}
p357c_pred = wave([([2, 1], []), ([5, 6, 7, 8], [[2, 1]])], m123_alive, sal)
ok5 = check("p357c (merged partition)", p357c_pred, p357a_obs)

# p357d: + R3c (the EXACT join1 prefix, salience 5) — genuinely
# shares join1; fires between R1's firings -> batches [S],[G1],[G2].
F0 = {1: -4, 2: 2, 5: 11, 6: 11, 7: 11, 8: 11}
split = [([2, 1], []), ([5, 6], [[2, 1]]), ([7, 8], [[5, 6], [2, 1]])]
p357d_pred = wave(split, m123_alive, sal)
p357d_obs = [
    (5,2),(6,2),(7,2),(8,2),
    (2,2),(5,5),(5,6),(6,5),(6,6),
    (7,7),(7,8),(7,5),(7,6),(8,7),(8,8),(8,5),(8,6),(5,7),(6,7),(5,8),(6,8),
    (2,5),(2,6),(2,7),(2,8),
]
ok6 = check("p357d", p357d_pred, p357d_obs)

# p357e: p357d + all-11 base — the O-block discriminator.
F0 = {1: -4, 2: 11, 5: 11, 6: 11, 7: 11, 8: 11}
p357e_pred = wave(split, m123_alive, sal)
p357e_obs = [
    (2,2),(5,5),(5,6),(5,2),(6,5),(6,6),(6,2),(2,5),(2,6),
    (7,7),(7,8),(7,5),(7,6),(7,2),(8,7),(8,8),(8,5),(8,6),(8,2),
    (5,7),(6,7),(2,7),(5,8),(6,8),(2,8),
]
ok7 = check("p357e", p357e_pred, p357e_obs)

# ---- ablations: every convention must be load-bearing --------------
# Each variant must FAIL on at least one cell.
def expect_fail(name, pred, obs):
    ok = pred == obs
    print(f"  ablation {name}: {'still fits (BAD)' if ok else 'refuted (good)'}")
    return not ok

print("ablations:")
ab = True

# (a) m123 with G1/G2 split (per-firing batches, no merge)
F0 = {1: -4, 2: 2, 5: 11, 6: 11, 7: 11, 8: 11}
p = wave([([2, 1], []), ([5, 6], [[2, 1]]), ([7, 8], [[5, 6], [2, 1]])],
         m123_alive, sal)
ab &= expect_fail("m123 split G1/G2", p, m123_obs)

# (b) witness with G1/G2 merged
F0 = {1: 9, 3: 10, 4: 2, 6: -4, 21: 11, 22: 11, 23: 11, 24: 11}
p = wave([([6, 4, 3, 1], []), ([21, 22, 23, 24], [[6, 4, 3, 1]])],
         w_alive, sal)
ab &= expect_fail("witness merge G1/G2", p, w_obs)

# (c) external batch FIFO instead of LIFO
p = wave([([1, 3, 4, 6], []), ([21, 22], [[1, 3, 4, 6]]),
          ([23, 24], [[21, 22], [1, 3, 4, 6]])], w_alive, sal)
ab &= expect_fail("ext FIFO", p, w_obs)

# (d) RHS batch LIFO instead of FIFO
p = wave([([6, 4, 3, 1], []), ([22, 21], [[6, 4, 3, 1]]),
          ([24, 23], [[22, 21], [6, 4, 3, 1]])], w_alive, sal)
ab &= expect_fail("RHS LIFO", p, w_obs)

# (e) rightIns-born before leftIns rows
def build_batch_rins_first(N, O_blocks):
    O = [f for blk in O_blocks for f in blk]
    rins = [(a, b) for b in N for a in O]
    rows = [(a, b) for a in N for b in N + O]
    return rins + rows

B = []
for N, O_blocks in [([6, 4, 3, 1], []), ([21, 22], [[6, 4, 3, 1]]),
                    ([23, 24], [[21, 22], [6, 4, 3, 1]])]:
    B.extend(build_batch_rins_first(N, O_blocks))
B = [q for q in B if q[0] in w_alive and q[1] in w_alive]
p = sorted(B, key=lambda q: -sal(q))
ab &= expect_fail("rIns-born first", p, w_obs)

# (b2) witness with S+G1 merged (tests that S-separation is forced;
# the sal -1 group's (1,3) < (3,21) is the discriminator)
p = wave([([6, 4, 3, 1, 21, 22], []),
          ([23, 24], [[6, 4, 3, 1, 21, 22]])], w_alive, sal)
ab &= expect_fail("witness merge S+G1", p, w_obs)

# (f) old blocks oldest-first in the row walk — undetectable in the
# witness (cross-block pairs never share a salience group there);
# p357e discriminates: (7,5),(7,6) before (7,2).
F0 = {1: -4, 2: 11, 5: 11, 6: 11, 7: 11, 8: 11}
p = wave([([2, 1], []), ([5, 6], [[2, 1]]),
          ([7, 8], [[2, 1], [5, 6]])], m123_alive, sal)
ab &= expect_fail("O-blocks oldest-first (p357e)", p, p357e_obs)
F0 = {1: 9, 3: 10, 4: 2, 6: -4, 21: 11, 22: 11, 23: 11, 24: 11}

# (g) row walk = old memory before new facts (N after O)
def build_batch_old_first(N, O_blocks):
    O = [f for blk in O_blocks for f in blk]
    rows = [(a, b) for a in N for b in O + N]
    rins = [(a, b) for b in N for a in O]
    return rows + rins

B = []
for N, O_blocks in [([6, 4, 3, 1], []), ([21, 22], [[6, 4, 3, 1]]),
                    ([23, 24], [[21, 22], [6, 4, 3, 1]])]:
    B.extend(build_batch_old_first(N, O_blocks))
B = [q for q in B if q[0] in w_alive and q[1] in w_alive]
p = sorted(B, key=lambda q: -sal(q))
ab &= expect_fail("row walk old-first", p, w_obs)

print(f"ablations all refuted: {ab}")
sys.exit(0 if all([ok1, ok2, ok3, ok4, ok5, ok6, ok7, ab]) else 1)
