#![allow(dead_code)]
//! End-to-end CLI tests using the real binary.

use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn cli_no_instance_status_returns_1() {
    let tmp = TempDir::new().unwrap();
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_presto"))
        .arg("--demo")
        .arg("status")
        .env("XDG_RUNTIME_DIR", tmp.path())
        .output()
        .expect("Failed to run presto");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("presto is not running"), "stderr: {}", stderr);
}

#[test]
fn cli_no_instance_pause_returns_1() {
    let tmp = TempDir::new().unwrap();
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_presto"))
        .arg("--demo")
        .arg("pause")
        .env("XDG_RUNTIME_DIR", tmp.path())
        .output()
        .expect("Failed to run presto");

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn cli_bad_seek_returns_2() {
    let tmp = TempDir::new().unwrap();
    std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_presto"))
        .arg("--demo")
        .arg("seek")
        .arg("bogus")
        .env("XDG_RUNTIME_DIR", tmp.path())
        .output()
        .expect("Failed to run presto");

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn cli_xdg_unset_status_returns_2() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_presto"));
    cmd.arg("status")
        .env_remove("XDG_RUNTIME_DIR");

    let output = cmd.output().expect("Failed to run presto");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("XDG_RUNTIME_DIR"), "stderr: {}", stderr);
}
