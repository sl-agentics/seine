# DECISIONS.md — running log

Append-only log of semantics probes, tie-break discoveries, design decisions,
and known limitations. Each checkpoint ends with a handoff note.

---

## 2026-07-03 — Session start, Phase 0

### D-001: Environment
- Java: OpenJDK 21.0.11 (Ubuntu). Maven 3.8.7.
- Rust: 1.96.1 stable, installed via rustup (minimal profile) this session —
  `~/.cargo/bin` must be on PATH.
- **Oracle pinned: Drools `9.44.0.Final`** (pre-seeded in local `~/.m2`; Maven
  Central reachable for missing transitives). Pinned in `oracle/pom.xml` via
  `<drools.version>` property. Locale forced to `en_US` in the runner.

### D-002: Project name
"**Seine**" — a seine is a fishing net, echoing Rete ("net") without using the
Drools trademark (brief §8). Crates: `seine-engine`, `seine-harness`. Name is
local-only for now; re-check crates.io availability before any publish.

### D-003: Canonical result JSON schema (locked — both runners target this forever)
```json
{
  "facts":   [ {"type": "T", "fields": {"a": 1, "b": "x"}}, ... ],
  "firings": [ {"rule": "R", "matches": [ <fact rendering>, ... ]}, ... ]
}
```
- `facts` = final working-memory contents as a **multiset**, canonically sorted
  by rendering (type, then field values); fields serialized with sorted keys.
- `firings` = **ordered** log of rule firings (afterMatchFired). Each entry
  carries the matched facts' renderings, sorted lexicographically *within* the
  entry (so we don't depend on either engine's internal tuple ordering), while
  the firing sequence itself is order-significant.
- Comparison is **semantic, not textual**: comparator parses both JSONs;
  f64 equality is IEEE-754 bit equality; i64 is exact. Java must emit doubles
  with a decimal point (Jackson default) so JSON number types round-trip.

### D-004: Scenario format
```json
{
  "name": "...",
  "types": [ {"name": "Person", "fields": [{"name":"age","type":"i64"}, ...]}, ... ],
  "facts": [ {"type": "Person", "fields": {"age": 30, ...}}, ... ],
  "drl":   "rule ... end"
}
```
- Field lists are **ordered** (arrays, not maps): declared-type constructor
  argument order in generated DRL `declare` blocks follows this order.
- Java runner does NOT codegen Java classes; it prepends generated `declare`
  blocks to the scenario DRL and instantiates via the `FactType` API. This
  keeps the oracle fully data-driven.
- Types: `i64` → long, `f64` → double, `String` → String, `bool` → boolean.

### D-005: Oracle runner design
- Batch-capable from day one (fuzz phases need thousands of cases without JVM
  restart): accepts N scenario paths, emits NDJSON `{"scenario": ..., "result": ...}`
  per line to stdout. Errors surface as `{"scenario":..., "error":...}`.
- Deps: drools-compiler/-core/-kiesession/-mvel/-xml-support + kie-api/-internal,
  Jackson for JSON, slf4j-nop to silence logging. All Drools deps pinned 9.44.0.Final.

### D-006: Oracle verified end-to-end (Phase 0 gate 1) ✅
`p0_trivial_adult` runs through real Drools 9.44.0.Final: DRL compiles (declare
blocks + rule), 2 firings captured, canonical JSON emitted, stderr clean.
- **Observed:** `KieSession.getObjects()` order is nondeterministic across runs
  (bob/alice/carol vs bob/carol/alice) → comparator MUST treat `facts` as a
  multiset. It does; canonicalization lives in the comparator, not the runners.
- **Preliminary observation (NOT yet pinned):** same-rule activations for facts
  all inserted before `fireAllRules()` fired in fact *insertion order*
  (alice→carol). Must be pinned with dedicated Phase 1 probes (multi-rule,
  salience, interleaved insert) before relying on it.
- Environment gotcha: the box had JRE-only Java 21; installed
  `openjdk-21-jdk-headless` via sudo apt for javac.

---

### D-007: Phase 0 walking skeleton GREEN ✅ (done-bar met)
- Rust workspace: `engine/` (seine-engine) + `harness/` (seine-harness).
- Store layout per brief: per-type per-field columnar arenas, global
  insertion-ordered `FactId(u32)` handles (never reused, so handle order ==
  Drools fact-handle recency), alive flags for Phase-2 retraction.
- DRL parser covers the Phase 0–1 grammar (single pattern, `==/!=/</<=/>/>=`,
  bindings, `salience`, `no-loop`, RHS `insert(new T(...))` with literals /
  `$fieldBind` / `$factBind.getField()`). Everything else is a parse error —
  the scope wall is mechanical.
- **PROVISIONAL conflict resolution** (must probe in Phase 1): highest
  salience, then lowest fact handle, then rule declaration order. Only the
  single-rule insertion-order part is oracle-backed (D-006).
- `make diff` = single command differential run (builds oracle if stale):
  PASS p0_trivial_adult, 1/1. `make test` = pure-Rust tests (6 passing).

## Phase 1

### D-008: Conflict resolution PINNED via probes pr01–pr08 (oracle-verified)
Drools 9.44.0.Final, all facts via insert, fireAllRules(), java dialect:
- **Order key = (salience DESC, rule declaration index ASC, fact insertion
  order ASC), re-evaluated globally after every firing.**
- pr01: equal salience → rule declaration order (A before B).
- pr02: salience descending across B(20) > A(10) > D(0,default) > C(-5).
- pr03/pr05: equal salience is rule-major: ALL of an earlier rule's
  activations (facts in insertion order) fire before the next rule's.
- pr06: **preemption**: if a firing inserts a fact that activates an
  earlier-declared rule, that rule fires NEXT, before the current rule's
  remaining activations (B(1),A(1),B(2),A(2)).
- pr07: declaration order, NOT rule-name order (Zeta fired before Alpha).
- pr08: fact insertion order, NOT field-value order (9,1,5).
- Engine `next_activation` implements exactly this key. All probes are
  permanent regression scenarios under scenarios/probes/.
- NOT yet pinned (Phase 2): tuple ordering for multi-pattern activations,
  behavior under update/delete, timestamp/recency tie-breaks after mutation.

### D-009: Declared-type boolean getters are isX() ONLY (oracle-pinned)
Probe: `$s.getOk()` on a declared type with `ok : boolean` is a Drools
**compile error** ("The method getOk() is undefined"); `$s.isOk()` works.
- Parser accepts both `getX`/`isX` and resolves to field `x`; the engine is
  therefore *more lenient* than Drools (`getOk` on bool would compile here but
  not in Drools). The generator only emits the Drools-legal form, so the
  differential surface stays in-subset. Known, documented leniency — not a
  divergence risk (divergence requires oracle-legal input).
- Regression: scenarios/probes/pr11_bool_is_getter.json.

### D-010: Phase 1 curated corpus + property generator
- Curated: p1_ops_{i64,f64,str_bool}, p1_multi_constraint, p1_empty_pattern_
  no_match, p1_bindings_rhs, p1_duplicate_facts, p1_salience_preempt,
  p1_chain, plus probes pr09 (string relational ops DO work in DRL and match
  Rust byte-order comparison for ASCII — corpus strings stay ASCII-only) and
  pr10 (numeric cross-type: i64 field vs f64 literal and vice versa promote
  like Java). All green.
- Generator (`seine-harness fuzz <count> [seed]`, default seed 42,
  SplitMix64): 2–4 types × 1–3 typed fields; 1–6 rules; 0–3 constraints +
  0–2 field bindings per pattern; salience −10..10 (35% of rules); no-loop
  (10%); RHS 0–2 inserts with literal/binding/getter args (type-correct,
  i64→f64 widening allowed). **Termination by construction:** a rule matching
  Ti only inserts Tj with j>i (type-index DAG), so chains strictly climb.
  Divergent cases are auto-saved to scenarios/failures/.

### D-012: Phase 1 COMPLETE ✅ (done-bar met)
- Curated corpus: 21/21 PASS (`make diff`).
- Property fuzz: **10,000 cases, seed 42, 0 divergences**, 237s wall
  (`cargo run -q -p seine-harness -- fuzz 10000 42`). Reproducible: case k of
  seed s is deterministic.
- Trial-run stats (first 100 cases): 72% of scenarios produce ≥1 firing,
  414 firings total, max 42 in one scenario — the corpus is not trivially
  empty.

---

**HANDOFF @ FINAL checkpoint (Phases 0–2 COMPLETE)** — Definition of Done
per brief §6, against the D-017 subset:
- Curated corpus: **102/102 PASS** (`make diff`): phase-0/1 seed suites,
  probes pr01–pr11 + u01–u16 + j01–j22, 47 named fuzz regressions. Every
  scenario asserts final-fact-set AND ordered-firing-log equivalence
  against real Drools 9.44.0.Final.
- Fuzz: **30,000 full cases (seeds 42, 7, 123) + 8k spot cases, all at
  zero divergences** over the Phase-1+2 grammar (`make fuzz SEED=n`).
  Runs are deterministic (SplitMix64; case k of seed s always identical).
- ONE out-of-subset xfail (xfail/fz_42_4373, D-016/D-022) with an
  automated delta-minimizer (xfail/minimize.py) and analysis notes; the
  subset wall (D-017: mutation programs ≤2-pattern rules) is enforced by
  the generator and documented in the README.
- `make test` = 6 pure-Rust tests, no JVM needed.
- Environment for a fresh session: PATH needs `~/.cargo/bin`; JVM 21 +
  Maven resolve Drools from `~/.m2` (pinned 9.44.0.Final in oracle/pom.xml).
- If resuming: (1) the open xfail — extend minimize.py to drop constraints
  and RHS actions, shrink values, then hand-trace the ~15-update swap;
  (2) Phase 3 stretch items (not/exists, accumulate, matches/contains/in)
  were NOT started — Phases 1–2 solidity was prioritized per the brief.

**HANDOFF @ checkpoint 3** — Phase 1 COMPLETE (single-pattern rules: all six
operators × 4 field types, bindings, salience, preemption, chains, no-loop
(inert for inserts), 10k fuzz cases zero divergences). Phase 2 goldens
already captured in D-011 (probes_pending/j01–j05, oracle-only). Next:
extend engine to multi-pattern joins (left-major nested-loop activation
order per j01), cross-pattern var constraints (`Expr::Var` rhs), then
update/modify/delete RHS with render-after-RHS switch (j03), no-loop
(j04), activation cancellation on delete (j05); then move j-probes into
scenarios/, add curated Phase 2 corpus, extend fuzzer grammar (joins +
mutation with termination discipline), 10k fuzz. Open divergences: none.

### D-018: Agenda evaluation = outrank model (fz_42_2906 corrected D-015's peek)
An executing rule is interrupted only by rules that OUTRANK it (salience
desc, then decl order). Implementation: eager (no-loop) rules merge staged
batches at every flush; then walk priority order merging each network and
fire the first unfired match — rules below the firing rule accumulate.
This replaced the "peek at first non-executing rule" model (which over-
evaluated: fz_42_2906's single-batch left-major order proved rules below
the executor are NOT evaluated mid-execution). pr06 preemption follows
from outranking; fz_42_4138 per-firing batches follow from no-loop
eagerness; fz_42_4141's one-batch follows from lazy descent.

### D-019: Phase 2 COMPLETE ✅ (done-bar met, subset per D-017)
- Curated corpus: **95/95 PASS** (`make diff`) — probes pr01–pr11,
  u01–u13, j01–j22, p0/p1 suites, 41 named fuzz regressions.
- Property fuzz over the full Phase-2 grammar (joins ≤3 patterns in
  insert/delete programs, ≤2 patterns with update/modify, self-joins,
  guard-monotone mutation): **10,000 cases seed 42 AND 10,000 cases seed 7,
  both 0 divergences** (~255s each; final run after D-020 fixes, corpus at
  100/100).
- Open xfails (xfail/): fz_42_3408, fz_42_4373 — 3-pattern rules × long
  multi-update histories, outside the D-017 subset, kept with analysis
  notes in D-016 for a future session.

### D-020: RHS binding snapshots + indexed-equality coercion (seed-7 wave)
Second-seed fuzz found 3 value-level (not ordering) divergences:
- **LHS bindings used on the RHS are snapshots taken when the consequence
  starts** (Drools extracts declarations once): setters earlier in the same
  RHS must not affect later `$b` references (fz_7_2525: `setF1(-2);
  setF1($b)` restores the match-time value). Getter calls (`$p.getX()`)
  remain live reads. Engine: `Src::SnapField` + per-firing snapshot.
- **Join `==` coerces the bound value to the LEFT field's type** (Java cast:
  double→long truncates toward zero) — `I(n == $x)` with n=0, $x=-0.5
  MATCHES (u14, fz_7_4974). Join `!=` and relationals promote to double
  (u15: `n != $x` with $x=1.5 matches ALL ints), and literal comparisons
  always promote (`I(n == 1.5)` never matches). Engine: `eval_cmp_join`.
- Probes u14/u15 + 3 regressions added; corpus 100/100.

### D-021: Hot-prefix move-to-front (u16) — fz_42_3408 resolved
Post-final-checkpoint: probe u16 (u13's shape + a SECOND update event)
reproduced the xfail class minimally and pinned the missing rule: prefixes
holding a fact that is HOT at one of their positions move to the front of
their level's prefix memory (relative order kept) — gated by hot positions,
unlike the right-memory move which is ungated (D-018/fz_42_3433 vs 4359).
fz_42_3408 now passes and is a regression; corpus 102/102.

### D-022: Cascade-based refire requeue (fz_42_4373 minimization, round 1)
A delta-minimizer (xfail/minimize round) shrank fz_42_4373 to 3 rules /
2 facts and pinned the requeue mechanism exactly: refires propagate like
inserts — the left-update stream walks a hot tuple's existing extensions
in RIGHT-MEMORY order, emissions REVERSE between joins, then the
right-update stream walks the left memory; the terminal requeues in
arrival order with dedup. This replaced the position-ascending
approximation (which coincidentally matched all shallower pins — the new
cascade reproduces every one of them; corpus 102/102, spot fuzz 8k more
cases clean). The minimized round-1 case passes.
**fz_42_4373 (full) remains the single open xfail**: divergence moved from
firing 391 to 665; a second minimization round leaves a 4-fact/3-rule case
diverging at firing 109 of 172 — a positional swap between a requeued
refire and a pending entry after ~15 update events. Next session: extend
the minimizer to also drop individual constraints/actions and shrink fact
fields, then hand-trace. The D-017 generator wall stays until resolved.

### D-023: LAST XFAIL RESOLVED — unified update cascade; D-017 wall LIFTED
Session continuation. Tooling first: `SEINE_HANDLES=1` makes both runners
emit fact-handle tags (`__h`) for unambiguous log comparison (oracle handle
ids are 1-based, engine 0-based — offset by one); tools/minimize.py is a
delta-debugger that shrinks a scenario while the divergence persists
(rules, facts, constraints, setters, statements).
Three minimization rounds against fz_42_4373 pinned, in order:
1. Refires propagate through the join chain exactly like inserts (D-022's
   cascade — round-1 case).
2. A hot-moved prefix block is NOT in prior memory order (round 3).
3. **The unifying rule (round 4): a property-hot update re-enters the
   staged flow as a re-insert.** Its U-chain (left-stream over the right
   memory, reversal between joins, right-stream over the left memory)
   determines at every level: the re-prepended block order of the prefix
   memory (with fresh creation seqs, so subsequent hot-first iterations see
   U order), and the requeue order of previously-fired activations at the
   terminal. Pending activations still keep their positions (u01–u04).
   This subsumes D-021's move-to-front and D-022's requeue ordering — both
   were special cases of the same mechanism.
fz_42_4373 passes. (The wall-lift attempted here was later re-imposed —
see D-025.)

### D-024: Widened-grammar wave (seeds 42/777) — three more pins
Lifting D-017 and fuzzing the full grammar found 3 divergences; each
minimized to ≤3 rules / ≤3 facts with tools/minimize.py + SEINE_HANDLES:
- **fz_42_5243 (2 rules, 2 facts):** the rule that just fired re-evaluates
  its own network even if its own RHS UNLINKED it (the executor is still
  active) — engine: force-merge of the last-fired rule bypassing the
  linking gate. Virgin/bystander unlinked rules still accumulate (fz_7_145
  unchanged).
- **fz_42_9462 (2 rules, 2 facts):** PENDING join activations whose tuple
  is hot at a RIGHT position also requeue (retract+reassert of the join
  child), and the requeue block is PREPENDED ahead of kept entries — every
  earlier requeue case had an empty kept list, masking the placement.
- **fz_777_1853 (1 rule, 2-3 facts, two rounds):**
  (a) HOT-position memory moves happen BEFORE the update cascade
  (fz_42_1057 sees moved order) but UNGATED moves of non-listening right
  memories happen AFTER it (the same-batch requeue sees pre-move order;
  fz_42_3433 only observed the move from a later batch);
  (b) the final requeue matrix: **requeue iff FIRED or RIGHT-hot; a
  PENDING activation hot only at pos0 (pure left-update, or k==1) is
  updated in place** — reconciling u01–u04, fz_42_2804/9462 and both
  rounds of fz_777_1853.
Corpus 106/106 after promoting all three.

### D-025: Widened-grammar campaign paused — wall re-imposed; open class
### = requeue PLACEMENT among pending join activations
After D-024's fixes, a 4-seed × 10k campaign on the unrestricted grammar
still produced ~2 divergences per 10k. Two minimized counterexamples now
DIRECTLY contradict each other under every simple placement rule tried:
- fz_42_9462 wants a requeued pending activation AHEAD of a pending cold
  one; the fz_42_3554 min-case wants requeued pending activations to stay
  IN PLACE (its firing-1 batch), while its firing-0 batch is ambiguous.
- Hand-derivation of PHREAK's agenda (in-place child updates vs
  retract/reassert, activation numbering, queue discipline) no longer
  converges from black-box order observations alone at this depth; the
  next step is modelling the true per-rule activation QUEUE (activation
  numbers, possibly LIFO segments) rather than a list with placement
  heuristics.
- State: engine keeps ALL D-023/D-024 fixes (each independently validated;
  corpus 106/106 includes fz_42_5243/9462, fz_777_1853); the D-017 wall is
  RE-IMPOSED in the generator; ~22 unminimized widened-grammar failures
  are parked in xfail/ as the work queue for the next campaign
  (tools/minimize.py + SEINE_HANDLES=1 are the workflow).
- IMPORTANT correction: the wall does NOT fully exclude the open class —
  a post-fix walled fuzz found ~2/10k divergences (fz_42_3311-class: the
  class reaches 2-pattern mutation programs too; earlier 30k-clean runs
  simply never drew these shapes). The proven-subset claim is therefore
  weakened until the class is closed; all failure cases are parked in
  xfail/ (26 files).
- One of them (fz_42_3311 round 1) pinned cleanly along the way: a BARE
  update() carries Drools' ALL-SET mask, which is CLASS-reactive — it
  refires even empty-listen patterns (unlike property masks, j13); engine
  treats the u64::MAX sentinel mask as intersecting everything.
- DIRECTION DECIDED for the next round: stop black-box order-fitting. The
  drools-core 9.44.0.Final -sources jar (fetched into ~/.m2, extracted for
  READING ONLY under the scratchpad) shows the real structures:
  PhreakJoinNode.doNode phase order (rightDel, leftDel,
  reorderRightMemory(removeAdd→moves tuple to END), reorderLeftMemory
  (remove-all→re-append), rightUpdates, leftUpdates, rightInserts,
  leftInserts), TupleList memories APPEND at tail, TupleSets staged lists
  PREPEND (LIFO), and child-tuple lists per parent. The next engine
  iteration should be a faithful behavioral port of this node algorithm
  (still validated only through oracle probes; no code copied), replacing
  the fitted emission heuristics in merge_staged.

**HANDOFF @ phreak-port MERGE (Session 3 close)** — The behavioral port of
the PHREAK node algorithm is the engine (engine/src/phreak.rs + engine.rs
integration). Proven state at merge:
- Corpus: 156/156 (was 106; +26 graduated ex-xfails, +24 new probes and
  regressions from this session's discriminator ladders).
- Fuzz, UNWALLED grammar (mutation + 3-pattern rules mix freely): seeds
  42, 7, 123, 777, 999 x 10,000 cases = 50k cases, ZERO divergences.
- All 26 parked xfail cases from D-025 resolved and graduated; xfail/ is
  gone; D-016/D-017/D-025 walls retired.
- `make test` green; tree clean at every commit on the branch.
New pinned mechanism classes this session: eager/lazy evaluation windows
(the j01-vs-9462 discriminator), bucket-change vs same-bucket child
sync-walks with cursor threading, object-identity staging folds,
downstream-pending clash-moves, property-miss right-tuple reAdd with
child realignment, side-aware index-key coercion, agenda-item lifecycle,
and build-time alpha literal sharing/hashing (D-027..D-029).
Next session candidates: Phase 3 stretch (not/exists, accumulate,
matches/contains/in) — restrict the generator first, probe before
implementing; or scale campaigns (more seeds, larger CASES) for the
current subset.

### D-029: Alpha-node literal sharing + hash-threshold coercion (seed 777)
fz_777_4504 (first unwalled multi-seed campaign find) exposed BUILD-TIME
alpha-network semantics for `field == literal`, pinned by probe series
w1-w18 + the pr_lit matrix:
- Node identity: (type, preceding-literal-constraint chain, field,
  literal COERCED to the field's type). A later rule whose coerced
  literal collides SHARES the first-built node and inherits its ORIGINAL
  literal: after `n == 1`, `n == 1.5` matches n=1 (w10); built the other
  way around, `n == 1` matches NOTHING (w16 — first-built 1.5 wins).
- Hashed sinks: >= 3 sibling eq-nodes (post-sharing) on one field switch
  membership to the COERCED key — a double literal on a long field
  truncates: `f0 == 2.5` matches f0 == 2 (w5/w8/w12, fz_777_4504's
  {1, -2, 2.5} group). Three IDENTICAL literals share one node and stay
  below the threshold (w7).
- Below the threshold each node evaluates its first-built literal with
  double promotion: standalone `n == 2.5` matches nothing (w4/u15).
`!=` and relationals always promote to double (pr_lit: `f0 != 2.5` and
`f0 == 2.5` BOTH match f0 == 2 when the eq-group is hashed).
Implemented as a compile-time literal rewrite (share_and_hash_alphas).
Multi-seed unwalled campaign: seeds 42/7/123/999 clean at 10k; seed 777
clean after this fix.

## Phase 3 (stretch: operators, not/exists — 2026-07-04)

### D-030: matches/contains/in semantics PINNED (probes op_m*/op_c*/op_i*)
Oracle-verified on Drools 9.44.0.Final; probe files promoted to
scenarios/probes/pr_op_*.json:
- **`matches` is java.util.regex full-string matching** (String.matches):
  `s matches "b"` does NOT match "abc" (op_m2); `""` matches only the empty
  string (op_m5). Classes/ranges/negation `[^a]`, alternation, groups,
  `. * + ?` behave standard (op_m4). It even COMPILES on numeric fields
  (op_m3: `n matches "1"` fires — value stringified); SUBSET WALL: the
  engine restricts `matches` to String fields with literal String rhs, so
  the engine is stricter than Drools here (safe: generator never emits it).
- **`contains` on a String field is substring semantics** (op_c1), and
  `contains ""` matches every string (op_c2). Wall: String field + literal
  String needle only (our fact model has no collections).
- **`in`/`not in` are a composite OR of `==`-with-promotion branches**:
  a double literal in the list does NOT truncate against a long field
  (op_i3: `n in (2.5, 9)` skips n=2), int literals promote against double
  fields (op_i3b), string and bool lists work (op_i5).
- **`in` does NOT participate in D-029 alpha eq-node machinery**: its
  branches don't count toward the >=3 hash threshold (op_i4: `n == 2.5`
  beside an in-rule stays sub-threshold/promote) and don't share nodes
  with plain `==` constraints (op_i6: `in (1.5, 9)` does not inherit the
  `n == 1` node's literal). BUT an in-constraint DOES contribute to the
  preceding-constraint prefix chain that scopes downstream eq-node groups
  (op_i7: three `n == lit` nodes under a common `m in (5)` prefix hash and
  truncate, while the identical literal at top level stays promote-only).
  Engine: share_and_hash_alphas pushes a descriptor for every constraint
  kind into the prefix; only Cmp/Eq/Lit constraints form group members.
- **Listen masks include fields referenced by the new operators** (op_m6:
  masked update {s,n,t} refires matches/in/contains rules; op_m7: a
  {t}-only update does not refire a rule matching on s).
- Engine regex: a tiny backtracking matcher over the tame subset
  (literals, `.`, classes with ranges/negation, groups, `|`, `* + ?`),
  full-string acceptance — equivalent to Java for this feature set (no
  backrefs/lookaround; acceptance-only so greediness is irrelevant).
  Corpus strings stay ASCII and newline-free (pr09/D-010), so Java's
  `.`-excludes-newline and negated-class-includes-newline edge cases
  cannot arise. Everything else (`{n,m}`, `\d`, anchors, `$`-vars) is a
  parse error = subset wall.

### D-031: not/exists CE semantics PINNED (probe ladder ne_n*/ne_e*/ne_f*/ne_l*)
Oracle-verified on Drools 9.44.0.Final; drools-core sources re-fetched for
READING (behavior reference only — no code copied). Pins:
- **Match rendering:** not/exists CEs contribute NO element to the firing's
  match list (ne_n1/ne_e1); a rule whose FIRST pattern is a CE matches on
  Drools' InitialFactImpl, which appears in the match objects (ne_f1) but
  never in the final fact set. The oracle canonicalizes it as
  `{"type":"InitialFact","fields":{}}` (raw toString carries an identity
  hash — nondeterministic); the engine mirrors with a synthetic reserved
  InitialFact fact inserted before scenario facts when needed.
- **Blocker model** (from sources, behavior confirmed by probes): each left
  tuple holds <=1 blocker (first matching right in bucket order); blocked
  lefts leave the left memory; a right's blocked-list PREPENDS. not
  propagates unblocked lefts, exists propagates blocked ones.
- **Cancellation/refire:** blocker arrival cancels pending not-activations
  (ne_n3); losing the last exists-support cancels pending exists ones
  (ne_e3). Support/blocker HANDOVER (another matching right remains) keeps
  state without firing or cancelling (ne_n7/ne_n10/ne_e3b/ne_e6). Unblocking
  REFIRES an already-fired not match (ne_n5); a mass unblock fires in
  REVERSE left-arrival order (ne_n4: A3,A2,A1).
- **No refire on in-place updates:** a property-relevant update of the
  blocking/supporting fact that leaves the block state unchanged does NOT
  refire the rule (ne_e5: exists refired neither, contrast join j12; not is
  trivially inert while blocked). Only alpha/bucket TRANSITIONS act (as
  right ins/del: ne_n8 fires R after the blocker leaves its alpha).
- **Chains:** CE children pass through later joins as ordinary tuples with
  the standard D-013 prefix reversal (ne_j1 fired A2C7,A2C8,A1C7,A1C8).
- **Linking:** not nodes start LINKED; only UNCONSTRAINED (no join
  constraint) not nodes can unlink — they unlink while rights exist (with
  a one-evaluation link pulse on the 0->1 right insert so the blocking
  batch processes) and re-link when the right count returns to 0. exists
  links like a join (rights nonempty). ne_l1/ne_l2: lefts staged while
  unlinked accumulate; the re-link batch processes right-delete unblocks
  BEFORE accumulated left inserts (ne_l2 fired A0 then A1).
- **doNode phase order (sources):** leftDel, existential-reorder-left,
  existential-reorder-right (captures tempBlocked + tempNextRightTuple =
  next non-staged neighbor forward else backward; re-added updates with
  empty tempNext become their own resume point), rightIns, rightUpd
  (unblocked-pass then tempBlocked walk; a null tempNext flips a loop-wide
  iterate-from-start flag that persists for later rights), rightDel
  (re-search from bucket start, staged-deleted rights ineligible), leftUpd
  (keep still-allowed blocker iff every beta constraint is
  equality-indexable or there is <=1 — isLeftUpdateOptimizationAllowed),
  leftIns. Staged-UPDATE lefts are skipped by every right-side walk
  ("children cannot be processed twice") and re-attached to the current
  right's blocked list when met in a tempBlocked walk.
- Subset walls: bindings ($x : f or fact binds) inside not/exists patterns
  are rejected (Drools scopes them out anyway); bare `not T(...)` /
  `exists T(...)` forms only (no parenthesized CE groups, no nesting); the
  type name InitialFact is reserved.

### D-028: PHREAK port LANDED — corpus 145/145, all xfails closed, wall lifted
The faithful port (branch `phreak-port`) replaced the fitted merge engine.
`engine/src/phreak.rs` implements the node algorithm; `engine.rs` keeps
compile/RHS/agenda. Everything below is oracle-pinned (probes pr_c*, pr_d*,
pr_v*, pr_coerce + 20 graduated fz_123_* regressions):
- Staging: TupleSets prepend (LIFO), consumed head-first everywhere; the
  staged-type folds are by OBJECT identity, so a killed-and-recreated child
  is del+ins, never an in-place update (c13). Same-list re-staging is a
  no-op; a walk touching a tuple staged in the DOWNSTREAM pending set moves
  it to the head (updateChildLeftTuple clash rule; merge_into_pending).
- Memories: TupleList append; removeAdd re-keys and moves to the END.
  Child tuples link at the END of both parents' lists; the sync-walk
  insert case threads a cursor (insert-before-cursor keeps alignment).
  Bucket-change vs same-bucket branches per doRightUpdates/doLeftUpdates,
  including the staged-update-left skip ("children cannot be processed
  twice") — right-insert processing has NO effective skip (flags cleared).
- k=1 rules: WM staging consumed OLDEST-first (pr08/pr04 pin).
- Terminal: updates then inserts, head-first, appending to the executor
  queue; queued activations keep position; unqueued (fired) re-append.
- Eagerness is real but only controls WHEN evaluation happens (per flush
  for no-loop rules); it does NOT change consumption order (c7 vs c10-c13
  probe ladder: the j01-vs-9462 "contradiction" was eager evaluation
  windows, not staging conventions).
- Property-miss reAdd: a modify whose mask MISSES a right input still
  removeAdds the right tuple (re-keyed, to memory END) immediately and
  re-appends its children in their left parents' lists — no staging, no
  child updates (fz_42_4359/3433 vs fz_42_1057/fz_123_1438; probes d4-d7).
- Indexed join keys are stored in each side's NATURAL type; the probing
  side coerces to the stored side's type: left-probes-right truncates
  (u14), right-probes-left widens, so long -1 does not find double -1.5
  (fz_123_3057; pr_coerce matrix).
- Agenda-item lifecycle (fz_42_1464 vs fz_42_124): the item is created on
  first LINK; once queued it EVALUATES whenever reached even if currently
  unlinked (memories advance, nothing fires); it is removed when its
  activation queue empties; new staging re-queues it ONLY while linked;
  never-linked rules accumulate staged input unevaluated (fz_7_145).
  The just-fired rule is still force-evaluated (fz_42_5243).
- A 64-combo grid search over staging/consumption directions confirmed
  the source-literal conventions are uniquely optimal; every remaining
  divergence was a missing MECHANISM, not a direction.
D-016/D-017/D-025 are RETIRED: the generator wall is lifted permanently
(gen.rs allows mutation + 3-pattern rules together). D-021/D-022 cascade
heuristics are superseded by the port. xfail/ is gone — all 26 cases are
regressions now.

### D-026: Faithful node-algorithm port — attempted, reverted, groundwork
### banked for next session
A full behavioral port of PhreakJoinNode/PhreakRuleTerminalNode was built
and exercised against the corpus, then REVERTED (46/106 → the fitted
engine at HEAD stays authoritative at 106/106). What the attempt
established (all verified by hand-simulation against oracle logs):
- The real algorithm reproduces u09's initial batch EXACTLY under: staged
  TupleSets prepend (LIFO) consumed newest-first, right-inserts processed
  before left-inserts, memories append at tail, trg prepends per child.
- The port's terminal semantics are the truth for the requeue class:
  RuleExecutor.tupleList holds only QUEUED activations; fired tuples leave
  the list (getNextTuple = removeFirst + setQueued(false)); a terminal
  UPDATE is a no-op for queued tuples and re-APPENDS unqueued (fired) ones
  ("effectively recreated"); no-loop compares the propagation origin's
  terminal; the salience queue only exists for dynamic salience.
- THE DISCRIMINATING PAIR for the remaining unknown: j01 (2-pattern
  indexed join, fires in left-FIFO x right-ascending order) vs fz_42_9462
  (2-pattern indexed self-join, initial firing order effectively
  left-LIFO). No single FIFO/LIFO staging convention reproduces both under
  the ported doNode; the difference likely lives in the eager-evaluation
  flush boundaries (9462's rule is no-loop/eager, j01's is not) and/or the
  indexed-join child-sync walk (doRightUpdatesProcessChildren).
- Next session: resume the port on a branch; instrument BOTH engines with
  SEINE_HANDLES over j01/u09/9462/pr08/pr04 as the calibration set; read
  PhreakJoinNode.doRightUpdatesProcessChildren + TupleIndexHashTable
  iteration order; only swap the engine when the calibration set is green,
  then run the corpus + full fuzz.
Sources for READING live under the scratchpad (re-fetch:
`mvn dependency:sources -DincludeArtifactIds=drools-core` and unzip; do
NOT copy code into the port — behavior only, validated via oracle).

## Phase 2 (pre-work: goldens captured, engine not yet extended)

### D-011: Join + mutation semantics observed via probes j01–j05 (oracle-only,
files in probes_pending/ — move into scenarios/ once the engine supports them)
- j01/j02: join activation order = leftmost pattern's fact handle asc, then
  right pattern's handle asc (nested-loop order, left-major). Match object
  list is in pattern declaration order [P, A].
- j03: **afterMatchFired renders facts POST-RHS**: `bump`'s own match shows
  `done: true` (the value its RHS just wrote). Engine currently renders
  matches pre-RHS; identical for Phase 1 (no mutation), but Phase 2 MUST
  switch to render-after-RHS. Also: update() re-evaluates and fires
  newly-matching rules ("see" fired after).
- j04: no-loop suppresses self-reactivation from the rule's own update();
  fires exactly once.
- j05: delete() cancels not-yet-fired activations (P(2)'s "see" activation
  never fired). Deleted facts can still be rendered in the firing log entry
  of the deleting rule (Java object outlives retraction; our arena keeps
  values under a dead alive-flag, so same capability).

### D-013: Phase 2 semantics FULLY PINNED via probes j01–j22 (oracle-verified)
**Join activation order (j01, j02, j08, j09, j17):** for patterns p0..pk-1,
enumerate left-major with a twist: prefix list for p1 = p0's facts ascending
(alpha→first-join is NOT reversed); before joining each pattern pi with i≥2,
REVERSE the accumulated prefix list (PHREAK prepends tuples into the next
join's staged list); right-side facts always iterate in ascending handle
order. Firing order within a rule = final list order. Verified exactly on
2-, 3- (j08: P2Q2R2 first), and 4-pattern (j17, all 16 tuples) joins.
Self-joins include same-fact-in-multiple-positions tuples (j09: (P1,P1)).
Match rendering lists facts in pattern declaration order, values POST-RHS.

**Property reactivity (ON by default; j06, j07, j12, j13, j14):**
- Pattern listen-mask = fields referenced in its constraints, INCLUDING
  field bindings (j14). Empty pattern `P()` listens to NOTHING (j13: no
  refire ever).
- update() modification mask = union of fields written by setters on that
  fact in the RHS before the update call; **no setters ⇒ ALL-fields mask**
  (j21: bare update() self-loops infinitely — fire-limit parity required).
- On update: every activation (fired or pending) whose tuple contains the
  fact at a position whose listen-mask overlaps the modification mask is
  cancelled & re-created if still matching — fired ones fire AGAIN (j12),
  non-overlapping ones do NOT refire (j06/j07: mask {t} vs listen {n}).
- Re-created activations occupy their natural (handle-order) position in
  the rule's candidate order, not last (j18: see fired 1, 20, 3).
- Refires preempt by the normal agenda key immediately (j16).
- no-loop: suppresses ONLY the same rule-instance's re-creation caused by
  its own update (j04); other rules and other tuples unaffected.

**Mutation misc:** modify($p){ setX(..), setY(..) } ≡ setters+update with
the block's mask (j10). delete() cancels pending activations (j05, j11);
deleted facts still render in the deleting rule's own firing entry (arena
keeps values under a dead flag). j22: left-side updates re-join and refire
with re-evaluated bindings.

**Termination discipline for the Phase-2 generator:** update rules must be
guard-monotone (pattern requires `g == false`, RHS sets g=true before
update; bool setters only ever write true), inserts keep the type-index
DAG rule (target index > max pattern index). Bare update() (all-fields
mask) is NEVER generated — it non-terminates (j21).

### D-014: Incremental join-network semantics PINNED (probes u01–u10 +
fuzz counterexamples fz_7_58/87/145/159, all now regressions)
The Phase-2 fuzzer found 4 divergences in its first 200 cases; resolving
them pinned the full PHREAK staging model. The engine now maintains a real
per-rule join network:
- **Eager alpha, lazy beta:** alpha tests are evaluated at insert/update
  time (a fact that starts alpha-passing only after a later update takes
  that LATER queue position — fz_7_58). Beta (join) processing is deferred
  per rule until the agenda next considers it, so deltas from several
  firings can merge into ONE batch (fz_7_87: two inserts from one RHS).
- **Segment linking (fz_7_145):** while any pattern position has zero
  alpha-active facts the rule is unlinked — staged events accumulate
  (pruning/cancellation still applies) and are processed as one batch when
  every position has data. This is why "initial facts + later inserts" can
  be one batch for a rule whose first pattern started empty.
- **Batch processing per join** (u05–u10): staged left tuples first, each
  against the FULL right memory; then staged right facts against PRE-batch
  lefts; update-driven new pairs before both, in update-event order (u07).
  Emissions REVERSE when propagated to the next join (linked-list prepend)
  and append unreversed at the terminal. Memory orders: alpha and prefix
  memories BLOCK-PREPEND new batches (FIFO within batch; u09 pinned
  [new..., old...] right iteration, fz_7_159 pinned batch-2-before-batch-1
  prefix iteration); the terminal match list keeps kept entries in place
  and appends emissions (u01–u04: still-matching updates keep position).
- Deactivate→reactivate cycles lose list position (re-derived tuples).
- Curated corpus after this work: 55/55 PASS (`make diff`).
- NOT pinned (documented leniencies): mixed insert+update emission
  interleaving within one batch beyond u07's coverage; multi-update single
  RHS refire ordering (generator emits ≤1 update per RHS); alpha-memory
  iteration order after unlink/relink cycles; setters without a following
  update() (Drools leaves stale matches; generator always pairs them).

### D-015: Second fuzz wave — full PHREAK agenda/staging model (probes u11,
### regressions fz_42_*, 17 resolved + 3 open xfails)
Phase-2 fuzz (seed 42) found 20 divergences by case ~4400; resolving 17
pinned the deepest layer of PHREAK semantics:
- **Eager vs lazy rule evaluation:** no-loop rules evaluate their staged
  batch at EVERY flush window (their activations must be known); plain
  rules evaluate via the agenda peek — walk priority order, merging dirty
  networks, stopping after the first rule other than the one that just
  fired that has an unfired match. Rules beyond keep accumulating batches
  (fz_42_4138 vs fz_42_4141 — same shape, differ only in no-loop).
- **Hot updates move facts to the FRONT of their alpha memories**
  (fz_42_388/1057), while pending activations keep agenda position.
- **Fired activations re-created by an update lose their agenda position**:
  they requeue during the update phase (before insert-derived appends),
  ordered per hot event, hot positions ascending, terminal-join left-memory
  order within, hot-moved rights first (fz_42_2804/2055/1057).
- Left-update child iteration follows tuple CREATION order, not memory
  order (u11, fz_42_1176): creation seqs tracked per prefix/match entry.
- Emission phases per join: LI (staged lefts x full rights), RI (staged
  rights x [hot lefts creation-order, cold lefts memory-order]), LU (hot
  lefts x full rights, missing only), RU (hot rights, missing only).
- Corpus: 72/72 green (`make diff`), including 17 fz_42_* regressions.

### D-016: OPEN xfails (xfail/, excluded from make diff) — updated
- fz_42_3433 RESOLVED: alpha-memory move-to-front on update is NOT gated by
  listen masks (any update repositions the fact in every alpha memory it
  occupies; property reactivity gates only tuple re-evaluation). Now a
  regression + engine behavior.
- fz_42_3408, fz_42_4373 remain OPEN: both need >2-pattern rules with long
  multi-update histories; the residual gap appears to be hot-left iteration
  order divergence between indexed and unindexed joins after accumulated
  moves (u11: hot-first for a join whose key changed; 3408's unconstrained
  join at the same shape iterates cold-first). u12/u13 (clean single-update
  probes of the same shapes) PASS — only deep histories diverge. Next
  session: build a u14 probe = u13 + a SECOND update event, compare hot
  iteration; suspect per-event compounding of alpha/prefix moves.

### D-017: Subset wall — mutation programs are capped at 2-pattern rules
Because of D-016, the PROVEN subset excludes programs that combine
update/modify with rules of 3+ patterns. The generator enforces it
(`allow_mutation` programs cap every rule at <=2 patterns; 3-pattern rules
appear only in insert-only programs). 1-2 pattern mutation semantics and
3-pattern static semantics are each fully pinned; several 3-pattern+update
scenarios pass anyway and remain as extra regressions beyond the promise
(fz_42_1176/2537/4138/3433, u11-u13).

---

**HANDOFF @ checkpoint 2** — Phase 0 COMPLETE. Proven: full pipeline
(scenario JSON → DRL parse → columnar WM → match/fire → canonical JSON →
comparator) matches real Drools 9.44.0.Final byte-for-byte semantically on
p0_trivial_adult; `make diff` green, `make test` green. Next: Phase 1 —
(1) probe conflict resolution: multi-rule same-fact tie-break, salience
order, interleaved insert-during-fire ordering; (2) curated single-pattern
scenarios (all operators × all field types, bindings, no-loop); (3) seeded
property generator ≥10k cases. Open divergences: none. Open risks: agenda
policy beyond single-rule case is provisional (D-007).

**HANDOFF @ checkpoint 1** — Phase 0 in progress. Proven: Java oracle
(oracle/, Drools 9.44.0.Final pinned) runs scenario JSON → canonical NDJSON,
verified on `scenarios/phase0/p0_trivial_adult.json`. Build:
`cd oracle && mvn -q -DskipTests package`; run:
`java -cp "oracle/target/classes:$(cat oracle/target/classpath.txt)" dev.seine.oracle.OracleRunner <scenario>...`.
Next: Rust workspace (engine + harness crates), walking-skeleton engine
(parse this one rule, columnar arena WM), comparator, `make diff` green on
p0_trivial_adult. No open divergences.
