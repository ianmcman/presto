use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

fn mock_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("PRESTO_MOCK_BIN") {
        return p.into();
    }
    let st = Command::new(env!("CARGO"))
        .args(["build", "-q", "-p", "presto-engine-mock"])
        .status()
        .unwrap();
    assert!(st.success());
    let exe = std::env::current_exe().unwrap();
    exe.parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("presto-engine-mock")
}

fn run(tmp: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_presto-spike"))
        .arg("--engine-bin")
        .arg(mock_bin())
        .arg("--socket")
        .arg(tmp.join("e.sock"))
        .arg("--profile")
        .arg(tmp.join("prof"))
        .arg("--log-dir")
        .arg(tmp.join("logs"))
        .args(args)
        .output()
        .unwrap()
}

fn tmp() -> tempfile::TempDir {
    // short path: sun_path is ~108 bytes
    tempfile::Builder::new()
        .prefix("p")
        .tempdir_in("/tmp")
        .unwrap()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn hello_rejects_mock_cap() {
    let t = tmp();
    let o = run(t.path(), &["check", "hello"]);
    let s = out(&o);
    assert!(!o.status.success());
    assert!(s.contains("FAIL") && s.contains("hello"), "{s}");
}

#[test]
fn hello_passes_with_allow_mock() {
    let t = tmp();
    let o = run(t.path(), &["--allow-mock", "check", "hello"]);
    let s = out(&o);
    assert!(o.status.success(), "{s}");
    assert!(s.contains("PASS") && s.contains("hello"), "{s}");
}

#[test]
fn profile_dir_is_0700() {
    let t = tmp();
    run(t.path(), &["--allow-mock", "check", "hello"]);
    let m = std::fs::metadata(t.path().join("prof")).unwrap();
    assert_eq!(m.permissions().mode() & 0o777, 0o700);
}
