//! Characterization test for the Phase 0 walking skeleton.
//! Golden values captured from Drools 9.44.0.Final via the oracle runner
//! (scenarios/phase0/p0_trivial_adult.json); see DECISIONS.md D-006.

use seine_engine::{Engine, FieldType, TypeSchema, Value};

#[test]
fn trivial_adult_matches_oracle_golden() {
    let mut e = Engine::new(vec![
        TypeSchema {
            name: "Person".into(),
            fields: vec![("name".into(), FieldType::Str), ("age".into(), FieldType::I64)],
            nullable: 0,
        },
        TypeSchema { name: "Adult".into(), fields: vec![("name".into(), FieldType::Str)], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl(
        "rule \"Adult\"\nwhen\n    $p : Person(age > 18)\nthen\n    insert(new Adult($p.getName()));\nend\n",
    )
    .unwrap();
    for (name, age) in [("alice", 30), ("bob", 10), ("carol", 45)] {
        e.insert(
            "Person",
            vec![("name".into(), Value::Str(name.into())), ("age".into(), Value::I64(age))],
        )
        .unwrap();
    }

    let firings = e.fire_all(100_000).unwrap();

    // Oracle golden: rule fired for alice then carol (insertion order).
    assert_eq!(firings.len(), 2);
    assert_eq!(firings[0].rule, "Adult");
    assert_eq!(firings[0].matches.len(), 1);
    assert_eq!(
        firings[0].matches[0].fields[0],
        ("name".into(), Value::Str("alice".into()))
    );
    assert_eq!(
        firings[1].matches[0].fields[0],
        ("name".into(), Value::Str("carol".into()))
    );

    // Oracle golden: final WM = 3 Persons + Adult(alice) + Adult(carol).
    let facts = e.facts();
    assert_eq!(facts.len(), 5);
    let adults: Vec<_> = facts.iter().filter(|f| f.type_name == "Adult").collect();
    assert_eq!(adults.len(), 2);
    assert!(adults
        .iter()
        .any(|f| f.fields[0] == ("name".into(), Value::Str("alice".into()))));
    assert!(adults
        .iter()
        .any(|f| f.fields[0] == ("name".into(), Value::Str("carol".into()))));
}
