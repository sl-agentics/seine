//! D-076 design-constraint test: the justification graph is QUERYABLE,
//! not just internal retraction bookkeeping — this is the why-engine's
//! substrate ("what justifies this fact / what would have to change for
//! it to retract"). Behavior differentially certified by pr_tms_*; this
//! test pins the introspection SURFACE.

use seine_engine::{Engine, FieldType, TypeSchema, Value};

fn engine() -> Engine {
    let mut e = Engine::new(vec![
        TypeSchema { name: "A".into(), fields: vec![("a".into(), FieldType::I64)] },
        TypeSchema { name: "B".into(), fields: vec![("b".into(), FieldType::I64)] },
        TypeSchema { name: "LK".into(), fields: vec![("v".into(), FieldType::I64)] },
    ])
    .unwrap();
    e.add_rules_drl(
        "rule J1 when A($v : a) then insertLogical(new LK(9)); end\n\
         rule J2 when B() then insertLogical(new LK(9)); end\n",
    )
    .unwrap();
    e
}

#[test]
fn justification_graph_is_queryable() {
    let mut e = engine();
    let a1 = e.insert("A", vec![("a".into(), Value::I64(1))]).unwrap();
    let a2 = e.insert("A", vec![("a".into(), Value::I64(2))]).unwrap();
    e.insert("B", vec![("b".into(), Value::I64(1))]).unwrap();
    e.fire_all(100_000).unwrap();

    // One justified fact, three supports (J1 twice, J2 once), in
    // support order — the "why does this hold" answer.
    let js = e.justifications();
    assert_eq!(js.len(), 1);
    let j = &js[0];
    assert_eq!(j.rendering.type_name, "LK");
    assert_eq!(j.supports.len(), 3);
    assert_eq!(j.supports[0].rule, "J1");
    assert_eq!(j.supports[0].tuple, vec![a1]);
    assert_eq!(j.supports[1].rule, "J1");
    assert_eq!(j.supports[1].tuple, vec![a2]);
    assert_eq!(j.supports[2].rule, "J2");
    assert!(j.stated_siblings.is_empty());
    assert_eq!(e.why(j.fact).unwrap().supports.len(), 3);

    // Remove one support: the graph answers "what changed" — two
    // supports remain, the fact holds.
    e.delete_fact(a1).unwrap();
    e.fire_all(100_000).unwrap();
    let js = e.justifications();
    assert_eq!(js.len(), 1);
    assert_eq!(js[0].supports.len(), 2);

    // Remove the rest: the fact auto-retracts and the graph empties.
    e.delete_fact(a2).unwrap();
    e.fire_all(100_000).unwrap();
    let b = e
        .facts()
        .iter()
        .find(|f| f.type_name == "B")
        .map(|_| ())
        .unwrap();
    let _ = b;
    // B still alive keeps J2's support: LK still justified.
    assert_eq!(e.justifications().len(), 1);
    let bfact = e.nth_inserted(2).unwrap();
    e.delete_fact(bfact).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.justifications().is_empty());
    assert!(e.why(seine_engine::FactId(3)).is_none());
    assert!(!e.facts().iter().any(|f| f.type_name == "LK"));
}

#[test]
fn cascade_depth_is_rule_bounded_and_stack_safe() {
    // D-076 recursion accounting: cascade depth <= #rules x literal
    // combos (no arithmetic in the subset -> value chains cannot grow
    // unboundedly). This drives the deepest expressible shape: a
    // 12-rule chain L1->..->L12, then deletes the root support and
    // expects the WHOLE chain to retract in one cascade, cleanly.
    let mut schemas = vec![TypeSchema {
        name: "A".into(),
        fields: vec![("a".into(), FieldType::I64)],
    }];
    for i in 1..=12 {
        schemas.push(TypeSchema {
            name: format!("L{i}"),
            fields: vec![("v".into(), FieldType::I64)],
        });
    }
    let mut e = Engine::new(schemas).unwrap();
    let mut drl = String::from("rule R0 when A() then insertLogical(new L1(1)); end
");
    for i in 1..12 {
        drl.push_str(&format!(
            "rule R{i} when L{i}($v : v) then insertLogical(new L{}($v)); end
",
            i + 1
        ));
    }
    e.add_rules_drl(&drl).unwrap();
    let a = e.insert("A", vec![("a".into(), Value::I64(7))]).unwrap();
    e.fire_all(100_000).unwrap();
    assert_eq!(e.justifications().len(), 12, "chain fully justified");
    e.delete_fact(a).unwrap();
    e.fire_all(100_000).unwrap();
    assert!(e.justifications().is_empty(), "one cascade tears down the chain");
    assert!(e.facts().iter().all(|f| !f.type_name.starts_with('L')));
}
