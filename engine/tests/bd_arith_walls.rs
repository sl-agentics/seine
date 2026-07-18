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
