//! D-302 adversarial round for the queryable justification graph
//! (`Engine::why` / `Engine::justifications`, the D-076 why-engine
//! substrate): the original pin (tms_queryable.rs) predates the D-296
//! lift and the D-293/297/298 TMS rebuilds, so nothing exercised the
//! graph over the NEW in-subset shapes. These tests ask the adversarial
//! questions: does why() explain an ungrounded orphan cycle, stay
//! truthful link-by-link at post-lift depth, re-root its answer on
//! supersede, and track pending-on-stated materialization?

use seine_engine::{Engine, FieldType, TypeSchema, Value};

fn i64f(n: &str) -> (String, FieldType) {
    (n.into(), FieldType::I64)
}

/// D-294 pin: an all-logical mutual-support cluster SURVIVES root
/// deletion (support counting, not well-foundedness). The why-engine's
/// honesty claim is that the graph EXPLAINS the orphan: each member's
/// only remaining support is the other member's rule and tuple.
#[test]
fn ungrounded_cluster_reports_its_cycle() {
    let mut e = Engine::new(vec![
        TypeSchema { name: "Root".into(), fields: vec![i64f("r")], nullable: 0 },
        TypeSchema { name: "M1".into(), fields: vec![i64f("v")], nullable: 0 },
        TypeSchema { name: "M2".into(), fields: vec![i64f("v")], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl(
        "rule Seed when Root() then insertLogical(new M1(1)); end\n\
         rule R12 when M1($v : v) then insertLogical(new M2($v)); end\n\
         rule R21 when M2($v : v) then insertLogical(new M1($v)); end\n",
    )
    .unwrap();
    let root = e.insert("Root", vec![("r".into(), Value::I64(0))]).unwrap();
    e.fire_all(100_000).unwrap();

    let js = e.justifications();
    assert_eq!(js.len(), 2, "M1(1) and M2(1) are justified");
    let m1 = js.iter().find(|j| j.rendering.type_name == "M1").unwrap();
    let m2 = js.iter().find(|j| j.rendering.type_name == "M2").unwrap();
    // grounded phase: M1 carries Seed's support AND R21's
    assert_eq!(m1.supports.len(), 2);
    assert!(m1.supports.iter().any(|s| s.rule == "Seed"));
    assert!(m1.supports.iter().any(|s| s.rule == "R21" && s.tuple == vec![m2.fact]));
    assert_eq!(m2.supports.len(), 1);
    assert_eq!(m2.supports[0].rule, "R12");
    assert_eq!(m2.supports[0].tuple, vec![m1.fact]);

    // delete the grounded root: the cluster survives (the certified
    // orphan), and why() explains it — each member supported only by
    // the other
    e.delete_fact(root).unwrap();
    e.fire_all(100_000).unwrap();
    let m1w = e.why(m1.fact).expect("M1 survives ungrounded");
    let m2w = e.why(m2.fact).expect("M2 survives ungrounded");
    assert_eq!(m1w.supports.len(), 1, "Seed's support is gone");
    assert_eq!(m1w.supports[0].rule, "R21");
    assert_eq!(m1w.supports[0].tuple, vec![m2.fact]);
    assert_eq!(m2w.supports[0].rule, "R12");
    assert_eq!(m2w.supports[0].tuple, vec![m1.fact]);
}

/// Post-lift depth: a 400-deep computed chain (D-283 RHS arithmetic +
/// D-296 in-subset growth), every link queryable, parent links walkable
/// root-to-tip by tuple, and one root deletion drains the whole graph
/// (through the D-293 worklist over the D-297/298 indexes).
#[test]
fn deep_chain_every_link_queryable_and_walkable() {
    let mut e = Engine::new(vec![TypeSchema {
        name: "T".into(),
        fields: vec![i64f("n")],
        nullable: 0,
    }])
    .unwrap();
    e.add_rules_drl(
        "rule G when T($n : n, n < 400) then insertLogical(new T($n + 1)); end\n",
    )
    .unwrap();
    let t0 = e.insert("T", vec![("n".into(), Value::I64(0))]).unwrap();
    e.fire_all(100_000).unwrap();

    let js = e.justifications();
    assert_eq!(js.len(), 400, "T(1)..=T(400) all justified");
    for j in &js {
        assert_eq!(j.supports.len(), 1);
        assert_eq!(j.supports[0].rule, "G");
    }
    // walk tip -> root by support tuples, checking the value decrements
    let val = |e: &Engine, f| match e.why(f).unwrap().rendering.fields[0].1 {
        Value::I64(n) => n,
        _ => panic!("i64 field"),
    };
    let mut cur = js
        .iter()
        .find(|j| matches!(j.rendering.fields[0].1, Value::I64(400)))
        .unwrap()
        .fact;
    let mut hops = 0;
    loop {
        let w = e.why(cur).unwrap();
        let parent = w.supports[0].tuple[0];
        hops += 1;
        if parent == t0 {
            break;
        }
        assert_eq!(val(&e, parent) + 1, val(&e, cur), "parent is n-1");
        cur = parent;
    }
    assert_eq!(hops, 400, "the walk visits every link exactly once");

    e.delete_fact(t0).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.justifications().is_empty(), "one cascade drains 400 deep");
}

/// Supersede / re-establishment: after every support is gone and the
/// key re-derives, why() must answer with the FRESH support and a
/// FRESH justified fact — never the retracted one.
#[test]
fn reestablished_key_reroots_the_answer() {
    let mut e = Engine::new(vec![
        TypeSchema { name: "A".into(), fields: vec![i64f("a")], nullable: 0 },
        TypeSchema { name: "B".into(), fields: vec![i64f("b")], nullable: 0 },
        TypeSchema { name: "LK".into(), fields: vec![i64f("v")], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl(
        "rule J1 when A() then insertLogical(new LK(9)); end\n\
         rule J2 when B() then insertLogical(new LK(9)); end\n",
    )
    .unwrap();
    let a = e.insert("A", vec![("a".into(), Value::I64(1))]).unwrap();
    e.fire_all(100_000).unwrap();
    let first = e.justifications()[0].fact;

    e.delete_fact(a).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.justifications().is_empty());
    assert!(e.why(first).is_none(), "the retracted handle answers None");

    let _b = e.insert("B", vec![("b".into(), Value::I64(2))]).unwrap();
    e.fire_all(100_000).unwrap();
    let js = e.justifications();
    assert_eq!(js.len(), 1);
    assert_ne!(js[0].fact, first, "re-establishment mints a fresh fact");
    assert_eq!(js[0].supports.len(), 1);
    assert_eq!(js[0].supports[0].rule, "J2");
    assert!(e.why(first).is_none(), "the old handle stays dead");
}

/// D-211 pending-on-stated, both directions. Stated first: the logical
/// lands PENDING (no justified fact — why() has nothing to say), and a
/// stated delete on the mixed key KILLS THE KEY WHOLE (the r1
/// mixed-key-kill pin, D-203..211): the pending belief unstages,
/// nothing materializes, and with no re-fire on stated the value is
/// simply gone — the graph must not resurrect an unstaged belief.
/// Justified first: a stated twin lists as a LIVE sibling and drops
/// out when it dies.
#[test]
fn pending_on_stated_and_sibling_liveness() {
    let build = || {
        let mut e = Engine::new(vec![
            TypeSchema { name: "A".into(), fields: vec![i64f("a")], nullable: 0 },
            TypeSchema { name: "LK".into(), fields: vec![i64f("v")], nullable: 0 },
        ])
        .unwrap();
        e.add_rules_drl("rule J1 when A() then insertLogical(new LK(9)); end\n").unwrap();
        e
    };

    // stated FIRST: the logical lands pending — the graph is empty
    let mut e = build();
    let stated = e.insert("LK", vec![("v".into(), Value::I64(9))]).unwrap();
    e.insert("A", vec![("a".into(), Value::I64(1))]).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.justifications().is_empty(), "pending on stated: nothing justified");
    assert!(e.why(stated).is_none(), "a stated fact is not a justified fact");
    // stated delete on the mixed key: the key dies WHOLE — the pending
    // belief unstages (r1 pin) and no justified fact appears
    e.delete_fact(stated).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.justifications().is_empty(), "unstaged belief must not resurrect");
    assert!(
        !e.facts().iter().any(|f| f.type_name == "LK"),
        "the mixed key died whole: no LK in working memory"
    );

    // justified FIRST: the stated twin lists as a LIVE sibling only
    let mut e = build();
    e.insert("A", vec![("a".into(), Value::I64(1))]).unwrap();
    e.fire_all(100_000).unwrap();
    let j = e.justifications()[0].fact;
    let stated = e.insert("LK", vec![("v".into(), Value::I64(9))]).unwrap();
    e.fire_all(100_000).unwrap();
    let w = e.why(j).expect("justified fact stays queryable");
    assert_eq!(w.stated_siblings, vec![stated]);
    e.delete_fact(stated).unwrap();
    e.fire_all(100_000).unwrap();
    if let Some(w) = e.why(j) {
        assert!(w.stated_siblings.is_empty(), "dead siblings are filtered");
    }
}

/// The negative space: unkeyed types, plain stated facts, and bogus
/// ids all answer None instead of fabricating a justification.
#[test]
fn unjustified_queries_answer_none() {
    let mut e = Engine::new(vec![
        TypeSchema { name: "A".into(), fields: vec![i64f("a")], nullable: 0 },
        TypeSchema { name: "LK".into(), fields: vec![i64f("v")], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl("rule J1 when A() then insertLogical(new LK(9)); end\n").unwrap();
    let a = e.insert("A", vec![("a".into(), Value::I64(1))]).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.why(a).is_none(), "a stated justifier is not itself justified");
    assert!(e.why(seine_engine::FactId(9_999)).is_none());
}
