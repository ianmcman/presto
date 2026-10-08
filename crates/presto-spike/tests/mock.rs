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

fn find(dir: &std::path::Path, prefix: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir.join("logs"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with(prefix))
        .collect()
}

#[test]
fn check_all_against_mock() {
    let t = tmp();
    let o = run(
        t.path(),
        &[
            "--label",
            "run",
            "--allow-mock",
            "check",
            "all",
            "--song",
            "s3",
            "--min-play-secs",
            "2",
            "--seek-secs",
            "30",
            "--auth-wait-secs",
            "5",
        ],
    );
    let s = out(&o);
    assert!(o.status.success(), "{s}");
    for n in ["hello", "musickit", "session", "api", "playback", "events"] {
        assert!(s.contains(&format!("PASS  {n}")), "{n}: {s}");
    }
    assert!(!s.contains("FAIL"), "{s}");
    let md = find(t.path(), "checklist-run-");
    assert_eq!(md.len(), 1);
    assert!(
        std::fs::read_to_string(&md[0])
            .unwrap()
            .contains("| check | result | detail |")
    );
    let ev = find(t.path(), "events-run-");
    assert_eq!(ev.len(), 1);
    assert!(
        std::fs::read_to_string(&ev[0])
            .unwrap()
            .contains("\"progress\"")
    );
}

#[test]
fn playback_via_search() {
    let t = tmp();
    let o = run(
        t.path(),
        &[
            "--allow-mock",
            "check",
            "playback",
            "--search-term",
            "cache",
            "--min-play-secs",
            "2",
            "--seek-secs",
            "30",
        ],
    );
    assert!(o.status.success(), "{}", out(&o));
}

#[test]
fn measure_writes_csv() {
    let t = tmp();
    let o = run(
        t.path(),
        &[
            "--allow-mock",
            "measure",
            "--song",
            "s3",
            "--play-secs",
            "3",
            "--interval-secs",
            "1",
            "--settle-secs",
            "1",
        ],
    );
    assert!(o.status.success(), "{}", out(&o));
    let f = find(t.path(), "rss-run-");
    assert_eq!(f.len(), 1);
    let csv = std::fs::read_to_string(&f[0]).unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next(), Some("date,phase,rss_mib,pss_mib"));
    let rest: Vec<_> = lines.collect();
    assert!(rest.len() >= 4);
    for p in ["idle_after_hello", "signed_in_idle", "playing"] {
        assert!(rest.iter().any(|l| l.contains(p)), "{p}");
    }
}
