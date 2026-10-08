//! Filesystem layout, private dirs, engine pidfile and logs.
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill, killpg};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct Paths {
    pub state: PathBuf,
    pub profile: PathBuf,
    pub logs: PathBuf,
    pub pidfile: PathBuf,
}

impl Paths {
    /// $XDG_STATE_HOME/presto, else $HOME/.local/state/presto
    pub fn from_env() -> Paths {
        let base = std::env::var_os("XDG_STATE_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
            });
        Paths::under(base.join("presto"))
    }

    pub fn under(state: PathBuf) -> Paths {
        Paths {
            profile: state.join("engine-profile"),
            logs: state.join("logs"),
            pidfile: state.join("engine.pid"),
            state,
        }
    }

    pub fn prepare(&self) -> io::Result<()> {
        for d in [&self.state, &self.profile, &self.logs] {
            ensure_private_dir(d)?;
        }
        Ok(())
    }
}

pub fn ensure_private_dir(p: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(p)?;
    let md = fs::symlink_metadata(p)?;
    if !md.is_dir() {
        return Err(io::Error::other(format!("{} is not a real directory", p.display())));
    }
    if md.uid() != nix::unistd::geteuid().as_raw() {
        return Err(io::Error::other(format!("{} is not owned by the current user", p.display())));
    }
    if md.mode() & 0o777 != 0o700 {
        fs::set_permissions(p, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Pidfile {
    pub pid: i32,
    pub start_ticks: u64,
    pub exe: PathBuf,
}

impl Pidfile {
    pub fn for_pid(pid: i32) -> io::Result<Pidfile> {
        Ok(Pidfile {
            pid,
            start_ticks: start_ticks(pid)?,
            exe: fs::read_link(format!("/proc/{pid}/exe"))?,
        })
    }

    pub fn write(&self, path: &Path) -> io::Result<()> {
        let mut f = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
        f.write_all(&serde_json::to_vec(self).map_err(io::Error::other)?)
    }

    pub fn read(path: &Path) -> io::Result<Option<Pidfile>> {
        match fs::read(path) {
            Ok(b) => Ok(Some(serde_json::from_slice(&b).map_err(io::Error::other)?)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Fields after the last ')' of /proc/<pid>/stat; index 0 is field 3 (state).
fn stat_rest(pid: i32) -> io::Result<Vec<String>> {
    let s = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let i = s.rfind(')').ok_or_else(|| io::Error::other("bad /proc stat"))?;
    Ok(s[i + 1..].split_whitespace().map(String::from).collect())
}

/// /proc/<pid>/stat field 22.
pub fn start_ticks(pid: i32) -> io::Result<u64> {
    stat_rest(pid)?
        .get(19)
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| io::Error::other("bad /proc stat"))
}

/// Process exists, is not a zombie, and has the given start time.
fn alive_with(pid: i32, ticks: u64) -> bool {
    match stat_rest(pid) {
        Ok(f) => f.first().map(String::as_str) != Some("Z") && f.get(19).and_then(|v| v.parse().ok()) == Some(ticks),
        Err(_) => false,
    }
}

fn signal(pid: i32, sig: Signal) {
    let p = Pid::from_raw(pid);
    if killpg(p, sig) == Err(Errno::ESRCH) {
        let _ = kill(p, sig);
    }
}

fn wait_dead(pid: i32, ticks: u64, limit: Duration) -> bool {
    let end = Instant::now() + limit;
    while alive_with(pid, ticks) {
        if Instant::now() >= end {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    true
}

// ponytail: no Chromium SingletonLock cleanup; Electron only takes it with requestSingleInstanceLock, which main.js never calls.
pub fn sweep_stale(pidfile: &Path, grace: Duration) -> io::Result<Option<i32>> {
    let Some(pf) = Pidfile::read(pidfile)? else { return Ok(None) };
    let ours = alive_with(pf.pid, pf.start_ticks)
        && fs::read_link(format!("/proc/{}/exe", pf.pid)).is_ok_and(|e| e == pf.exe);
    let mut killed = None;
    if ours {
        eprintln!("presto-core: killing stale engine pid {}", pf.pid);
        signal(pf.pid, Signal::SIGTERM);
        if !wait_dead(pf.pid, pf.start_ticks, grace) {
            signal(pf.pid, Signal::SIGKILL);
            wait_dead(pf.pid, pf.start_ticks, Duration::from_secs(1));
        }
        killed = Some(pf.pid);
    }
    match fs::remove_file(pidfile) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    Ok(killed)
}

const MAX_LOGS: usize = 10;

/// logs/engine-<unix_ts>-<attempt>.log, created 0600; keeps the newest 10.
pub fn new_log_file(logs: &Path, attempt: u32) -> io::Result<(PathBuf, File)> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let path = logs.join(format!("engine-{ts}-{attempt}.log"));
    let f = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&path)?;
    let mut old: Vec<_> = fs::read_dir(logs)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name();
            let n = n.to_string_lossy();
            n.starts_with("engine-") && n.ends_with(".log")
        })
        .map(|e| e.path())
        .filter(|p| *p != path)
        .filter_map(|p| Some((p.metadata().ok()?.modified().ok()?, p)))
        .collect();
    old.sort();
    let extra = old.len().saturating_sub(MAX_LOGS - 1);
    for (_, p) in old.into_iter().take(extra) {
        let _ = fs::remove_file(p);
    }
    Ok((path, f))
}

/// Last n lines of a file (lossy UTF-8); empty if unreadable.
pub fn tail(path: &Path, n: usize) -> Vec<String> {
    let Ok(b) = fs::read(path) else { return vec![] };
    let s = String::from_utf8_lossy(&b);
    let lines: Vec<_> = s.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::stat::{Mode, umask};

    fn mode(p: &Path) -> u32 {
        fs::metadata(p).unwrap().mode() & 0o777
    }

    #[test]
    fn paths_private_dir_new() {
        let t = tempfile::tempdir().unwrap();
        let old = umask(Mode::empty());
        let p = t.path().join("a/b");
        let r = ensure_private_dir(&p);
        umask(old);
        r.unwrap();
        assert_eq!(mode(&p), 0o700);
    }

    #[test]
    fn paths_private_dir_fixes_0755() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("d");
        DirBuilder::new().mode(0o755).create(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        ensure_private_dir(&p).unwrap();
        assert_eq!(mode(&p), 0o700);
    }

    #[test]
    fn paths_private_dir_rejects_symlink() {
        let t = tempfile::tempdir().unwrap();
        let real = t.path().join("real");
        fs::create_dir(&real).unwrap();
        let l = t.path().join("link");
        std::os::unix::fs::symlink(&real, &l).unwrap();
        assert!(ensure_private_dir(&l).is_err());
    }

    #[test]
    fn paths_pidfile_roundtrip() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("p");
        assert_eq!(Pidfile::read(&f).unwrap(), None);
        let pf = Pidfile::for_pid(std::process::id() as i32).unwrap();
        pf.write(&f).unwrap();
        assert_eq!(Pidfile::read(&f).unwrap(), Some(pf));
        assert_eq!(mode(&f), 0o600);
    }

    #[test]
    fn paths_start_ticks_self() {
        assert!(start_ticks(std::process::id() as i32).unwrap() > 0);
    }

    #[test]
    fn paths_log_prune() {
        let t = tempfile::tempdir().unwrap();
        for i in 0..12 {
            let (p, mut f) = new_log_file(t.path(), i).unwrap();
            writeln!(f, "l1\nl2\nl3").unwrap();
            if i == 11 {
                assert_eq!(tail(&p, 2), ["l2", "l3"]);
            }
        }
        assert_eq!(fs::read_dir(t.path()).unwrap().count(), 10);
    }
}
