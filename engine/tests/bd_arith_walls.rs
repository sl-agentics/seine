//! D-309: the decimal-arithmetic fences, pinned loud. Every cell here
//! is a POISON or degradation shape the D-308 campaign measured in the
//! oracle — the walls exist so no rule can depend on them.

use seine_engine::{Engine, FieldType, TypeSchema};

fn engine() -> Engine {
    Engine::new(vec![TypeSchema {
        name: "M".into(),
        fields: vec![
            ("a".into(), FieldType::Dec { p: 10, s: 2 }),
            ("b".into(), FieldType::Dec { p: 10, s: 2 }),
            ("d".into(), FieldType::F64),
            ("opt".into(), FieldType::Dec { p: 10, s: 2 }),
        ],
        nullable: 0b1000,
    }])
    .unwrap()
}

fn wall(drl: &str, needle: &str) {
    let mut e = engine();
    let err = e.add_rules_drl(drl).expect_err(drl).0;
    assert!(err.contains(needle), "{drl}\n  got: {err}\n  wanted: {needle}");
}

#[test]
fn division_and_rem_are_fenced() {
    wall("rule R when M(a / b > 0) then end", "silently degrades");
    wall("rule R when M(a % b == 0) then end", "silently degrades");
}

#[test]
fn doubles_in_decimal_arith_are_fenced() {
    // a double FIELD inside the expression
    wall("rule R when M(a + d > 0) then end", "coerces double literals RAW-BINARY");
    // a double LITERAL as the comparand — the measured poison
    wall("rule R when M(a + b == 3.30) then end", "measured grid");
    wall("rule R when M(a + b <= 3.30) then end", "measured grid");
}

#[test]
fn nullable_decimal_arith_is_walled() {
    wall("rule R when M(a + opt > 0) then end", "NULLABLE");
}

#[test]
fn rhs_decimal_arith_stays_error_parity() {
    // the oracle build-errors here too (D-308 A-cells)
    wall(
        "rule R when M($p : a, $f : b) then insert(new M($p + $f, $f, 0.0, $f)); end",
        "arithmetic is i64/f64 only",
    );
}

#[test]
fn in_subset_shapes_compile() {
    let mut e = engine();
    e.add_rules_drl(
        "rule R1 when M(a + b >= b) then end\n\
         rule R2 when M(a - b == 0) then end\n\
         rule R3 when M(a * b > 3) then end\n",
    )
    .expect("the D-309 agree subset compiles");
}

#[test]
fn overflow_is_a_typed_error_not_a_panic() {
    // D-310 (the adversary's split gate): the pin-J ceiling surfaces as
    // an EngineError at the API boundary — catchable, no unwinding —
    // and the engine stays usable afterwards.
    let mut e = Engine::new(vec![TypeSchema {
        name: "B".into(),
        fields: vec![
            ("a".into(), FieldType::Dec { p: 38, s: 0 }),
            ("b".into(), FieldType::Dec { p: 38, s: 0 }),
        ],
        nullable: 0,
    }])
    .unwrap();
    e.add_rules_drl("rule R when B(a * b >= a) then end").unwrap();
    let big: i128 = 10i128.pow(30);
    e.insert("B", vec![
        ("a".into(), seine_engine::Value::Dec { u: big, s: 0 }),
        ("b".into(), seine_engine::Value::Dec { u: big, s: 0 }),
    ])
    .unwrap();
    let err = e.fire_all(100_000).expect_err("overflow must error").0;
    assert!(err.contains("overflow past DECIMAL(38)"), "{err}");
    // the session recovers: a clean fact evaluates fine afterwards
    e.insert("B", vec![
        ("a".into(), seine_engine::Value::Dec { u: 2, s: 0 }),
        ("b".into(), seine_engine::Value::Dec { u: 3, s: 0 }),
    ])
    .unwrap();
    assert!(e.fire_all(100_000).is_ok(), "usable after the typed error");
}

#[test]
fn sum_overflow_is_a_typed_error_too() {
    // the balance-gate critical path: accumulate sum past DECIMAL(38)
    let mut e = Engine::new(vec![TypeSchema {
        name: "B".into(),
        fields: vec![("a".into(), FieldType::Dec { p: 38, s: 0 })],
        nullable: 0,
    }])
    .unwrap();
    e.add_rules_drl(
        "rule S when accumulate( B($x : a); $t : sum($x) ) then end",
    )
    .unwrap();
    let big: i128 = 99_999_999_999_999_999_999_999_999_999_999_999_999i128;
    for _ in 0..2 {
        e.insert("B", vec![("a".into(), seine_engine::Value::Dec { u: big, s: 0 })])
            .unwrap();
    }
    let err = e.fire_all(100_000).expect_err("sum overflow must error").0;
    assert!(err.contains("sum overflow"), "{err}");
}
