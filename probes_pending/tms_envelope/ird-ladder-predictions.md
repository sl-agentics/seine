# I-RD 9902 duplication-ladder predictions (logged BEFORE the runs)

Target: the 9902 finals-only divergence — multi-handle EqualityKey
bookkeeping (graft_targets/ird/README + the banked dump's final
state). The identity law's STATIC face: Drools's WM is
handle-granular; the engine's store is value-keyed and dedups.

Dump facts driving the predictions (9902 finals, 3×-stable):
- `JUSTIFIED fhs[@4+@8+@14+@20+] lfh=@4` — stated inserts of a
  justified key's value APPEND WM-visible siblings; the key label
  STAYS JUSTIFIED (textbook "stated overrides justified" does NOT
  show in the label).
- `STATED fhs[@5+@10+@15+]` — repeated stated inserts (external +
  RHS mixed) coexist per-handle, each WM-visible.
- 4048 F1 (`STATED fhs[@5!@3+]`) — justified-onto-stated is the
  ASYMMETRIC direction: the belief sibling is NOT WM-visible
  (pending_vals held for the unstage).
- Justified-onto-justified folds into ONE handle with n deps (4048
  @4 bs[n=2], 9902 @4 bs[n=3]) — pinned by both dumps, no cell.
- 9902 @6/@7 point at a justified key whose fhs dropped them —
  multi-epoch bookkeeping noise; the ladder stays SINGLE-EPOCH.

## The rungs (T0/T1 vocabulary; observer counts handles)

Observable per rung: ROBS firing count on value "v" (one firing per
WM-visible handle) + finals multiplicity of "v". SdDump 3× on L2/L3
for the key-status detail (JUSTIFIED-vs-STATED label, fhs marks)
that harness counts cannot see.

| rung | shape | oracle pred | engine pred |
|------|-------|-------------|-------------|
| ird_l1_stated_x3 | external stated ×3, no TMS use | ROBS(v)=3, finals ×3 | 1, ×1 |
| ird_l2_stated_onto_justified | RJ insertLogical(v); RS1+RS2 RHS-stated insert(v) | ROBS(v)=3, finals ×3 | 1, ×1 |
| ird_l3_justified_onto_stated | stated v initial; RJ insertLogical(v) | ROBS(v)=1, finals ×1 | 1, ×1 |
| ird_l4_stated_rhs_onto_external | stated v initial; RS1+RS2 RHS insert(v) | ROBS(v)=3, finals ×3 | 1, ×1 |

SdDump predictions: L2 key label JUSTIFIED with fhs=[belief+,
stated+, stated+] lfh=belief bs[n=1]; L3 key label STATED with
fhs=[belief!, stated+] and pending_vals held.

## Outcome → conclusion

- Per predictions ⇒ the static face pinned: WM-visible handle count
  = (all stated handles) + (belief handle iff the key was born
  justified); the belief sibling on a stated-born key is non-WM.
  The engine fix (if ported) = per-handle multiplicity in the
  store/TMS key model, not the executor.
- L1=3 but L4=1 (or vice versa) ⇒ external-vs-RHS route matters —
  new discriminator needed on the staging path.
- L2=2 ⇒ the stated insert REPLACES/upgrades rather than appends
  (textbook behavior) — 9902's multiplicity then needs a different
  source (epoch machinery); rebuild toward the epoch shape.
- L3=2 ⇒ the 4048 F1 non-WM sibling read is wrong; recheck the
  splitter's b-cells (their unstage premise rides on it).
- L1≠3 ⇒ even the TMS-free identity store duplicates differently;
  the engine's dedup is wrong at a shallower altitude than TMS.

Oracle bar: 3× identity-stable (harness counts); SdDump 3× byte-
stable on L2/L3. Engine runs diagnostic.

## Round 2 (logged BEFORE the L5/L6 runs): the break/re-justify rungs

L1-L4 RESULT (see ird-ladder-results.md): oracle matched all four
predictions (3/3/1/3) — but the ENGINE MATCHED THE ORACLE on every
rung. The banked "engine's value-keyed store dedups" read is WRONG
as a general statement. The ACTUAL 9902 divergence (measured): 14/14
firings, finals differ by EXACTLY ONE T1(false,true) — oracle 7
handles, engine 6. The value is the one whose key DIED AND WAS
REBORN across epoch 1 (the @6/@7 orphans + the fresh @12+ key).

New reading to split: when a JUSTIFIED-status key holding stated
siblings loses its LAST justification (belief set empties):
- **P-ORPHAN (oracle per the dump)**: the key dies entirely; the
  stated handles are ORPHANED alive in WM (dropped from any key's
  fhs); a LATER re-justification of the value starts a FRESH key
  whose justified handle is WM-VISIBLE (the L2-born-justified rule).
- **P-SURVIVE (suspected engine)**: the key survives as STATED
  holding the siblings; a later re-justification takes the L3
  justified-onto-stated route — non-WM sibling, pending_vals — so
  ONE FEWER WM handle. Exactly the observed 9902 delta.

| rung | shape | oracle pred | engine pred |
|------|-------|-------------|-------------|
| ird_l5_break_orphan | J1+s1+s2, then RKILL deletes J1's premise (belief empties); no re-justify | finals(v)=2, ROBS(v)=2 (belief handle dies, its queued act cancels — ordinary retraction) | same (2/2) — sanity rung, both readings agree |
| ird_l6_break_rejustify | as L5, then premise B arrives and RJ2 re-justifies v | finals(v)=3, ROBS(v)=3 (fresh key ⇒ WM-visible justified handle) | finals(v)=2, ROBS(v)=2 (L3 sibling route) |

If L6 comes out NON-divergent (engine==oracle==3): the in-epoch
RHS-delete break isn't the 9902 break — build L7 with the 9902-
faithful 2-epoch UPDATE-driven break (alpha-breaking update path).
If L6 oracle=2: P-ORPHAN is wrong — the oracle key survived the
break in the minimal shape; re-read the 9902 dump's epoch-1
boundary (the orphaning may key on the UPDATE path or epoch
boundary specifically).

## Round 3 (logged BEFORE the x1 run): the unification cell

L5/L6 RESULTS: exactly as predicted (L5 2/2 both; L6 oracle 3/3 vs
engine 2/2). The L6 SdDump (3×-stable) shows the key EMPTY at
FIRING 4 (the orphaning, directly observed) and the fresh
`JUSTIFIED fhs[@8+]` at FIRING 5.

The splitter round could not separate two phrasings of the dynamic
law: P-ORIGIN (unstage-BORN handles' deletes skip act-cancel) vs
P-MAP-ABSENCE (deletes of any handle Drools has DROPPED from TMS
bookkeeping skip act-cancel) — in the splitter's reachable space
they were extensionally equal. The BREAK-ORPHAN is a second way to
make a TMS-dropped WM handle, so it splits them:

| cell | shape | P-MAP-ABSENCE | P-ORIGIN |
|------|-------|---------------|----------|
| ird_x1_orphan_del | L5 base (one stated sibling s1); after the break orphans s1, RD deletes it; ROBS queued | ROBS(v)=1 (**survives**) | ROBS(v)=0 (cancels) |

Engine: 0 either way (no survive machinery).

- x1 FIRE ⇒ **THE SEVENTH LAW (the ORPHAN law)**: Drools's TMS
  drops handles from bookkeeping at unstage and at key-death;
  dropped handles behave as plain non-TMS WM objects — their
  deletes skip the TMS-mediated queued-act cancel (the dynamic
  face) and their values re-key fresh (the static face). ONE
  mechanism, both faces; the envelope closes.
- x1 cancel ⇒ the dynamic face is genuinely origin-keyed
  (unstage-specific); the static face stands alone; two laws.
