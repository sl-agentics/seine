#!/usr/bin/env python3
"""Gate extracted baseline candidates through the Seine differential stack.

Stages:
  1. ENGINE PARSE GATE   `seine-harness run` — a parse/compile error routes the
     candidate OUT-OF-SUBSET (reason recorded; that routing is Deliverable-2
     data, not failure).
  2. ORACLE RUN          real Drools executes the scenario; oracle errors mean
     the TRANSLATION is invalid (quarantine-review) — e.g. DRL that only
     compiled against the Java bean, not the declare-based type.
  3. DRIFT CHECK         oracle firing count vs the JUnit-recorded
     expect_fire_count. Mismatch = translation drift -> quarantine-review
     (NOT a faithfulness bug; the adaptation changed behavior).
  4. DIFFERENTIAL        `seine-harness diff` on survivors. PASS -> baseline
     member. FAIL -> candidate FAITHFULNESS BUG (report, do not fix).

Usage: baseline_gate.py --candidates DIR --accept DIR --outdir DIR
Writes: <outdir>/gate_report.tsv, moves accepted scenarios into --accept,
        leaves everything else in place with routing recorded.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys

HARNESS = ['cargo', 'run', '-q', '-p', 'seine-harness', '--']


def run_ndjson(cmd, paths, timeout=1800):
    out = subprocess.run(cmd + paths, capture_output=True, text=True, timeout=timeout)
    lines = []
    for ln in out.stdout.splitlines():
        ln = ln.strip()
        if ln.startswith('{'):
            try:
                lines.append(json.loads(ln))
            except json.JSONDecodeError:
                pass
    return lines, out.returncode, out.stderr


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--candidates', required=True)
    ap.add_argument('--accept', required=True)
    ap.add_argument('--outdir', required=True)
    ap.add_argument('--batch', type=int, default=400)
    args = ap.parse_args()

    files = sorted(
        os.path.join(args.candidates, f)
        for f in os.listdir(args.candidates) if f.endswith('.json'))
    os.makedirs(args.accept, exist_ok=True)
    os.makedirs(args.outdir, exist_ok=True)
    by_name = {}
    for f in files:
        with open(f) as fh:
            sc = json.load(fh)
        by_name[sc['name']] = {'path': f, 'scenario': sc, 'route': None, 'detail': ''}

    # ---- stage 1: engine parse gate ------------------------------------
    print(f'[gate] stage 1: engine parse gate over {len(files)} candidates')
    for i in range(0, len(files), args.batch):
        chunk = files[i:i + args.batch]
        results, _, err = run_ndjson(HARNESS + ['run'], chunk)
        got = set()
        for r in results:
            name = r.get('scenario')
            got.add(name)
            if name not in by_name:
                continue
            if 'error' in r:
                by_name[name]['route'] = 'out-of-subset'
                by_name[name]['detail'] = r['error'][:200]
        for f in chunk:
            with open(f) as fh:
                nm = json.load(fh)['name']
            if nm not in got and by_name[nm]['route'] is None:
                by_name[nm]['route'] = 'engine-crash'
                by_name[nm]['detail'] = err[-200:] if err else 'no output'

    in_subset = [v['path'] for v in by_name.values() if v['route'] is None]
    print(f'[gate] in-subset after parse gate: {len(in_subset)}')

    # ---- stage 2: oracle run -------------------------------------------
    print('[gate] stage 2: oracle run')
    oracle_res = {}
    for i in range(0, len(in_subset), args.batch):
        chunk = in_subset[i:i + args.batch]
        results, _, _ = run_ndjson(HARNESS + ['oracle'], chunk, timeout=3600)
        for r in results:
            oracle_res[r.get('scenario')] = r

    for v in by_name.values():
        if v['route'] is not None:
            continue
        name = v['scenario']['name']
        r = oracle_res.get(name)
        if r is None:
            v['route'] = 'oracle-missing'
        elif 'error' in r:
            v['route'] = 'translation-invalid'
            v['detail'] = str(r['error'])[:200]

    # ---- stage 3: fire-count drift check -------------------------------
    print('[gate] stage 3: drift check')
    for v in by_name.values():
        if v['route'] is not None:
            continue
        name = v['scenario']['name']
        expect = v['scenario'].get('provenance', {}).get('expect_fire_count')
        if expect is None:
            continue
        firings = oracle_res[name].get('result', {}).get('firings', [])
        if len(firings) != expect:
            v['route'] = 'translation-drift'
            v['detail'] = f'junit expected {expect} fires, oracle produced {len(firings)}'

    survivors = [v['path'] for v in by_name.values() if v['route'] is None]
    print(f'[gate] survivors into differential: {len(survivors)}')

    # ---- stage 4: differential -----------------------------------------
    passed, failed = [], []
    for i in range(0, len(survivors), args.batch):
        chunk = survivors[i:i + args.batch]
        out = subprocess.run(HARNESS + ['diff'] + chunk, capture_output=True,
                             text=True, timeout=3600)
        for ln in out.stdout.splitlines():
            if ln.startswith('PASS '):
                passed.append(ln[5:].strip())
            elif ln.startswith('FAIL '):
                failed.append(ln[5:].strip())
    for v in by_name.values():
        if v['route'] is not None:
            continue
        name = v['scenario']['name']
        if name in passed:
            v['route'] = 'baseline-pass'
        elif name in failed:
            v['route'] = 'DIVERGENCE'
        else:
            v['route'] = 'diff-missing'

    # ---- emit -----------------------------------------------------------
    report = os.path.join(args.outdir, 'gate_report.tsv')
    with open(report, 'w') as f:
        for name in sorted(by_name):
            v = by_name[name]
            f.write(f'{name}\t{v["route"]}\t{v["detail"]}\n')
    for v in by_name.values():
        if v['route'] == 'baseline-pass':
            shutil.copy(v['path'], args.accept)

    counts = {}
    for v in by_name.values():
        counts[v['route']] = counts.get(v['route'], 0) + 1
    for k in sorted(counts, key=counts.get, reverse=True):
        print(f'{counts[k]:5d}  {k}')
    print(f'report: {report}')
    if counts.get('DIVERGENCE'):
        print('!! DIVERGENCES found — faithfulness-bug candidates (report, do not fix):')
        for name in sorted(by_name):
            if by_name[name]['route'] == 'DIVERGENCE':
                print(f'   {name}')


if __name__ == '__main__':
    main()
