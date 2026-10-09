use presto_ipc::*;
use serde_json::json;

fn rt(f: &Frame) {
    let s = serde_json::to_string(f).unwrap();
    assert_eq!(&serde_json::from_str::<Frame>(&s).unwrap(), f, "{s}");
}

#[test]
fn cdm_shape() {
    let e = Event::Cdm {
        state: CdmState::Checking,
        version: None,
        message: None,
    };
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({"type":"cdm","state":"checking","version":null,"message":null})
    );
}

#[test]
fn cmd_shape() {
    let f = Frame::Cmd {
        id: 7,
        cmd: Command::Seek { ms: 1000 },
    };
    assert_eq!(
        serde_json::to_value(&f).unwrap(),
        json!({"t":"cmd","id":7,"cmd":{"type":"seek","ms":1000}})
    );
}

#[test]
fn hello_shape() {
    let f = Frame::Hello(Hello::new(
        Role::Engine,
        &[caps::PLAYBACK],
        Some("x 1".into()),
    ));
    assert_eq!(
        serde_json::to_value(&f).unwrap(),
        json!({"t":"hello","proto":{"major":1,"minor":2},"role":"engine","capabilities":["playback"],"engine":"x 1"})
    );
}

#[test]
fn res_err_shape() {
    let f = Frame::Res {
        id: 3,
        outcome: Outcome::Err {
            error: IpcError {
                kind: ErrorKind::RateLimited {
                    retry_after_ms: Some(500),
                },
                message: "m".into(),
            },
        },
    };
    assert_eq!(
        serde_json::to_value(&f).unwrap(),
        json!({"t":"res","id":3,"outcome":{"status":"err","error":{"kind":{"code":"rate_limited","retry_after_ms":500},"message":"m"}}})
    );
}

fn item() -> QueueItem {
    QueueItem {
        id: "i".into(),
        title: "t".into(),
        artist: "a".into(),
        album: "b".into(),
        duration_ms: 1,
        artwork_url: Some("u".into()),
        playable: true,
    }
}

#[test]
fn all_variants_roundtrip() {
    let cmds = vec![
        Command::Play,
        Command::Pause,
        Command::Seek { ms: 1 },
        Command::Next,
        Command::Prev,
        Command::SetVolume { volume: 0.5 },
        Command::SetShuffle { on: true },
        Command::SetRepeat {
            mode: RepeatMode::All,
        },
        Command::SetQueue {
            ids: vec!["a".into()],
            start: 0,
            play: true,
        },
        Command::ShowWindow { show: false },
    ];
    let errs = vec![
        ErrorKind::Timeout,
        ErrorKind::AuthExpired,
        ErrorKind::RateLimited {
            retry_after_ms: None,
        },
        ErrorKind::RateLimited {
            retry_after_ms: Some(1),
        },
        ErrorKind::NotFound,
        ErrorKind::Unavailable,
        ErrorKind::Upstream { status: 502 },
        ErrorKind::Internal,
    ];
    let events = vec![
        Event::PlaybackState {
            state: PlayState::Playing,
            seq: 1,
        },
        Event::Progress {
            position_ms: 1,
            duration_ms: 2,
            seq: 1,
        },
        Event::TrackChanged { item: Some(item()) },
        Event::TrackChanged { item: None },
        Event::QueueChanged {
            rev: 1,
            items: vec![item()],
            index: Some(0),
        },
        Event::Volume { volume: 0.25 },
        Event::Shuffle { on: false },
        Event::Repeat {
            mode: RepeatMode::One,
        },
        Event::Auth {
            state: AuthState::Expired,
        },
        Event::Error {
            error: IpcError::new(ErrorKind::Internal, "x"),
        },
        Event::BridgeReady {
            version: "1.1.0".into(),
            capabilities: vec!["api".into()],
            musickit_build: Some("3.x".into()),
        },
        Event::BridgeReady {
            version: "1.1.0".into(),
            capabilities: vec![],
            musickit_build: None,
        },
        Event::Cdm {
            state: CdmState::Checking,
            version: None,
            message: None,
        },
        Event::Cdm {
            state: CdmState::Ready,
            version: Some("4.10.3112.0".into()),
            message: None,
        },
        Event::Cdm {
            state: CdmState::Failed,
            version: None,
            message: Some("timed out after 120 s".into()),
        },
    ];
    let faults = vec![
        FaultSpec::None,
        FaultSpec::Hang,
        FaultSpec::Crash { after_ms: Some(1) },
        FaultSpec::Crash { after_ms: None },
        FaultSpec::AuthExpired,
        FaultSpec::Slow { delay_ms: 5 },
    ];
    let mut frames = vec![
        Frame::Hello(Hello::new(Role::Presto, &[], None)),
        Frame::Req {
            id: 1,
            req: ApiRequest::get("/v1/x"),
        },
        Frame::Req {
            id: 2,
            req: ApiRequest {
                method: HttpMethod::Post,
                path: "/p".into(),
                query: [("k".to_string(), "v".to_string())].into(),
                body: Some(json!({"a":1})),
            },
        },
        Frame::Res {
            id: 1,
            outcome: Outcome::Ok { data: json!([1]) },
        },
        Frame::Ping { seq: 1 },
        Frame::Pong { seq: 1 },
    ];
    frames.extend(cmds.into_iter().map(|cmd| Frame::Cmd { id: 1, cmd }));
    frames.extend(errs.into_iter().map(|kind| Frame::Res {
        id: 1,
        outcome: Outcome::Err {
            error: IpcError::new(kind, "m"),
        },
    }));
    frames.extend(events.into_iter().map(|evt| Frame::Evt { evt }));
    frames.extend(faults.into_iter().map(|fault| Frame::Mock { id: 1, fault }));
    for f in &frames {
        rt(f);
    }
}

#[test]
fn version_check() {
    let mut h = Hello::new(Role::Engine, &[], None);
    h.proto = ProtoVersion { major: 2, minor: 0 };
    let e = h.check(PROTO).unwrap_err().to_string();
    assert!(e.contains("1.2") && e.contains("2.0"), "{e}");
    h.proto = ProtoVersion { major: 1, minor: 9 };
    assert!(h.check(PROTO).is_ok());
}

#[test]
fn fault_parse() {
    assert_eq!(
        "crash@5000".parse(),
        Ok(FaultSpec::Crash {
            after_ms: Some(5000)
        })
    );
    assert_eq!("slow".parse(), Ok(FaultSpec::Slow { delay_ms: 3000 }));
    assert_eq!("slow=1500".parse(), Ok(FaultSpec::Slow { delay_ms: 1500 }));
    for s in ["hang", "crash", "auth_expired", "none"] {
        assert!(s.parse::<FaultSpec>().is_ok(), "{s}");
    }
    assert!("bogus".parse::<FaultSpec>().is_err());
}

#[test]
fn kinds() {
    for k in [
        Kind::Hello,
        Kind::Command,
        Kind::SetQueue,
        Kind::ApiRead,
        Kind::ApiWrite,
        Kind::MockControl,
    ] {
        assert!(!k.timeout().is_zero());
    }
    let cmd = |cmd| Frame::Cmd { id: 1, cmd };
    assert_eq!(
        cmd(Command::SetQueue {
            ids: vec![],
            start: 0,
            play: true,
        })
        .kind(),
        Some(Kind::SetQueue)
    );
    assert_eq!(cmd(Command::Play).kind(), Some(Kind::Command));
    assert_eq!(
        Frame::Req {
            id: 1,
            req: ApiRequest::get("/")
        }
        .kind(),
        Some(Kind::ApiRead)
    );
    let mut post = ApiRequest::get("/");
    post.method = HttpMethod::Post;
    assert_eq!(Frame::Req { id: 1, req: post }.kind(), Some(Kind::ApiWrite));
    assert_eq!(
        Frame::Hello(Hello::new(Role::Presto, &[], None)).kind(),
        Some(Kind::Hello)
    );
    assert_eq!(
        Frame::Mock {
            id: 1,
            fault: FaultSpec::None
        }
        .kind(),
        Some(Kind::MockControl)
    );
    for f in [
        Frame::Ping { seq: 1 },
        Frame::Pong { seq: 1 },
        Frame::Evt {
            evt: Event::Shuffle { on: true },
        },
        Frame::Res {
            id: 1,
            outcome: Outcome::Ok { data: json!(null) },
        },
    ] {
        assert_eq!(f.kind(), None);
    }
}

#[test]
fn set_queue_play_defaults_true() {
    let c: Command = serde_json::from_str(r#"{"type":"set_queue","ids":["a"],"start":0}"#).unwrap();
    assert_eq!(
        c,
        Command::SetQueue {
            ids: vec!["a".into()],
            start: 0,
            play: true
        }
    );
}
