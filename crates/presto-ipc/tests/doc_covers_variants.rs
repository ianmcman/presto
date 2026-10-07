mod common;

const DOC: &str = include_str!("../../../docs/PROTOCOL.md");

#[test]
fn doc_mentions_every_wire_name() {
    let missing: Vec<_> = common::schema_names::<presto_ipc::Frame>()
        .into_iter()
        .filter(|n| !DOC.contains(&format!("`{n}`")))
        .collect();
    assert!(missing.is_empty(), "PROTOCOL.md is missing: {missing:?}");
}

#[test]
fn doc_states_version() {
    let v = presto_ipc::PROTO;
    assert!(DOC.contains(&format!("`{}.{}`", v.major, v.minor)));
}
