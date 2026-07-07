//! D-057 walls (as amended by D-107): out-of-subset ?query-CE SHAPES
//! stay compile errors; the qce x mutation walls are LIFTED — the qmut
//! ladder pinned pull-at-activation semantics and mutation composes.

use seine_engine::{Engine, FieldType, TypeSchema, Value};

fn engine() -> Engine {
    Engine::new(vec![
        TypeSchema { name: "A".into(), fields: vec![("id".into(), FieldType::I64)], nullable: 0 },
        TypeSchema { name: "B".into(), fields: vec![("k".into(), FieldType::I64)], nullable: 0 },
        TypeSchema { name: "Out".into(), fields: vec![("v".into(), FieldType::I64)], nullable: 0 },
    ])
    .unwrap()
}

const BVALS: &str = "query BVals(long $v)\n    B(k == $v)\nend\n";

#[test]
fn q2_pull_ce_fires_per_row() {
    let mut e = engine();
    e.add_rules_drl(&format!(
        "{BVALS}\nrule R1\nwhen\n    $a : A()\n    ?BVals($x;)\nthen\n    insert(new Out($x));\nend\n"
    ))
    .unwrap();
    e.insert("A", vec![("id".into(), Value::I64(1))]).unwrap();
    for k in [10, 20, 30] {
        e.insert("B", vec![("k".into(), Value::I64(k))]).unwrap();
    }
    let firings = e.fire_all(100_000).unwrap();
    assert_eq!(firings.len(), 3);
    // the CE contributes a QueryArgs element with the row value
    assert_eq!(firings[0].matches[1].type_name, "QueryArgs");
    // rows never appear in the final fact set
    assert!(e.facts().iter().all(|f| !f.type_name.starts_with("__qrow$")));
}

#[test]
fn q2_walls_reject() {
    // push (reactive) form: positional body dies at parse ...
    assert!(engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    $a : A()\n    BVals($x;)\nthen\n    insert(new Out($x));\nend\n"
        ))
        .is_err());
    // ... and a pattern-shaped call dies at compile with the pointed wall
    let err = engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    $a : A()\n    BVals()\nthen\n    insert(new Out(1));\nend\n"
        ))
        .unwrap_err();
    assert!(err.to_string().contains("push"), "{err}");
    // D-107: ?query CE mixed with mutation actions now COMPILES (the
    // qmut ladder pinned pull-at-activation; mutation composes)
    assert!(engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    ?BVals($x;)\nthen\n    insert(new Out($x));\nend\n\nrule R2\nwhen\n    $a : A()\nthen\n    delete($a);\nend\n"
        ))
        .is_ok());
    // ?query CE mixed with not/exists in the same rule
    let err = engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    $a : A()\n    not B(k == 1)\n    ?BVals($x;)\nthen\n    insert(new Out($x));\nend\n"
        ))
        .unwrap_err();
    assert!(err.to_string().contains("D-057"), "{err}");
    // ?query inside not
    let err = engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    $a : A()\n    not ?BVals($x;)\nthen\n    insert(new Out(1));\nend\n"
        ))
        .unwrap_err();
    assert!(err.to_string().contains("D-057"), "{err}");
    // CE-bound var in a salience expression
    let err = engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1 salience($x)\nwhen\n    $a : A()\n    ?BVals($x;)\nthen\n    insert(new Out($x));\nend\n"
        ))
        .unwrap_err();
    assert!(err.to_string().contains("salience"), "{err}");
    // arity mismatch
    let err = engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    $a : A()\n    ?BVals($x, $y;)\nthen\n    insert(new Out($x));\nend\n"
        ))
        .unwrap_err();
    assert!(err.to_string().contains("args"), "{err}");
    // literal arg type mismatch (exact match required)
    let err = engine()
        .add_rules_drl(&format!(
            "{BVALS}\nrule R1\nwhen\n    $a : A()\n    ?BVals(1.5;)\nthen\n    insert(new Out(1));\nend\n"
        ))
        .unwrap_err();
    assert!(err.to_string().contains("exactly"), "{err}");
}

#[test]
fn q2_external_mutation_composes() {
    // D-107: external mutation with resident ?query CEs is IN subset —
    // pull-at-activation (the qm2/qm3 pins: churn on the queried side
    // does not re-pull existing matches).
    let mut e = engine();
    e.add_rules_drl(&format!(
        "{BVALS}\nrule R1\nwhen\n    $a : A()\n    ?BVals($x;)\nthen\n    insert(new Out($x));\nend\n"
    ))
    .unwrap();
    let h = e.insert("A", vec![("id".into(), Value::I64(1))]).unwrap();
    e.fire_all(100_000).unwrap();
    e.update_fact(h, vec![("id".into(), Value::I64(2))]).unwrap();
    e.fire_all(100_000).unwrap();
    e.delete_fact(h).unwrap();
    e.fire_all(100_000).unwrap();
}
