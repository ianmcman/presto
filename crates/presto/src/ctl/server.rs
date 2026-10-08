//! Control socket server: accepts requests, manages subscriptions, coordinates with control operations.

use crate::control::{self, Control, Position};
use crate::status::{self, status_json};
use presto_core::state::CoreState;
use presto_ipc::ctl::{CtlRequest, CtlOp, CtlReply, CTL_PROTO, MAX_LINE};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;

pub async fn serve(
    listener: UnixListener,
    state: watch::Receiver<CoreState>,
    ctl: Arc<dyn Control>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = state.clone();
                let ctl = ctl.clone();
                tokio::spawn(async move {
                    handle_connection(stream, state, ctl).await;
                });
            }
            Err(e) => {
                eprintln!("ctl: accept error: {}", e);
                break;
            }
        }
    }
}

async fn handle_connection(
    stream: UnixStream,
    mut state: watch::Receiver<CoreState>,
    ctl: Arc<dyn Control>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut buf = String::new();
    let timeout = Duration::from_secs(10);

    loop {
        buf.clear();

        let n = match tokio::time::timeout(
            timeout,
            reader.read_line(&mut buf),
        ).await {
            Ok(Ok(n)) => n,
            Ok(Err(_)) | Err(_) => break,
        };

        if n == 0 {
            break;
        }

        // Check line length
        if buf.len() > MAX_LINE + 1 {
            let _ = writer.write_all(
                serde_json::to_string(&CtlReply {
                    proto: CTL_PROTO,
                    id: 0,
                    ok: false,
                    error: Some("request too large".to_string()),
                    status: None,
                }).unwrap().as_bytes()
            ).await;
            let _ = writer.write_all(b"\n").await;
            break;
        }

        let line = buf.trim_end();

        // Try to parse as JSON
        let req: CtlRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(_) => {
                let _ = writer.write_all(
                    serde_json::to_string(&CtlReply {
                        proto: CTL_PROTO,
                        id: 0,
                        ok: false,
                        error: Some("bad request".to_string()),
                        status: None,
                    }).unwrap().as_bytes()
                ).await;
                let _ = writer.write_all(b"\n").await;
                break;
            }
        };

        // Check proto
        if req.proto != CTL_PROTO {
            let _ = writer.write_all(
                serde_json::to_string(&CtlReply {
                    proto: CTL_PROTO,
                    id: req.id,
                    ok: false,
                    error: Some("unsupported proto".to_string()),
                    status: None,
                }).unwrap().as_bytes()
            ).await;
            let _ = writer.write_all(b"\n").await;
            break;
        }

        // Handle Subscribe separately
        if matches!(req.op, CtlOp::Subscribe) {
            handle_subscribe(&mut writer, &mut state, &ctl, req.id).await;
            break;
        }

        // For other ops, plan and apply
        let current_state = state.borrow().clone();
        let pos_ms = Position::new().now(&current_state);

        let reply = match control::plan(&req.op, &current_state, pos_ms) {
            Ok(acts) => {
                control::apply(&*ctl, acts);
                CtlReply::ok(req.id)
            }
            Err(reject) => {
                CtlReply {
                    proto: CTL_PROTO,
                    id: req.id,
                    ok: false,
                    error: Some(reject.message().to_string()),
                    status: None,
                }
            }
        };

        // Handle Status op: include status in reply
        let reply = if matches!(req.op, CtlOp::Status) {
            let art_path = art_for(&current_state, &*ctl);
            let status = status_json(&current_state, pos_ms, art_path.as_deref());
            CtlReply {
                status: Some(status),
                ..reply
            }
        } else {
            reply
        };

        if let Ok(json) = serde_json::to_string(&reply) {
            let _ = writer.write_all(json.as_bytes()).await;
            let _ = writer.write_all(b"\n").await;
        }
    }
}

async fn handle_subscribe(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    state: &mut watch::Receiver<CoreState>,
    ctl: &Arc<dyn Control>,
    req_id: u64,
) {
    let mut last_status = None;
    let mut last_position_emit = std::time::Instant::now();
    let position_rate_limit = Duration::from_secs(1);

    loop {
        // Emit current status on first iteration
        let current_state = state.borrow().clone();
        let pos_ms = Position::new().now(&current_state);
        let art_path = art_for(&current_state, &**ctl);
        let current_status = status_json(&current_state, pos_ms, art_path.as_deref());

        let should_emit = if let Some(ref last) = last_status {
            !status::same_but_position(&last, &current_status) ||
            last_position_emit.elapsed() >= position_rate_limit
        } else {
            true
        };

        if should_emit {
            let reply = CtlReply {
                proto: CTL_PROTO,
                id: req_id,
                ok: true,
                error: None,
                status: Some(current_status.clone()),
            };

            if let Ok(json) = serde_json::to_string(&reply) {
                if writer.write_all(json.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
            }

            last_status = Some(current_status);
            last_position_emit = std::time::Instant::now();
        }

        // Wait for state change
        if state.changed().await.is_err() {
            break;
        }
    }
}

fn art_for(s: &CoreState, ctl: &dyn Control) -> Option<std::path::PathBuf> {
    s.player.track.as_ref()
        .and_then(|t| t.artwork_url.as_ref())
        .and_then(|u| ctl.art(u))
}
