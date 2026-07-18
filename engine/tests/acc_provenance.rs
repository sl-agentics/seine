//! D-305: accumulate-source provenance — the introspection channel
//! that closes the D-304 audit gap. The certified match tuple carries
//! the aggregation RESULT (Drools-faithful opacity); acc_sources(result)
//! answers "which facts summed into this", snapshotted at the
//! computation that produced the result's current value.

use seine_engine::{dec_parse, Engine, FactId, FieldType, Firing, TypeSchema, Value};

fn dec(s: &str) -> Value {
    let (u, sc) = dec_parse(s).expect("decimal literal");
    Value::Dec { u, s: sc }
}

fn result_handle(firings: &[Firing]) -> Option<FactId> {
    firings
        .iter()
        .flat_map(|f| f.matches.iter())
        .find(|m| m.type_name == "BigDecimal")
        .map(|m| FactId(m.handle))
}

fn engine() -> Engine {
    let mut e = Engine::new(vec![
        TypeSchema {
            name: "Line".into(),
            fields: vec![("amount".into(), FieldType::Dec { p: 18, s: 2 })],
            nullable: 0,
        },
        TypeSchema {
            name: "Bal".into(),
            fields: vec![("v".into(), FieldType::Dec { p: 38, s: 2 })],
            nullable: 0,
        },
    ])
    .unwrap();
    e.add_rules_drl(
        "rule Sum when accumulate( Line($a : amount); $t : sum($a) ) \
         then insert(new Bal($t)); end\n",
    )
    .unwrap();
    e
}

#[test]
fn sources_snapshot_matches_the_result() {
    let mut e = engine();
    let l1 = e.insert("Line", vec![("amount".into(), dec("100.10"))]).unwrap();
    let l2 = e.insert("Line", vec![("amount".into(), dec("50.20"))]).unwrap();
    let l3 = e.insert("Line", vec![("amount".into(), dec("-150.30"))]).unwrap();
    let firings = e.fire_all(100_000).unwrap();

    let result = result_handle(&firings).expect("acc result in the match tuple");
    let src = e.acc_sources(result).expect("provenance for the result");
    // MATCH order, not insertion order: staging is LIFO, so the batch
    // arrives newest-first — the snapshot mirrors the engine's own
    // accumulation order
    let handles: Vec<_> = src.iter().map(|(f, _)| *f).collect();
    assert_eq!(handles, vec![l3, l2, l1], "sources in match order");
    assert_eq!(src[0].1, dec("-150.30"));
    assert_eq!(src[2].1, dec("100.10"));

    // recompute: delete a line — the reused result fact re-snapshots
    e.delete_fact(l2).unwrap();
    e.fire_all(100_000).unwrap();
    let src = e.acc_sources(result).expect("recomputed provenance");
    let mut handles: Vec<_> = src.iter().map(|(f, _)| *f).collect();
    handles.sort_by_key(|f| f.0);
    assert_eq!(handles, vec![l1, l3], "the deleted line left the snapshot");

    // non-result and bogus handles answer None — never fabricate
    assert!(e.acc_sources(l1).is_none());
    assert!(e.acc_sources(FactId(41_414)).is_none());
}

#[test]
fn empty_source_is_an_empty_snapshot_not_none() {
    // sum over an empty source fires with identity 0 (ruling 2): the
    // honest provenance answer is "computed from nothing"
    let mut e = engine();
    let firings = e.fire_all(100_000).unwrap();
    let result = result_handle(&firings).expect("identity-0 result fires");
    assert_eq!(e.acc_sources(result), Some(&[][..]), "empty, not None");
}
