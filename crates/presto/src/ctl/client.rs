//! Control socket client: connects to the running app and sends commands.

use crate::ctl::CtlPaths;
use crate::cli::Sub;
use crate::status::one_line;
use presto_ipc::ctl::{CtlRequest, CtlReply, CTL_PROTO};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

#[derive(Debug)]
pub enum ClientError {
    NotRunning,
    Io(io::Error),
    Protocol(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::NotRunning => write!(f, "presto is not running"),
            ClientError::Io(e) => write!(f, "{}", e),
            ClientError::Protocol(s) => write!(f, "{}", s),
        }
    }
}

pub fn request(p: &CtlPaths, op: presto_ipc::ctl::CtlOp) -> Result<CtlReply, ClientError> {
    let mut stream = UnixStream::connect(&p.sock)
        .map_err(|e| match e.kind() {
            io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound => ClientError::NotRunning,
            _ => ClientError::Io(e),
        })?;

    stream.set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(ClientError::Io)?;

    let req = CtlRequest {
        proto: CTL_PROTO,
        id: 1,
        op,
    };

    let line = serde_json::to_string(&req).map_err(|e| ClientError::Protocol(e.to_string()))?;
    stream.write_all(line.as_bytes()).map_err(ClientError::Io)?;
    stream.write_all(b"\n").map_err(ClientError::Io)?;

    let mut reader = BufReader::new(&stream);
    let mut response = String::new();
    reader.read_line(&mut response).map_err(ClientError::Io)?;

    serde_json::from_str(&response)
        .map_err(|e| ClientError::Protocol(e.to_string()))
}

pub fn run(p: &CtlPaths, sub: &Sub) -> i32 {
    match sub.to_op() {
        Err(msg) => {
            eprintln!("{}", msg);
            2
        }
        Ok(op) => {
            match &op {
                presto_ipc::ctl::CtlOp::Subscribe => {
                    run_subscribe(p, matches!(sub, Sub::Status { json: true, .. }))
                }
                _ => {
                    match request(p, op.clone()) {
                        Err(e) => {
                            eprintln!("{}", e);
                            match e {
                                ClientError::NotRunning => 1,
                                _ => 3,
                            }
                        }
                        Ok(reply) => {
                            if !reply.ok {
                                if let Some(err) = reply.error {
                                    eprintln!("{}", err);
                                }
                                3
                            } else {
                                // Print status if requested
                                if let presto_ipc::ctl::CtlOp::Status = op {
                                    if let Some(status) = reply.status {
                                        if matches!(sub, Sub::Status { json: true, .. }) {
                                            if let Ok(json) = serde_json::to_string(&status) {
                                                println!("{}", json);
                                            }
                                        } else {
                                            println!("{}", one_line(&status));
                                        }
                                    }
                                }
                                0
                            }
                        }
                    }
                }
            }
        }
    }
}

fn run_subscribe(p: &CtlPaths, json: bool) -> i32 {
    match UnixStream::connect(&p.sock) {
        Err(_) => {
            eprintln!("presto is not running");
            1
        }
        Ok(mut stream) => {
            let req = CtlRequest {
                proto: CTL_PROTO,
                id: 1,
                op: presto_ipc::ctl::CtlOp::Subscribe,
            };

            let line = serde_json::to_string(&req).unwrap();
            if stream.write_all(line.as_bytes()).is_err() {
                return 1;
            }
            if stream.write_all(b"\n").is_err() {
                return 1;
            }

            let mut reader = BufReader::new(&stream);
            let mut response = String::new();

            loop {
                response.clear();
                if reader.read_line(&mut response).is_err() {
                    break;
                }
                if response.is_empty() {
                    break;
                }

                if let Ok(reply) = serde_json::from_str::<CtlReply>(&response) {
                    if let Some(status) = reply.status {
                        match (json, serde_json::to_string(&status)) {
                            (true, Ok(line)) => println!("{}", line),
                            _ => println!("{}", one_line(&status)),
                        }
                        let _ = std::io::stdout().flush();
                    }
                }
            }

            0
        }
    }
}
