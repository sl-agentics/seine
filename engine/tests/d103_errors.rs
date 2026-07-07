//! D-103: positioned syntax errors — fail fast and loud.

use seine_engine::drl::parse_file;

fn err(src: &str) -> String {
    parse_file(src).unwrap_err().to_string()
}

#[test]
fn position_line_col_and_caret() {
    let e = err("rule R when Person(age > ) then end");
    assert!(e.contains("line 1"), "{e}");
    assert!(e.contains("col"), "{e}");
    assert!(e.contains('^'), "{e}");
    assert!(e.contains("Person(age > )"), "source line echoed: {e}");
}

#[test]
fn position_on_later_line() {
    let e = err("rule R\nwhen\n    Person(age > )\nthen\nend");
    assert!(e.contains("line 3"), "{e}");
    assert!(e.contains("Person(age > )"), "{e}");
}

#[test]
fn lexer_error_positioned() {
    let e = err("rule R when P(x == @) then end");
    assert!(e.contains("unexpected character"), "{e}");
    assert!(e.contains("line 1"), "{e}");
    assert!(e.contains('^'), "{e}");
}

#[test]
fn unterminated_string_positioned() {
    let e = err("rule \"R\" when P(name == \"oops) then end");
    assert!(e.contains("unterminated string"), "{e}");
    assert!(e.contains("line 1"), "{e}");
}

#[test]
fn eof_error_lands_on_last_token() {
    let e = err("rule R when P()");
    assert!(e.contains("line 1"), "{e}");
}

#[test]
fn semantic_walls_stay_loud_without_position() {
    // post-parse lowering errors carry the wall text (span optional)
    let e = err("rule R when not (exists (P() and Q())) then end");
    assert!(e.contains("out of subset"), "{e}");
}

#[test]
fn unit_walls_name_offending_rules() {
    let mut e = seine_engine::Engine::new(vec![seine_engine::TypeSchema {
        name: "P".into(),
        fields: vec![("v".into(), seine_engine::FieldType::I64)],
        nullable: 0,
    }])
    .unwrap();
    let src = r#"
query qp(long $v) P(v == $v) end
rule UsesQuery when P($x : v) ?qp($x;) then end
rule Mutates when $p : P(v > 0) then delete($p); end
"#;
    let err = e.add_rules_drl(src).unwrap_err().to_string();
    assert!(err.contains("UsesQuery"), "{err}");
    assert!(err.contains("Mutates"), "{err}");
}

#[test]
fn rule_scoped_compile_error_names_rule() {
    let mut e = seine_engine::Engine::new(vec![seine_engine::TypeSchema {
        name: "P".into(),
        fields: vec![("v".into(), seine_engine::FieldType::I64)],
        nullable: 0,
    }])
    .unwrap();
    let err = e
        .add_rules_drl("rule BadBind when P(v == $nope) then end")
        .unwrap_err()
        .to_string();
    assert!(err.contains("BadBind"), "{err}");
}
