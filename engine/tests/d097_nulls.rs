//! D-097 conformance suite: SQL three-valued logic for nullable
//! fields, generated FROM the measured DuckDB pins
//! (docs/duckdb-datatype-pins.md, oracle duckdb 1.5.4). Every expected
//! count below mirrors a pin table row — pins A–H. Authority per
//! D-095 is the columnar ecosystem, NOT Drools.

use seine_engine::{Engine, FieldType, TypeSchema, Value};

/// T(v: i64 nullable, w: i64, s: String nullable)
fn engine(drl: &str) -> Engine {
    let mut e = Engine::new(vec![
        TypeSchema {
            name: "T".into(),
            fields: vec![
                ("v".into(), FieldType::I64),
                ("w".into(), FieldType::I64),
                ("s".into(), FieldType::Str),
            ],
            nullable: 0b101, // v and s nullable
        },
        TypeSchema {
            name: "U".into(),
            fields: vec![("k".into(), FieldType::I64)],
            nullable: 0b1,
        },
        TypeSchema { name: "Out".into(), fields: vec![], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl(drl).unwrap();
    e
}

fn t(e: &mut Engine, v: Option<i64>, w: i64, s: Option<&str>) {
    e.insert(
        "T",
        vec![
            ("v".into(), v.map(Value::I64).unwrap_or(Value::Null)),
            ("w".into(), Value::I64(w)),
            ("s".into(), s.map(|x| Value::Str(x.into())).unwrap_or(Value::Null)),
        ],
    )
    .unwrap();
}

fn fired(e: &mut Engine) -> usize {
    e.fire_all(100_000).unwrap().len()
}

/// Pin D: WHERE admits only TRUE — `v > 2` over [1, NULL, 5] -> one row;
/// and UNKNOWN is excluded from the NEGATION too.
#[test]
fn where_true_and_negation_exclude_unknown() {
    let mut e = engine("rule R when T(v > 2) then insert(new Out()); end");
    t(&mut e, Some(1), 0, None);
    t(&mut e, None, 0, None);
    t(&mut e, Some(5), 0, None);
    assert_eq!(fired(&mut e), 1, "pin D: only v=5 passes");

    let mut e = engine("rule R when T(!(v > 2)) then insert(new Out()); end");
    t(&mut e, Some(1), 0, None);
    t(&mut e, None, 0, None);
    t(&mut e, Some(5), 0, None);
    assert_eq!(fired(&mut e), 1, "pin D: NULL row excluded from NOT(v>2) as well");
}

/// Pin A: `= NULL` is UNKNOWN (never admits) for every operand — the
/// 3VL comparison arm; the SURFACE `== null` is IS NULL (definite).
#[test]
fn is_null_surface_vs_3vl_comparison() {
    // surface == null / != null (D-097 ruling 1)
    let mut e = engine("rule R when T(v == null) then insert(new Out()); end");
    t(&mut e, None, 0, None);
    t(&mut e, Some(1), 0, None);
    assert_eq!(fired(&mut e), 1, "IS NULL admits exactly the null row");

    let mut e = engine("rule R when T(v != null) then insert(new Out()); end");
    t(&mut e, None, 0, None);
    t(&mut e, Some(1), 0, None);
    assert_eq!(fired(&mut e), 1, "IS NOT NULL admits exactly the non-null row");

    // null-vs-null field JOIN comparison is UNKNOWN (pin A: NULL = NULL -> NULL)
    let mut e = engine(
        "rule R when T(w == 1, $av : v) T(w == 2, v == $av) then insert(new Out()); end",
    );
    t(&mut e, None, 1, None);
    t(&mut e, None, 2, None);
    assert_eq!(fired(&mut e), 0, "pin A/F: null v never equals null v");
}

/// Pin B: the 3VL tables through composite groups. `NULL AND FALSE =
/// FALSE` means a definite false disjunct still decides; `NULL OR TRUE
/// = TRUE` admits despite the unknown side.
#[test]
fn three_valued_connectives() {
    // v > 2 OR w == 7: unknown OR true -> TRUE
    let mut e = engine("rule R when T(v > 2 || w == 7) then insert(new Out()); end");
    t(&mut e, None, 7, None);
    assert_eq!(fired(&mut e), 1, "pin B: NULL OR TRUE = TRUE");

    // v > 2 OR w == 7: unknown OR false -> UNKNOWN (excluded)
    let mut e = engine("rule R when T(v > 2 || w == 0) then insert(new Out()); end");
    t(&mut e, None, 7, None);
    assert_eq!(fired(&mut e), 0, "pin B: NULL OR FALSE = NULL");

    // !(v > 2 && w == 7) with v null, w == 7: NOT(NULL AND TRUE) = NOT NULL = NULL
    let mut e = engine("rule R when T(!(v > 2 && w == 7)) then insert(new Out()); end");
    t(&mut e, None, 7, None);
    assert_eq!(fired(&mut e), 0, "pin B: NOT(UNKNOWN) = UNKNOWN");

    // !(v > 2 && w == 7) with v null, w != 7: NOT(NULL AND FALSE) = NOT FALSE = TRUE
    let mut e = engine("rule R when T(!(v > 2 && w == 7)) then insert(new Out()); end");
    t(&mut e, None, 0, None);
    assert_eq!(fired(&mut e), 1, "pin B: NULL AND FALSE = FALSE, negation admits");
}

/// Pin C: IN/NOT IN with nulls — the not-in trap.
#[test]
fn in_list_null_semantics() {
    // v IN (1, null): v=1 -> TRUE
    let mut e = engine("rule R when T(v in (1, null)) then insert(new Out()); end");
    t(&mut e, Some(1), 0, None);
    t(&mut e, Some(3), 0, None); // 3 IN (1, NULL) -> NULL
    t(&mut e, None, 0, None); // NULL IN (...) -> NULL
    assert_eq!(fired(&mut e), 1, "pin C: only the definite member admits");

    // v NOT IN (2, null) is never TRUE (the trap): 1 NOT IN (2, NULL) -> NULL
    let mut e = engine("rule R when T(v not in (2, null)) then insert(new Out()); end");
    t(&mut e, Some(1), 0, None);
    t(&mut e, Some(2), 0, None);
    assert_eq!(fired(&mut e), 0, "pin C: not-in with a null member never admits");

    // control: v NOT IN (2, 9) admits 1
    let mut e = engine("rule R when T(v not in (2, 9)) then insert(new Out()); end");
    t(&mut e, Some(1), 0, None);
    t(&mut e, None, 0, None); // NULL NOT IN (...) -> NULL
    assert_eq!(fired(&mut e), 1);
}

/// Pin E: string operators with a null subject are UNKNOWN.
#[test]
fn string_ops_null_subject() {
    let mut e = engine("rule R when T(s matches \"a.*\") then insert(new Out()); end");
    t(&mut e, None, 0, Some("abc"));
    t(&mut e, None, 0, None);
    assert_eq!(fired(&mut e), 1);

    let mut e = engine("rule R when T(s contains \"b\") then insert(new Out()); end");
    t(&mut e, None, 0, Some("abc"));
    t(&mut e, None, 0, None);
    assert_eq!(fired(&mut e), 1);
}

/// Pin F: null keys never equi-join — the eq-hash bucket path (the
/// beta-indexed variant of the pin-A join case, exercising keys_match).
#[test]
fn null_keys_never_equi_join() {
    let drl = "rule R when T(w == 1, $av : v) U(k == $av) then insert(new Out()); end";
    let mut e = engine(drl);
    t(&mut e, Some(1), 1, None);
    t(&mut e, None, 1, None);
    e.insert("U", vec![("k".into(), Value::I64(1))]).unwrap();
    e.insert("U", vec![("k".into(), Value::Null)]).unwrap();
    // only (v=1, k=1) pairs; (null,1) (1,null) (null,null) all UNKNOWN
    assert_eq!(fired(&mut e), 1, "pin F: exactly one join pair");
}

/// Pin G: aggregates skip null contributions; sum over all-null keeps
/// the D-097 ruling-2 result (0, fires — Drools engine axis).
#[test]
fn aggregates_skip_null_contributions() {
    let drl = "rule R when accumulate( T(w == 1, $x : v); $s : sum($x) ) then insert(new Out()); end";
    let mut e = engine(drl);
    t(&mut e, Some(1), 1, None);
    t(&mut e, None, 1, None);
    t(&mut e, Some(3), 1, None);
    let f = e.fire_all(100_000).unwrap();
    assert_eq!(f.len(), 1);
    // sum skips the null: 1 + 3 = 4
    let sv = &f[0].matches.iter().find(|m| m.type_name == "Long").unwrap().fields[0].1;
    assert_eq!(sv, &Value::I64(4), "pin G: sum([1, NULL, 3]) = 4");

    // all-null: sum = 0 and FIRES (D-097 ruling 2 — deliberate SQL deviation)
    let mut e = engine(drl);
    t(&mut e, None, 1, None);
    t(&mut e, None, 1, None);
    let f = e.fire_all(100_000).unwrap();
    assert_eq!(f.len(), 1, "D-097 ruling 2: all-null sum fires");
    let sv = &f[0].matches.iter().find(|m| m.type_name == "Long").unwrap().fields[0].1;
    assert_eq!(sv, &Value::I64(0), "D-097 ruling 2: all-null sum = 0");

    // min skips nulls; all-null min does NOT propagate (pin G == D-038)
    let drl = "rule R when accumulate( T(w == 1, $x : v); $m : min($x) ) then insert(new Out()); end";
    let mut e = engine(drl);
    t(&mut e, None, 1, None);
    t(&mut e, Some(5), 1, None);
    let f = e.fire_all(100_000).unwrap();
    assert_eq!(f.len(), 1);
    let mv = &f[0].matches.iter().find(|m| m.type_name == "Long").unwrap().fields[0].1;
    assert_eq!(mv, &Value::I64(5), "pin G: min skips the null");

    let mut e = engine(drl);
    t(&mut e, None, 1, None);
    assert_eq!(fired(&mut e), 0, "pin G: all-null min never propagates");
}

/// Walls (D-097): definite compile errors, loud runtime rejection.
#[test]
fn walls() {
    // relational op with null literal
    let mut e = Engine::new(vec![TypeSchema {
        name: "T".into(),
        fields: vec![("v".into(), FieldType::I64)],
        nullable: 1,
    }])
    .unwrap();
    let r = e.add_rules_drl("rule R when T(v > null) then end");
    assert!(r.is_err(), "only ==/!= accept null");

    // null test on a non-nullable field
    let mut e = Engine::new(vec![TypeSchema {
        name: "T".into(),
        fields: vec![("v".into(), FieldType::I64)],
        nullable: 0,
    }])
    .unwrap();
    let r = e.add_rules_drl("rule R when T(v == null) then end");
    assert!(r.is_err(), "null test needs a nullable field");

    // runtime: null into a non-nullable field is rejected loudly
    let r = e.insert("T", vec![("v".into(), Value::Null)]);
    assert!(r.is_err(), "store rejects null for non-nullable");
}
