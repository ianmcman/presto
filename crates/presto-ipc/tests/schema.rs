mod common;
use common::{is_forbidden, schema_names};

#[test]
fn snapshot() {
    insta::assert_json_snapshot!(schemars::schema_for!(presto_ipc::Frame));
}

#[test]
fn no_token_like_names() {
    let n = schema_names::<presto_ipc::Frame>();
    for want in ["position_ms", "seek", "queue_changed", "auth_expired"] {
        assert!(n.contains(want), "collector missed {want}: {n:?}");
    }
    let bad: Vec<_> = n.iter().filter(|x| is_forbidden(x)).collect();
    assert!(bad.is_empty(), "credential-like names in IPC types: {bad:?}");
}

#[test]
fn canary_catches_bearer() {
    #[allow(dead_code)]
    #[derive(schemars::JsonSchema)]
    struct Canary {
        bearer: String,
        music_user_token: String,
        author: String,
    }
    let flagged: Vec<_> = schema_names::<Canary>()
        .into_iter()
        .filter(|x| is_forbidden(x))
        .collect();
    assert_eq!(flagged, ["bearer", "music_user_token"]);
}

#[test]
fn all_names_lowercase() {
    let bad: Vec<_> = schema_names::<presto_ipc::Frame>()
        .into_iter()
        .filter(|x| x.bytes().any(|b| b.is_ascii_uppercase()))
        .collect();
    assert!(bad.is_empty(), "non-lowercase names: {bad:?}");
}
