# Brief: Faithful Rust Port of the Drools Core Rule Engine (Bounded Subset, Differentially Proven)

**Audience:** an autonomous coding agent running a single capped, unsupervised session.
**Mission (one line):** Port the *core forward-chaining engine* of Drools into Rust over a deliberately small DRL subset, and **prove** the port is faithful by running real Drools as a live oracle and differential-testing every behavior against it.

The proof is the point. A small subset proven to 100% equivalence beats a large subset that is probably right. Coverage loses to correctness every time.

---

## 0. Prime directive — read this before anything else

1. **Drools is the oracle. Never invent semantics.** For any question about how a rule "should" behave, do not reason it out from a textbook or from the Rete literature — modern Drools runs **PHREAK** (a lazy, goal-oriented evolution of Rete), and its observable behavior is the *only* spec that counts. Write the smallest possible DRL probe, run it through real Drools, capture the actual result, encode that, and keep the probe as a regression test.
2. **The bar is exact equivalence, not "close."** For any in-subset program, the port must produce (a) the identical final set of facts in working memory, and (b) the identical *ordered* sequence of rule firings. Firing order is not optional — conflict resolution is exactly where faithful ports silently diverge.
3. **Always keep `main` green.** End every phase on a working, fully-tested tree. Commit at every green checkpoint. Never leave the tree broken at a stopping point — the session is capped and may end at any moment; whatever exists must be a coherent, proven artifact.
4. **Vertical slice first, then widen.** Get the thinnest possible end-to-end path (parse one trivial rule → build network → fire → match Drools byte-for-byte) working before adding any feature. Depth-complete beats breadth-incomplete.
5. **Respect the scope walls (Section 3).** If a generated test scenario uses an out-of-subset feature, the *generator* is wrong — constrain the generator, do not expand the engine. This is the single most important rule for not burning the whole budget.

---

## 1. Why this exists (context)

This is a clean-room-adjacent reimplementation intended to land as the first real repository in a personal software org, under a clean IP provenance story. Drools' core engine is Apache 2.0 and now lives at the Apache Software Foundation (Apache KIE, incubating), which makes a derivative port licensable and clean **if** the hygiene in Section 8 is followed exactly. There is no dominant, mature PHREAK/Rete forward-chaining engine in Rust, so a faithful, well-proven subset port is a genuine contribution rather than an exercise.

---

## 2. Scope — IN (the supported DRL subset)

Deliver this in strict phase order. Each phase has a hard done-bar; do not start a phase until the previous one is green.

### Phase 0 — Harness, oracle, and walking skeleton (foundational — do this first)
- Rust workspace: an **engine** crate and a **harness** crate.
- **Reference runner (Java):** loads a DRL string, asserts facts from a scenario, fires all rules, and emits a **canonical result JSON**: the final fact set + the ordered firing log. Uses Drools via Maven/Gradle (`drools-compiler` / `kie-ci`), pinned to a specific version recorded in the repo.
- **Port runner (Rust):** same input interface, same canonical JSON output.
- **Comparator:** diffs two canonical JSONs and reports divergences precisely.
- **Scenario format:** JSON declaring fact-type schemas + initial facts + inline DRL.
- **Memory-architecture constraint (day one, non-negotiable):** represent working memory, the fact store, and the alpha/beta memories as **arena-backed, id-based (integer-handle, not pointer-chasing), columnar-friendly** structures from the very first commit. Facts as packed values in contiguous arenas; beta-memory tuples as compact id-tuples, not boxed object graphs. This is a *layout* constraint, not an optimization pass — do not tune anything yet, just don't foreclose the future. Getting this right up front is what keeps later moves reachable — mmap-backing the arenas for OS-paged cold spill, splitting a hot/cold fact tier, or sharding facts by key so a working set larger than RAM can stream through in partitions. A naive pointer-chasing object graph permanently locks all of those out; an id-based columnar layout costs little now and holds every door open. (Beyond-RAM itself is **out of scope** for this run — see Section 3 — this bullet only preserves the option.)
- **Done-bar:** one trivial rule (`Person(age > 18)` → insert an `Adult` fact) passes end-to-end **identically** on both engines through the full pipeline.

### Phase 1 — Alpha network (single-pattern rules)
- Fact model: a handful of generic types with typed fields (`i64`, `f64`, `String`, `bool`).
- Patterns: single type + comma-separated field constraints (implicit AND).
- Operators: `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Variable binding: `$p : Type(field > 3)` and field binding `$a : age`.
- RHS actions: `insert(new Type(...))`, plus firings captured automatically by the harness.
- Agenda + conflict resolution with **`salience`**; pin the default tie-break order via the oracle (do not assume — probe it).
- Refraction / `no-loop` so runs terminate.
- **Done-bar:** curated single-pattern scenarios at 100% match, **and** a property-based generator emitting random single-pattern programs finds zero divergences over ≥10,000 cases.

### Phase 2 — Beta network (joins) + fact mutation
- Cross-pattern joins on shared bound variables: `$p : Person($a : age)` … `Account(balance > $a)`.
- RHS: `update` / `modify` (must trigger correct re-evaluation of dependent patterns — the PHREAK-interesting part), and `delete` / `retract`.
- Re-firing semantics on mutation, and termination behavior — pin *exactly* against the oracle.
- **Done-bar:** curated multi-pattern + mutation scenarios at 100% match, **and** a fuzz generator over the full Phase-2 grammar finds zero divergences over ≥10,000 cases.

### Phase 3 — Stretch (ONLY if budget remains and Phases 1–2 are rock-solid)
`not` / `exists` conditional elements; `accumulate` / `collect`; additional operators (`matches`, `contains`, `in`); salience expressions. Each is independently optional. Do not start any Phase 3 item at the expense of Phase 1–2 solidity.

---

## 3. Scope — OUT (hard walls — do not cross)

Explicitly **not** in scope, regardless of how easy they look or how tempting mid-run:
- **MVEL dialect** (only the minimal Java-like expression subset needed for the operators above).
- **DMN, CEP / temporal operators, complex event processing.**
- **Backward chaining, query support, truth maintenance beyond what Phase 2 mutation requires.**
- **Workbench / KIE authoring tooling / DRL6 full grammar / decision tables / rule templates.**
- **Persistence, marshalling, sessions clustering, multithreaded firing.**
- **Beyond-RAM / disk-backed / external-store working memory** (RocksDB/LMDB-backed match state, mmap spill, hot/cold tiering, partition-sharding). The Phase-0 layout constraint keeps these *reachable later*, but building any of them is out of scope: every viable path either assumes fact partitioning or requires diverging from Drools' matching algorithm, which would break the differential oracle. Bank the in-RAM constant-factor win now; park beyond-RAM with the algorithmic-divergence ideas.
- Anything requiring network calls or external state at rule-fire time.

If a feature here "comes up," the correct response is to restrict the input/generator, not to build it.

---

## 4. Deliverables

1. A named Rust engine crate implementing the Phase 1–2 subset (Phase 3 features only if reached).
2. The differential harness: Java reference runner + Rust runner + comparator + scenario generator, runnable via a **single command** (e.g. `make diff` / a `cargo xtask`).
3. A test corpus: curated seed scenarios + the property/fuzz generators, all wired into CI-style local runs.
4. A `README` documenting the supported subset, the explicit non-goals (copy Section 3), how to run the harness, the pinned Drools version, and the provenance/licensing story.
5. A running `DECISIONS.md` capturing every semantics probe, every tie-break discovery, and every documented known-limitation, so the run is auditable after the fact.

---

## 5. Method — order of work

1. **Characterization first, then port.** Before porting a behavior, capture real Drools' actual output for it as a golden-master scenario. Port to match the golden master. This makes the test suite the proof artifact and independently documents Drools' behavior.
2. **Differential on top of golden-master.** Once a phase's hand-written scenarios pass, turn on the generator/fuzzer for that phase's grammar to find divergences the seed cases missed. Every divergence found becomes a new named regression case.
3. **Probe, don't guess.** Any uncertainty → minimal DRL probe → run through oracle → encode + regress.
4. **Prove order, not just state.** Assert the ordered firing log, not only the final fact set. A port can reach the right final state via the wrong agenda and be subtly, dangerously wrong.

---

## 6. Definition of Done (measurable)

- The Phase 1–2 subset is fully supported and every curated scenario passes at **100%** on both final-fact-set equivalence and firing-order equivalence.
- The property/fuzz suite generates random *in-subset* programs and asserts equivalence, run to **≥10,000 cases per phase with zero open divergences**.
- Any behavior that cannot be matched is isolated as a documented, `xfail`-marked regression with a minimal probe, and explicitly labeled out-of-subset in the README — **not** left as a silent mismatch.
- The suite doubles as executable documentation of the supported semantics.
- Repo satisfies every item in Section 8.

---

## 7. Guardrails & budget discipline (how not to sink the run)

- **Time-box every hard divergence.** If one divergence resists resolution past a fixed budget, convert it to a documented known-limitation (`xfail` + probe) and move on. Do not let one edge case eat the session.
- **Prefer proven-small over probably-large.** If forced to choose, shrink the subset and prove it, rather than widen and hope.
- **Never break the tree at a checkpoint.** Commit only green. If a change can't be finished cleanly, revert to the last green rather than leaving partial work.
- **Don't fabricate.** No invented semantics, no invented attributions, no plausible-looking-but-unverified behavior. The oracle is always one probe away — use it.
- **Log as you go** to `DECISIONS.md` so a human can review what was decided and why.

---

## 8. IP & licensing hygiene (load-bearing — the whole point of a clean first repo)

- License the port **Apache License 2.0**. Include a `LICENSE` file and a `NOTICE` file.
- **Preserve attribution:** retain the relevant copyright/patent/attribution notices; mark changed files as changed; carry forward the upstream `NOTICE` content that pertains to any ported portions. (Apache 2.0 §4.)
- **"Drools" is a trademark.** The copyright license does not grant trademark rights. **Do not name the project Drools.** Pick an original name; describe it factually as "a Rust port of a bounded subset of the Drools DRL forward-chaining semantics" / "DRL-compatible." (Suggestions to pick from or ignore: none imposed — your call.)
- **Keep the fact models and test rules generic.** Do **not** encode any third-party or employer business rules, schemas, or domain logic into the test corpus. Use invented, neutral domains (people/accounts/orders toys). Clean provenance means nothing proprietary leaks in.
- **Clean commit history:** meaningful commits, no vendored source with stripped headers, no copy-paste of upstream Java source into Rust files. Reimplement to match *behavior* (via the oracle), not by transliterating source.

---

## 9. Environment prerequisites (verify in the first 15 minutes)

- Rust stable toolchain (`rustc`, `cargo`).
- JVM 17+ and Maven or Gradle **with access to Maven Central** to resolve Drools (`drools-compiler` / `kie-ci`, a specific recent ASF/KIE-line version — record it). If the runtime is network-restricted, pre-vendor the Drools jar set instead.
- **Determinism:** pin the Drools version; fix locale to `en_US` (Drools has locale-sensitive behavior); seed the scenario generator so runs reproduce.
- Confirm the oracle works end-to-end (hello-world DRL through real Drools from Java, emitting canonical JSON) **before** writing any engine code.

---

## 10. First-hour checklist

1. Verify toolchains: `rustc`/`cargo`, `java`, `mvn`/`gradle`; resolve Drools; run one hello-world DRL through real Drools and emit canonical result JSON. **Oracle must work before anything else.**
2. Lock the canonical result-JSON schema (final fact set + ordered firing log). Both runners target it forever.
3. Scaffold the workspace; add `LICENSE` (Apache-2.0), `NOTICE`, `README` stub (subset + non-goals + provenance), `DECISIONS.md`. Name the crate (not "Drools").
4. Land the Phase 0 walking skeleton green end-to-end on both engines.
5. Only then begin Phase 1.

---

### Defaults chosen for you (override freely)
- Subset staged alpha → beta → mutation, with `not`/`exists`/`accumulate` as stretch. If you'd rather include one join in Phase 1 or defer `modify` semantics, adjust — but keep each phase's done-bar intact.
- Proof standard fixed at final-state **and** firing-order equivalence, ≥10k fuzz cases/phase. Loosening the firing-order requirement would materially weaken the proof; don't, unless deliberately.
