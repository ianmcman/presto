mod common;
use common::{schema_names, is_forbidden};

#[test]
fn serialize_ctl_request() {
    let req = presto_ipc::ctl::CtlRequest {
        proto: 1,
        id: 7,
        op: presto_ipc::ctl::CtlOp::Seek { ms: 72000 },
    };
    let json = serde_json::to_string(&req).unwrap();
    assert_eq!(json, r#"{"proto":1,"id":7,"op":"seek","ms":72000}"#);
    let parsed: presto_ipc::ctl::CtlRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, req);
}

#[test]
fn deserialize_play_op() {
    let json = r#"{"proto":1,"id":1,"op":"play"}"#;
    let req: presto_ipc::ctl::CtlRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.id, 1);
    assert_eq!(req.op, presto_ipc::ctl::CtlOp::Play);
}

#[test]
fn schema_contains_expected_keys() {
    let req_names = schema_names::<presto_ipc::ctl::CtlRequest>();
    let reply_names = schema_names::<presto_ipc::ctl::CtlReply>();

    // Check CtlRequest has "subscribe" op
    assert!(req_names.contains("subscribe"), "CtlRequest missing subscribe op");

    // Check CtlReply has status fields
    for want in ["position_ms", "artwork_path", "engine"] {
        assert!(reply_names.contains(want), "CtlReply missing {want}: {reply_names:?}");
    }
}

#[test]
fn no_token_like_names() {
    let req_names = schema_names::<presto_ipc::ctl::CtlRequest>();
    let reply_names = schema_names::<presto_ipc::ctl::CtlReply>();

    let bad_req: Vec<_> = req_names.iter().filter(|x| is_forbidden(x)).collect();
    assert!(bad_req.is_empty(), "forbidden names in CtlRequest: {bad_req:?}");

    let bad_reply: Vec<_> = reply_names.iter().filter(|x| is_forbidden(x)).collect();
    assert!(bad_reply.is_empty(), "forbidden names in CtlReply: {bad_reply:?}");
}

#[test]
fn snapshot_ctl_request() {
    insta::assert_json_snapshot!(schemars::schema_for!(presto_ipc::ctl::CtlRequest));
}

#[test]
fn snapshot_ctl_reply() {
    insta::assert_json_snapshot!(schemars::schema_for!(presto_ipc::ctl::CtlReply));
}
