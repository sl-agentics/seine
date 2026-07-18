//! D-314: averageExact — exact decimal average, java.math rounding.
//! The grid twins live in probes_pending/dec_avg/ (oracle programs
//! computing sum.divide(count, scale, mode) — value-for-value
//! receipts in PINS.md); these vectors pin OUR i128 division across
//! every mode and both signs.

use seine_engine::{Engine, FieldType, TypeSchema, Value};

fn avg(vals: &[&str], scale: u8, mode: &str) -> Option<String> {
    let mut e = Engine::new(vec![
        TypeSchema {
            name: "L".into(),
            fields: vec![("a".into(), FieldType::Dec { p: 10, s: 2 })],
            nullable: 0b1,
        },
        TypeSchema {
            name: "B".into(),
            fields: vec![("v".into(), FieldType::Dec { p: 38, s: scale })],
            nullable: 0,
        },
    ])
    .unwrap();
    e.add_rules_drl(&format!(
        "rule R when accumulate( L($a : a); $m : averageExact($a, {scale}, {mode}) ) \
         then insert(new B($m)); end"
    ))
    .unwrap();
    for v in vals {
        let val = if *v == "null" {
            Value::Null
        } else {
            let (u, s) = seine_engine::dec_parse(v).unwrap();
            Value::Dec { u, s }
        };
        e.insert("L", vec![("a".into(), val)]).unwrap();
    }
    e.fire_all(100_000).unwrap();
    e.facts().iter().find(|f| f.type_name == "B").map(|f| match &f.fields[0].1 {
        Value::Dec { u, s } => seine_engine::dec_render(*u, *s),
        other => panic!("not a decimal: {other:?}"),
    })
}

#[test]
fn the_rounding_grid() {
    // (vals, scale, [(mode, expected)]) — the PINS.md vectors
    let grid: &[(&[&str], u8, &[(&str, &str)])] = &[
        // V1: avg 0.025 — the positive half boundary
        (&["0.02", "0.03"], 2, &[
            ("up", "0.03"), ("down", "0.02"), ("ceiling", "0.03"),
            ("floor", "0.02"), ("half_up", "0.03"), ("half_down", "0.02"),
            ("half_even", "0.02"),
        ]),
        // V2: avg -0.025 — the negative half boundary
        (&["-0.02", "-0.03"], 2, &[
            ("up", "-0.03"), ("down", "-0.02"), ("ceiling", "-0.02"),
            ("floor", "-0.03"), ("half_up", "-0.03"), ("half_down", "-0.02"),
            ("half_even", "-0.02"),
        ]),
        // V3: 3.01/3 = 1.00333… — positive non-terminating
        (&["1.00", "1.00", "1.01"], 2, &[
            ("up", "1.01"), ("down", "1.00"), ("ceiling", "1.01"),
            ("floor", "1.00"), ("half_up", "1.00"), ("half_down", "1.00"),
            ("half_even", "1.00"),
        ]),
        // V4: the negative twin
        (&["-1.00", "-1.00", "-1.01"], 2, &[
            ("up", "-1.01"), ("down", "-1.00"), ("ceiling", "-1.00"),
            ("floor", "-1.01"), ("half_up", "-1.00"), ("half_down", "-1.00"),
            ("half_even", "-1.00"),
        ]),
        // V5: exact division — every mode is a no-op
        (&["1.10", "2.20"], 2, &[
            ("up", "1.65"), ("down", "1.65"), ("ceiling", "1.65"),
            ("floor", "1.65"), ("half_up", "1.65"), ("half_down", "1.65"),
            ("half_even", "1.65"),
        ]),
        // half_even parity: avg 0.035 → even neighbor is 0.04
        (&["0.03", "0.04"], 2, &[("half_even", "0.04"), ("half_up", "0.04")]),
        // scale narrowing: 0.025 at scale 0 is nowhere near half of 1
        (&["0.02", "0.03"], 0, &[("half_up", "0"), ("up", "1"), ("ceiling", "1")]),
        // scale widening: exact at 4 digits
        (&["0.02", "0.03"], 4, &[("half_up", "0.0250"), ("down", "0.0250")]),
    ];
    for (vals, scale, cells) in grid {
        for (mode, want) in *cells {
            let got = avg(vals, *scale, mode).expect("fires");
            assert_eq!(&got, want, "avg{vals:?} scale {scale} {mode}");
        }
    }
}

#[test]
fn nulls_skip_both_sum_and_count() {
    assert_eq!(avg(&["1.00", "null", "2.00"], 2, "half_up").unwrap(), "1.50");
}

#[test]
fn empty_and_all_null_block_propagation() {
    // like `average` (P2): count == 0 → the rule never fires
    assert!(avg(&[], 2, "half_up").is_none());
    assert!(avg(&["null", "null"], 2, "half_up").is_none());
}

#[test]
fn walls_are_loud() {
    let mk = || {
        Engine::new(vec![
            TypeSchema {
                name: "L".into(),
                fields: vec![("n".into(), FieldType::I64)],
                nullable: 0,
            },
        ])
        .unwrap()
    };
    let err = mk()
        .add_rules_drl("rule R when accumulate( L($n : n); $m : averageExact($n, 2, half_up) ) then end")
        .expect_err("i64 source walled")
        .0;
    assert!(err.contains("decimal source"), "{err}");
    let err = mk()
        .add_rules_drl("rule R when accumulate( L($n : n); $m : averageExact($n, 2, nearest) ) then end")
        .expect_err("unknown mode walled")
        .0;
    assert!(err.contains("half_even"), "{err}");
    let err = mk()
        .add_rules_drl("rule R when accumulate( L($n : n); $m : averageExact($n, 39, half_up) ) then end")
        .expect_err("scale > 38 walled")
        .0;
    assert!(err.contains("0..=38"), "{err}");
}
