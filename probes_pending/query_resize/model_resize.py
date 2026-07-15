#!/usr/bin/env python3
"""The >96-key query-index RESIZE mechanism — executable spec + checker.

Bryan's premise (2026-07-14): Drools 9 requires Java 11+, so hash order
cannot vary on rebucketing — the chain reversal past 96 keys is
deterministic, pinnable Drools behavior. CONFIRMED: oracle output is
byte-identical across runs (3x md5), every input is a spec-fixed VALUE
hash (Long.hashCode / String.hashCode — never identity hash), and the
resize algorithm lives in the pinned jar itself
(drools-core org.drools.core.util.AbstractHashTable).

THE PINNED MECHANISM (every component source-anchored in drools-core
9.44.0.Final AND confirmed by live-table dumps — QueryIndexDump graft +
the SEINE_RESIZE_TRACE shadow):

1. LIFO FLUSH — staged right inserts prepend (TupleSetsImpl.addInsert:
   insertFirst = tuple) and the flush walks getInsertFirst
   (PhreakJoinNode.doRightInserts), so keys enter the index table in
   REVERSE arrival order.
2. BULK PRE-SIZE — a staged batch of MORE THAN 32 inserts first calls
   ensureCapacity(N) (PhreakJoinNode.java:133): if size+N exceeds
   threshold (0.75*capacity), capacity doubles from its current value
   until >= size+N, in ONE resize call. On first population the table
   is EMPTY, so this resize moves no chains — nothing reverses. (This
   is why a plain 100-key batch shows NO reversal: it lands in a
   len-256 table pre-sized while empty.)
3. HEAD-INSERT — each new distinct key's TupleList is inserted at the
   HEAD of its bucket chain (TupleIndexHashTable.getOrCreate).
4. INCREMENTAL RESIZE WITH CHAIN REVERSAL — getOrCreate:
   `if (size++ >= threshold) resize(2*len)`: post-add, at the insert
   where pre-add size >= 0.75*capacity. AbstractHashTable.resize walks
   each old chain HEAD->TAIL and HEAD-INSERTS into the new table, so
   entries that stay in one bucket come out REVERSED. Buckets never
   merge (new index = hash & (2len-1); one old bucket splits into two).
   Observed live: RESIZE 256->512 OLD b45[780 949 1186] ->
   NEW b45[1186 780] + b301[949].
5. EMISSION — unbound-unification query rows = the REVERSE of the
   full-table iteration (slots ascending, chains head->tail;
   FieldIndexHashTableFullIterator + downstream reversal).

Hash pipeline (already pinned, D-050): key_hash = JDK6 supplemental
rehash of (seed*31 + java value hash); seed from extractor indexes;
bucket = hash & (capacity-1). Capacity starts at 128, threshold 96.

Checker: regenerates the 19 recon scenarios' inputs, reads oracle
outputs (cargo run -q -p seine-harness -- oracle scenarios/*.json),
and asserts the model reproduces every row order. 19/19 at pin time.

Usage:
  cargo run -q -p seine-harness -- oracle \
      scenarios/probes/pr_rz_*.json 2>/dev/null > /tmp/rz_out.json
  python3 probes_pending/query_resize/model_resize.py /tmp/rz_out.json
"""
import json
import os
import sys

M32 = 0xFFFFFFFF
HERE = os.path.dirname(os.path.abspath(__file__))


def jh(n):
    u = n & 0xFFFFFFFFFFFFFFFF
    return (u ^ (u >> 32)) & M32


def rehash(h):
    h &= M32
    h ^= ((h >> 20) ^ (h >> 12))
    h &= M32
    return (h ^ (h >> 7) ^ (h >> 4)) & M32


def kh(v, seed=994):
    """seed 994 = the D-053 seed for a single-field index on B(k)."""
    return rehash((seed * 31 + jh(v)) & M32)


def rows(arrival):
    """The complete mechanism, components 1-5 above."""
    seq = list(arrival)[::-1]                       # 1. LIFO flush
    cap, size = 128, 0
    slots = [[] for _ in range(cap)]

    def transfer(newcap):
        nonlocal cap, slots
        ns = [[] for _ in range(newcap)]
        for chain in slots:
            for v in chain:                          # 4. head->tail walk,
                ns[kh(v) & (newcap - 1)].insert(0, v)  # head-insert: reversal
        cap, slots = newcap, ns

    if len(seq) > 32:                                # 2. bulk pre-size
        need = size + len(seq)
        if need > (cap * 3) // 4:
            newcap = cap * 2
            while newcap < need:
                newcap *= 2
            transfer(newcap)
    for v in seq:
        slots[kh(v) & (cap - 1)].insert(0, v)        # 3. head-insert
        presize = size
        size += 1
        if presize >= (cap * 3) // 4:                # 4. post-add resize
            transfer(cap * 2)
    full = [v for c in slots for v in c]
    return full[::-1]                                # 5. reversed emission


def lcg_shuffle(xs):
    xs = list(xs)
    s = 12345
    for i in range(len(xs) - 1, 0, -1):
        s = (s * 1103515245 + 12345) % (1 << 31)
        j = s % (i + 1)
        xs[i], xs[j] = xs[j], xs[i]
    return xs


def inputs():
    fam = json.load(open(os.path.join(HERE, "families.txt")))
    fs, fp, fd, fil = (fam["fam_same"], fam["fam_split"],
                       fam["fam_deep"], fam["fillers"])
    out = {}
    for K in (6, 90, 96, 97, 100, 130, 192, 193, 200, 300):
        out[f"rz_asc_{K}"] = list(range(1, K + 1))
    for K in (97, 130, 200):
        out[f"rz_shuf_{K}"] = lcg_shuffle(range(1, K + 1))
    out.update({
        "rz_chain_same": fs + fil[:94],
        "rz_chain_split": fp + fil[:94],
        "rz_trigger_join": fs[:3] + fil[:93] + [fs[3]] + fil[93:96],
        "rz_chain_inter": fil[:48] + fs[:3] + fil[48:94] + fs[3:],
        "rz_second_resize": fd + fs + fp + fil[:184],
        "rz_sub96_chain": fs + fil[:80],
    })
    return out


def main():
    oracle = {}
    for path in sys.argv[1:]:
        txt = open(path).read()
        dec = json.JSONDecoder()
        i = 0
        while i < len(txt):
            while i < len(txt) and txt[i].isspace():
                i += 1
            if i >= len(txt):
                break
            obj, i = dec.raw_decode(txt, i)
            oracle[obj["scenario"]] = [
                r["$v"]["fields"]["value"]
                for r in obj["result"]["queries"][0]["rows"]
            ]
    bad = 0
    checked = 0
    for name, ks in inputs().items():
        if name not in oracle:
            print(f"{name:<18} SKIP (no oracle output)")
            continue
        checked += 1
        good = rows(ks) == oracle[name]
        bad += not good
        print(f"{name:<18} {'MATCH' if good else 'DIFF'}")
    print(f"{'FAIL' if bad else 'OK'}: {checked - bad}/{checked} match")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
