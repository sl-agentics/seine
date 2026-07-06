#!/usr/bin/env python3
"""P1c replica: subnetwork not/exists counting node — list-convention
elimination against the sn_* order probes (D-088).

Certified mechanics are FIXED (Staged prepend/head-first consumption,
merge/append_into_pending, first-sink append + later-sink peer flip,
join child creation orders, terminal FIFO, agenda salience/decl order,
eager per-flush vs lazy accumulation). FREE dimensions cover only the
NEW hops: RIA transfer direction, counting-node child staging per phase,
tip child-delete staging order, and the external-origin variant.

Pipeline modeled (the level-1 probe family):
  P-LIA -> [subnet head: P x A] -> [subnet tip: PA x B] -> RIA ->
  sn node rights;  P-LIA -> sn node lefts (level-1: identical copies).
Sharing: multiple rules' sn nodes hang off ONE subnet chain (RIA multi-
sink); twin rules share the SAME sn node with terminal sinks in decl
order (first preserved, later flipped) — the certified D-037 model.
"""
import itertools, json, sys

# ---------------------------------------------------------------- staged
class Staged:
    def __init__(s):
        s.ins, s.upd, s.dele = [], [], []
    def is_empty(s):
        return not (s.ins or s.upd or s.dele)
    def take(s):
        o = Staged(); o.ins, o.upd, o.dele = s.ins, s.upd, s.dele
        s.ins, s.upd, s.dele = [], [], []
        return o
    def add_ins(s, t):
        if t in s.upd or t in s.ins: return
        s.ins.insert(0, t)
    def add_upd(s, t):
        if t in s.ins or t in s.upd or t in s.dele: return
        s.upd.insert(0, t)
    def add_del(s, t):
        if t in s.ins:
            s.ins.remove(t); return
        if t in s.upd: s.upd.remove(t)
        if t in s.dele: return
        s.dele.insert(0, t)

def merge_into_pending(pending, trg):
    for t in reversed(trg.dele): pending.add_del(t)
    for t in reversed(trg.upd):
        if t in pending.ins:
            pending.ins.remove(t); pending.ins.insert(0, t); continue
        if t in pending.upd: pending.upd.remove(t)
        if t in pending.dele: continue
        pending.upd.insert(0, t)
    for t in reversed(trg.ins):
        if t in pending.ins: continue
        pending.ins.insert(0, t)
    return pending

def append_into_pending(pending, trg):
    pending.ins.extend(trg.ins); pending.dele.extend(trg.dele); pending.upd.extend(trg.upd)
    return pending

def peer_merge(pending, trg):
    for t in trg.dele: pending.add_del(t)
    for t in trg.upd:
        if t in pending.ins or t in pending.upd or t in pending.dele: continue
        pending.upd.insert(0, t)
    for t in trg.ins:
        if t in pending.ins:
            pending.ins.remove(t); pending.ins.insert(0, t); continue
        pending.ins.insert(0, t)
    return pending

# ---------------------------------------------------------------- machine
class M:
    """One convention-set machine, replaying one probe timeline."""
    def __init__(m, conv, rules):
        # conv: dict of free dimensions
        # rules: list of dicts {name, kind: not|exists|k1|join2-*, sal, decl,
        #        eager, rhs: [(action...)], }
        m.c = conv
        m.rules = rules
        m.P, m.A, m.B = [], [], []      # alpha memberships (fact ids)
        m.head_lefts = []               # P facts in memory order
        m.head_children = []            # T_i tuples: (P,) extended -> ('T', p)
        m.tip_lefts = []                # T tuples in memory order
        m.tip_children = []             # S tuples ('S', p) creation order
        m.head_s0 = Staged(); m.head_sr = Staged()
        m.tip_sl = Staged(); m.tip_sr = Staged()
        m.sn = {}                       # per sn-node-id state
        m.queues = {r['name']: [] for r in rules}
        m.fired = []
        m.k1_s0 = {r['name']: Staged() for r in rules if r['kind'] == 'k1'}
        # sn nodes: group rules by shared sn node (same kind+decl-adjacent
        # identical structure share ONE node with multiple term sinks).
        m.sn_nodes = []                 # {id, kind, sinks:[rule names], sl:Staged, sr:Staged, s0:Staged, lefts:[], matches:{}, child:{}}
        shared = {}
        for r in rules:
            if r['kind'] not in ('not', 'exists'): continue
            key = (r['kind'], r.get('share_key', r['name']))
            if key in shared:
                m.sn_nodes[shared[key]]['sinks'].append(r['name'])
            else:
                shared[key] = len(m.sn_nodes)
                m.sn_nodes.append(dict(kind=r['kind'], sinks=[r['name']],
                                       sl=Staged(), sr=Staged(), s0=Staged(),
                                       lefts=[], matches={}, child={}))
        # RIA sinks in build (decl) order = m.sn_nodes order
        m.term_pending = {r['name']: Staged() for r in rules}

    # ---------------- WM actions (staging, eager alpha)
    def insert(m, typ, fid, external=False):
        if typ == 'P':
            m.P.append(fid)
            m.head_s0.add_ins(fid)
            for n in m.sn_nodes: n['s0'].add_ins(fid)
        elif typ == 'A':
            m.A.append(fid); m.head_sr.add_ins(fid)
        elif typ == 'B':
            m.B.append(fid); m.tip_sr.add_ins(fid)
        for r in m.rules:
            if r['kind'] == 'k1' and r.get('ptype', 'P') == typ:
                m.k1_s0[r['name']].add_ins(fid)

    def delete(m, typ, fid, external=False):
        if typ == 'B':
            m.B.remove(fid)
            if external and m.c['ext_del'] == 'append':
                if fid in m.tip_sr.ins: m.tip_sr.ins.remove(fid)
                else: m.tip_sr.dele.append(fid)
            else:
                m.tip_sr.add_del(fid)
        elif typ == 'A':
            m.A.remove(fid)
            if external and m.c['ext_del'] == 'append':
                if fid in m.head_sr.ins: m.head_sr.ins.remove(fid)
                else: m.head_sr.dele.append(fid)
            else:
                m.head_sr.add_del(fid)

    # ---------------- node evaluations
    def eval_head(m):
        """P x A join. Level-1: consumes head_s0 (facts) + staged rights."""
        s0 = m.head_s0.take(); sr = m.head_sr.take()
        src = merge_into_pending(Staged(), stage_from(s0))
        trg = Staged()
        # left inserts x full rights
        for p in src.ins:
            m.head_lefts.append(p)
            for a in m.A:
                t = ('T', p)
                m.head_children.append(t)
                trg.add_ins(t)
        # left deletes
        for p in src.dele:
            if p in m.head_lefts: m.head_lefts.remove(p)
            for t in [c for c in m.head_children if c[1] == p]:
                m.head_children.remove(t); trg.add_del(t)
        # right inserts x pre-batch lefts (memory forward)
        for a in sr.ins:
            for p in list(m.head_lefts):
                if ('T', p) not in m.head_children:
                    t = ('T', p)
                    m.head_children.append(t); trg.add_ins(t)
        # right deletes kill children (creation order or reverse — FREE)
        for a in sr.dele:
            kids = [c for c in m.head_children]  # all children involve the sole A
            if m.c['tip_del_walk'] == 'rev': kids = list(reversed(kids))
            for t in kids:
                m.head_children.remove(t); trg.add_del(t)
        return trg

    def eval_tip(m, incoming):
        """TA x B join; incoming = staged lefts from head (already appended)."""
        m.tip_sl = merge_into_pending(m.tip_sl, incoming)
        src = m.tip_sl.take(); sr = m.tip_sr.take()
        trg = Staged()
        for t in src.ins:
            m.tip_lefts.append(t)
            for b in m.B:
                s = ('S', t[1])
                m.tip_children.append(s); trg.add_ins(s)
        for t in src.dele:
            if t in m.tip_lefts: m.tip_lefts.remove(t)
            for s in [c for c in m.tip_children if c[1] == t[1]]:
                m.tip_children.remove(s); trg.add_del(s)
        for b in sr.ins:
            for t in list(m.tip_lefts):
                s = ('S', t[1])
                if s not in m.tip_children:
                    m.tip_children.append(s); trg.add_ins(s)
        for b in sr.dele:
            kids = list(m.tip_children)
            if m.c['tip_del_walk'] == 'rev': kids = list(reversed(kids))
            for s in kids:
                m.tip_children.remove(s); trg.add_del(s)
        return trg

    def ria_propagate(m, trg):
        """Stage subnet tuples into each sn node's rights (first sink
        direct, later sinks same treatment — doRiaNode2 stages per-sink
        copies; direction FREE per kind)."""
        for n in m.sn_nodes:
            tins = trg.ins if m.c['ria_ins'] == 'keep' else list(reversed(trg.ins))
            tdel = trg.dele if m.c['ria_del'] == 'keep' else list(reversed(trg.dele))
            for s in tins:
                if s in n['sr'].dele:  # same-batch del+ins: fold? (c5b says count>=1 holds)
                    pass
                n['sr'].ins.append(s) if m.c['sn_r_stage'] == 'append' else n['sr'].ins.insert(0, s)
            for s in tdel:
                if s in n['sr'].ins:
                    n['sr'].ins.remove(s); continue
                n['sr'].dele.append(s) if m.c['sn_r_stage'] == 'append' else n['sr'].dele.insert(0, s)

    def eval_sn(m, n):
        """The counting machine (phase order per sources)."""
        s0 = n['s0'].take()
        sl = merge_into_pending(n['sl'], stage_from(s0)); n['sl'] = Staged()
        sr_ = Staged(); sr_.ins, sr_.dele, sr_.upd = n['sr'].ins, n['sr'].dele, n['sr'].upd
        n['sr'] = Staged()
        trg = Staged()
        # leftDel
        for p in sl.dele:
            if p in n['lefts']: n['lefts'].remove(p)
            if p in n['child']:
                trg.add_del(n['child'].pop(p))
            n['matches'].pop(p, None)
        # rightIns
        for s in sr_.ins:
            p = s[1]
            n['matches'].setdefault(p, []).append(s)
            if len(n['matches'][p]) == 1:
                if n['kind'] == 'exists':
                    c = ('C', p)
                    n['child'][p] = c
                    (trg.ins.append(c) if m.c['sn_ri_add'] == 'append' else trg.ins.insert(0, c))
                else:
                    if p in n['child']:
                        trg.add_del(n['child'].pop(p))
        # leftIns
        for p in sl.ins:
            n['lefts'].append(p)
            if n['kind'] == 'not' and not n['matches'].get(p):
                c = ('C', p)
                n['child'][p] = c
                (trg.ins.append(c) if m.c['sn_li_add'] == 'append' else trg.ins.insert(0, c))
        # rightUpd: no-op
        # rightDel
        for s in sr_.dele:
            p = s[1]
            lst = n['matches'].get(p)
            if lst and s in lst: lst.remove(s)
            if lst is not None and not lst and p in n['lefts']:
                if n['kind'] == 'exists':
                    if p in n['child']:
                        trg.add_del(n['child'].pop(p))
                else:
                    c = ('C', p)
                    n['child'][p] = c
                    (trg.ins.append(c) if m.c['sn_rd_add'] == 'append' else trg.ins.insert(0, c))
        # leftUpd
        for p in sl.upd:
            if p in n['child']:
                trg.add_upd(n['child'][p])
        return trg

    # ---------------- rule evaluation / agenda
    def evaluate_rule(m, r):
        name = r['name']
        if r['kind'] == 'k1':
            s0 = m.k1_s0[name].take()
            for f in reversed(s0.dele):
                m.queues[name] = [a for a in m.queues[name] if a != f]
            for f in reversed(s0.ins):
                m.queues[name].append(f)
            return
        # CE rules: walk head -> tip -> RIA -> own sn node -> terminal.
        # Shared chain: whichever rule evaluates first claims the batch.
        trg_head = m.eval_head()
        trg_tip = m.eval_tip(trg_head)
        if not trg_tip.is_empty():
            m.ria_propagate(trg_tip)
        node = next(n for n in m.sn_nodes if name in n['sinks'])
        trg = m.eval_sn(node)
        # propagate to terminal sinks: first sink append, later flipped
        for i, sink in enumerate(node['sinks']):
            if i == 0:
                m.term_pending[sink] = append_into_pending(m.term_pending[sink], trg)
            else:
                m.term_pending[sink] = peer_merge(m.term_pending[sink], trg)
        # consume own terminal
        src = m.term_pending[name].take()
        for t in src.dele:
            m.queues[name] = [a for a in m.queues[name] if a != t]
        for t in src.upd:
            if t not in m.queues[name]:
                m.queues[name].append(t)
        for t in src.ins:
            m.queues[name].append(t)

    def flush_eager(m):
        for r in m.rules:
            if r.get('eager'):
                m.evaluate_rule(r)

    def fire_all(m):
        guard = 0
        while True:
            guard += 1
            if guard > 300: raise RuntimeError('loop')
            # eager rules evaluate at EVERY flush incl. the initial one
            m.flush_eager()
            # agenda walk: (sal desc, decl asc); evaluate then fire first
            fired_one = False
            for r in sorted(m.rules, key=lambda r: (-r['sal'], r['decl'])):
                m.evaluate_rule(r)
                if m.queues[r['name']]:
                    act = m.queues[r['name']].pop(0)
                    m.fired.append((r['name'], act))
                    for action in r.get('rhs', []):
                        m.apply_action(action)
                    m.flush_eager()
                    fired_one = True
                    break
            if not fired_one:
                return

    def apply_action(m, action):
        op = action[0]
        if op == 'insert':
            m.ins_ctr = getattr(m, 'ins_ctr', 0) + 1
            m.insert(action[1], f'{action[2]}{m.ins_ctr}')
        elif op == 'delete_first':
            typ = action[1]
            pool = {'A': m.A, 'B': m.B}[typ]
            if pool: m.delete(typ, pool[0])

def stage_from(s0):
    t = Staged(); t.ins = list(s0.ins); t.upd = list(s0.upd); t.dele = list(s0.dele)
    return t

# ---------------------------------------------------------------- probes
def run_probe(conv, probe):
    rules = probe['rules']
    m = M(conv, rules)
    fid = 0
    for typ in probe['facts']:
        fid += 1
        m.insert(typ, f'{typ}{fid}')
    m.fire_all()
    for ep in probe.get('epochs', []):
        for act in ep:
            if act[0] == 'xdel':
                typ = act[1]
                pool = {'A': m.A, 'B': m.B}[typ]
                if pool: m.delete(typ, pool[0], external=True)
            elif act[0] == 'xins':
                fid += 1
                m.insert(act[1], f'{act[1]}{fid}', external=True)
        m.fire_all()
    return [(r, a[1] if isinstance(a, tuple) else a) for (r, a) in m.fired]

PROBES = {
  # name: {facts, rules, epochs, expect: [(rule, Pfact)...] — CE rules only}
  'a3_not': dict(
      facts=['P','P','P','A'],
      rules=[dict(name='R2', kind='not', sal=0, decl=0)],
      expect=[('R2','P1'),('R2','P2'),('R2','P3')]),
  'a3_ex': dict(
      facts=['P','P','P','A','B'],
      rules=[dict(name='R3', kind='exists', sal=0, decl=0)],
      expect=[('R3','P3'),('R3','P2'),('R3','P1')]),
  'b3': dict(
      facts=['P','P','P','A','B'],
      rules=[dict(name='R_del', kind='k1', ptype='B', sal=10, decl=0,
                  rhs=[('delete_first','B')]),
             dict(name='R_not', kind='not', sal=0, decl=1)],
      expect=[('R_del','B5'),('R_not','P1'),('R_not','P2'),('R_not','P3')]),
  'b3x': dict(
      facts=['P','P','A','B'],
      rules=[dict(name='R_del', kind='k1', ptype='B', sal=10, decl=0,
                  rhs=[('delete_first','B')]),
             dict(name='R_not', kind='not', sal=0, decl=1)],
      expect=[('R_del','B4'),('R_not','P1'),('R_not','P2')]),
  'b3s': dict(
      facts=['P','P','P','A','B','G'],
      rules=[dict(name='R_not', kind='not', sal=0, decl=0),
             dict(name='R_ex', kind='exists', sal=5, decl=1),
             dict(name='R_del', kind='k1', ptype='G', sal=10, decl=2,
                  rhs=[('delete_first','B')])],
      expect=[('R_del','G6'),('R_not','P1'),('R_not','P2'),('R_not','P3')]),
  'b4': dict(
      facts=['P','P','P','A'],
      rules=[dict(name='R_ins', kind='k1', ptype='A', sal=10, decl=0,
                  rhs=[('insert','B','B9')]),
             dict(name='R_ex', kind='exists', sal=0, decl=1)],
      expect=[('R_ins','A4'),('R_ex','P3'),('R_ex','P2'),('R_ex','P1')]),
  'x3': dict(
      facts=['P','P','P','A','B'],
      rules=[dict(name='R_not', kind='not', sal=0, decl=0)],
      epochs=[[('xdel','B')]],
      expect=[('R_not','P3'),('R_not','P2'),('R_not','P1')]),
  'x4': dict(
      facts=['P','P','P','A','B'],
      rules=[dict(name='R_ex', kind='exists', sal=5, decl=0),
             dict(name='R_not', kind='not', sal=0, decl=1)],
      epochs=[[('xdel','B')]],
      expect=[('R_ex','P3'),('R_ex','P2'),('R_ex','P1'),
              ('R_not','P3'),('R_not','P2'),('R_not','P1')]),
  'x1': dict(
      facts=['P','P','A','B'],
      rules=[dict(name='R_not', kind='not', sal=0, decl=0),
             dict(name='R_ex', kind='exists', sal=5, decl=1)],
      epochs=[[('xdel','B')], [('xins','B')]],
      expect=[('R_ex','P2'),('R_ex','P1'),
              ('R_not','P2'),('R_not','P1'),
              ('R_ex','P2'),('R_ex','P1')]),
  'd1': dict(
      facts=['P','P','P','A'],
      rules=[dict(name='R1', kind='not', sal=0, decl=0, share_key='g'),
             dict(name='R2', kind='not', sal=0, decl=1, share_key='g')],
      expect=[('R1','P1'),('R1','P2'),('R1','P3'),
              ('R2','P3'),('R2','P2'),('R2','P1')]),
  'd3': dict(
      facts=['P','P','A','B','G'],
      rules=[dict(name='R_ex', kind='exists', sal=20, decl=0),
             dict(name='R_del', kind='k1', ptype='G', sal=10, decl=1,
                  rhs=[('delete_first','B')]),
             dict(name='R_not', kind='not', sal=0, decl=2)],
      expect=[('R_ex','P2'),('R_ex','P1'),('R_del','G5'),
              ('R_not','P1'),('R_not','P2')]),
  'b3e': dict(
      facts=['P','P','P','A','B'],
      rules=[dict(name='R_del', kind='k1', ptype='B', sal=10, decl=0,
                  rhs=[('delete_first','B')]),
             dict(name='R_not', kind='not', sal=0, decl=1, eager=True)],
      expect=[('R_del','*'),('R_not','P3'),('R_not','P2'),('R_not','P1')]),
  'x5': dict(
      facts=['A','B'],
      rules=[dict(name='R_not', kind='not', sal=0, decl=0)],
      epochs=[[('xins','P'),('xins','P'),('xins','P'),('xdel','B')]],
      expect=[('R_not','P3'),('R_not','P4'),('R_not','P5')]),
  'c9': dict(
      facts=['G','G','A','B'],
      rules=[dict(name='R_src', kind='k1', ptype='G', sal=10, decl=0,
                  rhs=[('insert','P','PX')]),
             dict(name='R_ex_eager', kind='exists', sal=0, decl=1, eager=True),
             dict(name='R_ex_lazy', kind='exists', sal=-5, decl=2)],
      expect=[('R_src','*'),('R_src','*'),
              ('R_ex_eager','PX1'),('R_ex_eager','PX2'),
              ('R_ex_lazy','PX2'),('R_ex_lazy','PX1')]),
}


# ------------------------------------------------- fork-level-2 (sn_c3)
def run_c3(conv, variant):
    """P(2) x Q(2) prefix join, then group. variant: 'not'|'exists'|'notmid'.
    Facts: P1,P2,Q1,Q2; group over (A,B) with A,B ABSENT for not-variants
    (fires all 4) and (C,D) PRESENT for the exists variant.
    'notmid': P -> group -> Q join (CE mid-rule, D-013 reversal after).
    Returns fired tuple list like ('P1','Q2')."""
    # certified 2-pattern join trg for the initial batch, pinned by j01:
    # terminal order P1Q1, P1Q2, P2Q1, P2Q2 == trg list order consumed
    # head-first (terminal appends).
    pq = [('P1','Q1'), ('P1','Q2'), ('P2','Q1'), ('P2','Q2')]
    if variant in ('not', 'exists'):
        # fork node sinks: subnet head + sn node (order = conv['fork_sink'])
        # subnet chain: no facts for not (children never form) /
        # C,D singletons for exists (every left forms one S tuple).
        if conv['fork_sink'] == 'subnet_first':
            sn_left_src = peer_flip(pq)      # sn node = later sink
            subnet_src = list(pq)            # first sink: appended as-is
        else:
            sn_left_src = list(pq)
            subnet_src = peer_flip(pq)
        if variant == 'not':
            # leftIns walk head->tail, no matches -> children
            trg = []
            for t in sn_left_src:
                add(trg, ('C', t), conv['sn_li_add'])
            return [x[1] for x in trg]       # terminal consumes head-first
        else:
            # subnet chain: head joins C (single), tip joins D (single):
            # walk src head->tail, child prepend per join level (certified)
            lvl1 = []
            for t in subnet_src:
                lvl1.insert(0, t)
            lvl2 = []
            for t in lvl1:
                lvl2.insert(0, t)
            # RIA hop
            sr = list(reversed(lvl2)) if conv['ria_ins'] == 'flip' else list(lvl2)
            sr2 = []
            for s in sr:
                (sr2.append(s) if conv['sn_r_stage'] == 'append' else sr2.insert(0, s))
            # rightIns: 0->1 per left -> child
            trg = []
            for s in sr2:
                add(trg, ('C', s), conv['sn_ri_add'])
            return [x[1] for x in trg]
    else:  # notmid: P -> sn -> join Q
        ps = [('P1',), ('P2',)]
        # level-1: sn node gets LIA copies (unflipped); subnet chain empty
        trg = []
        for t in ps:
            add(trg, ('C', t), conv['sn_li_add'])
        # children [Cx] flow to the Q join: propagation BETWEEN nodes
        # reverses (D-013/D-014: emissions reverse when propagated to the
        # next join = per-entry prepend into its staged lefts)
        qsrc = []
        for c in trg:
            qsrc.insert(0, c)
        out = []
        for c in qsrc:
            for q in ['Q1', 'Q2']:
                out.insert(0, (c[1][0], q))
        return list(reversed(out))           # terminal head-first append
    return []

def peer_flip(lst):
    out = []
    for t in lst:
        out.insert(0, t)
    return out

def add(lst, item, how):
    (lst.append(item) if how == 'append' else lst.insert(0, item))

C3_EXPECT = {
    'not':   [('P1','Q1'), ('P1','Q2'), ('P2','Q1'), ('P2','Q2')],
    'exists':[('P2','Q2'), ('P2','Q1'), ('P1','Q2'), ('P1','Q1')],
    'notmid':[('P2','Q1'), ('P2','Q2'), ('P1','Q1'), ('P1','Q2')],
}

def main():
    dims = dict(
        ria_ins=['keep','flip'],
        ria_del=['keep','flip'],
        sn_r_stage=['append','prepend'],
        sn_li_add=['prepend','append'],
        sn_ri_add=['prepend','append'],
        sn_rd_add=['prepend','append'],
        tip_del_walk=['fwd','rev'],
        ext_del=['same','append'],
        fork_sink=['subnet_first','sn_first'],
    )
    keys = list(dims)
    survivors = []
    for combo in itertools.product(*(dims[k] for k in keys)):
        conv = dict(zip(keys, combo))
        ok = True
        for variant, want in C3_EXPECT.items():
            if run_c3(conv, variant) != want:
                ok = False; break
        if not ok:
            continue
        for pname, p in PROBES.items():
            try:
                fired = run_probe(conv, p)
            except Exception:
                ok = False; break
            got = [(r, a) for (r, a) in fired]
            want = p['expect']
            # compare only tuples for rules in expect (k1 fire values may
            # differ in fact naming) — normalize: compare rule sequence and
            # the P-suffix for CE rules
            def norm(seq):
                out = []
                for r, a in seq:
                    if isinstance(a, str) and a.startswith('P'):
                        out.append((r, a))
                    else:
                        out.append((r, '*'))
                return out
            if norm(got) != norm(want):
                ok = False; break
        if ok:
            survivors.append(conv)
    print(f'{len(survivors)} survivor(s) of {2**len(keys)}')
    for s in survivors[:10]:
        print(json.dumps(s))

if __name__ == '__main__':
    main()
