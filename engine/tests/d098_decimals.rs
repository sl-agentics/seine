//! D-098 conformance suite: exact decimals per the measured DuckDB
//! pins (docs/duckdb-datatype-pins.md, pin J) — Arrow
//! Decimal128-compatible i128 scaled fixed-point, cross-scale value
//! comparison, half-up ingest rounding, loud overflow, exact
//! aggregation, and the D-097 ruling-4 decimal-vs-f64 WALL.

use seine_engine::{Engine, FieldType, TypeSchema, Value};

fn dec(u: i128, s: u8) -> Value {
    Value::Dec { u, s }
}

/// M(amount: decimal(10,2), tag: i64, opt: decimal(10,2) nullable)
fn engine(drl: &str) -> Engine {
    let mut e = Engine::new(vec![
        TypeSchema {
            name: "M".into(),
            fields: vec![
                ("amount".into(), FieldType::Dec { p: 10, s: 2 }),
                ("tag".into(), FieldType::I64),
                ("opt".into(), FieldType::Dec { p: 10, s: 2 }),
            ],
            nullable: 0b100,
        },
        TypeSchema { name: "Out".into(), fields: vec![], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl(drl).unwrap();
    e
}

fn engine_err(drl: &str) -> String {
    let mut e = Engine::new(vec![
        TypeSchema {
            name: "M".into(),
            fields: vec![
                ("amount".into(), FieldType::Dec { p: 10, s: 2 }),
                ("tag".into(), FieldType::I64),
                ("opt".into(), FieldType::Dec { p: 10, s: 2 }),
            ],
            nullable: 0b100,
        },
        TypeSchema { name: "Out".into(), fields: vec![], nullable: 0 },
    ])
    .unwrap();
    e.add_rules_drl(drl).expect_err("must wall").0
}

fn m(e: &mut Engine, amount: &str, tag: i64, opt: Option<&str>) {
    e.insert(
        "M",
        vec![
            ("amount".into(), Value::Str(amount.into())),
            ("tag".into(), Value::I64(tag)),
            ("opt".into(), opt.map(|x| Value::Str(x.into())).unwrap_or(Value::Null)),
        ],
    )
    .unwrap();
}

fn fired(e: &mut Engine) -> usize {
    e.fire_all(100_000).unwrap().len()
}

/// Pin J: comparisons are exact and value-based; literals written in
/// DRL recover exactly; decimal-vs-i64 compares exactly.
#[test]
fn exact_value_comparisons() {
    let mut e = engine("rule R when M(amount == 1.25) then end");
    m(&mut e, "1.25", 0, None);
    m(&mut e, "1.24", 0, None);
    assert_eq!(fired(&mut e), 1, "exact eq at scale");

    // the 0.1 + 0.2 class: 0.30 stored exactly, compared exactly
    let mut e = engine("rule R when M(amount == 0.30) then end");
    m(&mut e, "0.30", 0, None);
    assert_eq!(fired(&mut e), 1, "no IEEE residue");

    // decimal vs integer literal: 2.00 == 2
    let mut e = engine("rule R when M(amount == 2) then end");
    m(&mut e, "2.00", 0, None);
    m(&mut e, "2.01", 0, None);
    assert_eq!(fired(&mut e), 1, "pin J: decimal-integer value equality");

    // relational
    let mut e = engine("rule R when M(amount > 1.5) then end");
    m(&mut e, "1.51", 0, None);
    m(&mut e, "1.50", 0, None);
    m(&mut e, "1.49", 0, None);
    assert_eq!(fired(&mut e), 1);
}

/// Pin J: cross-scale VALUE equality on join keys (1.10 == 1.1-style)
/// — two decimal(10,2) facts joining on a decimal field, plus the
/// TMS-adjacent normalization (via the eq-hash-excluded plain path).
#[test]
fn cross_scale_join_equality() {
    let drl = "rule R when M(tag == 1, $a : amount) M(tag == 2, amount == $a) then end";
    let mut e = engine(drl);
    m(&mut e, "1.5", 1, None); // ingests at scale 2 -> 1.50
    m(&mut e, "1.50", 2, None);
    assert_eq!(fired(&mut e), 1, "value-equal across written scales");
}

/// VERBATIM ingest (MEASURED, D-315 p1/p2: the oracle's setTyped is
/// `new BigDecimal(text)` — the string's own scale survives, nothing
/// is ever half-up'd into the declared scale); the loud precision
/// overflow and the float rejection stay.
#[test]
fn ingest_rounding_and_overflow() {
    let mut e = engine("rule R when M(amount == 1.005) then end");
    m(&mut e, "1.005", 0, None); // stays scale 3, verbatim
    assert_eq!(fired(&mut e), 1, "D-315: verbatim, 1.005 stays 1.005");

    let mut e = engine("rule R when M(amount == 1.01) then end");
    m(&mut e, "1.005", 0, None);
    assert_eq!(fired(&mut e), 0, "D-315: no half-up rescale on ingest");

    let mut e = engine("rule R when M(tag == 0) then end");
    let r = e.insert(
        "M",
        vec![
            ("amount".into(), Value::Str("123456789.00".into())), // 11 digits > p=10
            ("tag".into(), Value::I64(0)),
            ("opt".into(), Value::Null),
        ],
    );
    assert!(r.is_err(), "precision overflow errors loudly (pin J)");

    // JSON/IEEE floats are rejected for decimal fields
    let r = e.insert(
        "M",
        vec![
            ("amount".into(), Value::F64(1.25)),
            ("tag".into(), Value::I64(0)),
            ("opt".into(), Value::Null),
        ],
    );
    assert!(r.is_err(), "floats never ingest into decimals");
}

/// D-097 ruling 4: decimal-vs-f64 is a COMPILE error.
#[test]
fn decimal_f64_wall() {
    let mut e = Engine::new(vec![TypeSchema {
        name: "W".into(),
        fields: vec![
            ("d".into(), FieldType::Dec { p: 6, s: 2 }),
            ("f".into(), FieldType::F64),
        ],
        nullable: 0,
    }])
    .unwrap();
    let r = e.add_rules_drl("rule R when W($x : f) W(d == $x) then end");
    assert!(r.is_err(), "decimal-vs-double binding comparison walled");
    let msg = format!("{:?}", r.err().unwrap());
    assert!(msg.contains("WALLED"), "the wall names itself: {msg}");
}

/// Pin J aggregates: sum exact (and widening), average -> DOUBLE,
/// min/max preserve the decimal; nulls skip (D-097 composition);
/// an empty/all-null sum FIRES with BigDecimal.ZERO — scale 0, "0"
/// (MEASURED, D-313 fuzz: the old at-the-field's-scale value was a
/// ruling-2 composition the oracle falsified).
#[test]
fn decimal_aggregates() {
    let drl = "rule R when accumulate( M(tag == 1, $x : amount); $s : sum($x) ) then end";
    let mut e = engine(drl);
    m(&mut e, "0.10", 1, None);
    m(&mut e, "0.20", 1, None);
    m(&mut e, "0.30", 1, None);
    let f = e.fire_all(100_000).unwrap();
    assert_eq!(f.len(), 1);
    let sv = &f[0].matches.iter().find(|x| x.type_name == "BigDecimal").unwrap().fields[0].1;
    assert_eq!(sv, &dec(60, 2), "pin J: exact sum 0.60");

    // average -> f64 (pin J: AVG is DOUBLE)
    let drl = "rule R when accumulate( M(tag == 1, $x : amount); $a : average($x) ) then end";
    let mut e = engine(drl);
    m(&mut e, "1.00", 1, None);
    m(&mut e, "2.00", 1, None);
    let f = e.fire_all(100_000).unwrap();
    let av = &f[0].matches.iter().find(|x| x.type_name == "Double").unwrap().fields[0].1;
    assert_eq!(av, &Value::F64(1.5), "pin J: AVG(decimal) is DOUBLE");

    // min preserves decimal; null contributions skip
    let drl = "rule R when accumulate( M(tag == 1, $x : opt); $m : min($x) ) then end";
    let mut e = engine(drl);
    m(&mut e, "0.00", 1, None);
    m(&mut e, "0.00", 1, Some("2.50"));
    let f = e.fire_all(100_000).unwrap();
    let mv = &f[0].matches.iter().find(|x| x.type_name == "BigDecimal").unwrap().fields[0].1;
    assert_eq!(mv, &dec(250, 2), "min skips the null, preserves decimal");

    // all-null sum: fires with BigDecimal.ZERO — scale 0 (measured,
    // D-313; the fold ratchets scale only on real contributions)
    let drl = "rule R when accumulate( M(tag == 1, $x : opt); $s : sum($x) ) then end";
    let mut e = engine(drl);
    m(&mut e, "0.00", 1, None);
    let f = e.fire_all(100_000).unwrap();
    assert_eq!(f.len(), 1, "still fires on all-null");
    let sv = &f[0].matches.iter().find(|x| x.type_name == "BigDecimal").unwrap().fields[0].1;
    assert_eq!(sv, &dec(0, 0), "BigDecimal.ZERO: scale 0, like the oracle");
}

/// in-lists over decimals convert exactly; RHS numeric literals into
/// decimal fields are ERROR PARITY (MEASURED, D-315 p4_lit: javac
/// rejects the BigDecimal constructor/setter — bindings and ingested
/// data are the decimal routes).
#[test]
fn decimal_lists_and_rhs() {
    let mut e = engine("rule R when M(amount in (1.25, 3.5)) then end");
    m(&mut e, "1.25", 0, None);
    m(&mut e, "3.50", 0, None);
    m(&mut e, "2.00", 0, None);
    assert_eq!(fired(&mut e), 2, "in-list decimal membership by value");

    let err = engine_err(
        "rule A when M(tag == 7) then insert(new M(2.5, 8, null)); end",
    );
    assert!(err.contains("error parity"), "{err}");
    let err = engine_err(
        "rule A when $m : M(tag == 7) then modify($m) { setAmount(2.5) } end",
    );
    assert!(err.contains("error parity"), "{err}");
}
