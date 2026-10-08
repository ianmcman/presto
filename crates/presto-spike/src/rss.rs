//! Process-tree memory from /proc (D-14: measurement only, no threshold).

/// (rss_kb, pss_kb) from `Rss:` and `Pss:` lines of smaps_rollup.
pub fn parse_rollup(s: &str) -> (u64, u64) {
    let field = |key: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    (field("Rss:"), field("Pss:"))
}

/// `root` plus all descendants. ppid is field 4 of /proc/<pid>/stat, parsed
/// after the last ')' because comm may contain spaces.
pub fn tree(root: u32) -> Vec<u32> {
    let mut parent_of = Vec::new();
    if let Ok(dir) = std::fs::read_dir("/proc") {
        for e in dir.flatten() {
            let Some(pid) = e.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
                continue;
            };
            let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                continue;
            };
            let ppid = stat
                .rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u32>().ok());
            if let Some(pp) = ppid {
                parent_of.push((pid, pp));
            }
        }
    }
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let cur = out[i];
        for &(pid, pp) in &parent_of {
            if pp == cur && !out.contains(&pid) {
                out.push(pid);
            }
        }
        i += 1;
    }
    out
}

/// Summed (rss, pss) in MiB over the tree; vanished pids are skipped.
pub fn sample(root: u32) -> (f64, f64) {
    let (mut rss, mut pss) = (0u64, 0u64);
    for pid in tree(root) {
        if let Ok(s) = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")) {
            let (r, p) = parse_rollup(&s);
            rss += r;
            pss += p;
        }
    }
    (rss as f64 / 1024.0, pss as f64 / 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rollup() {
        assert_eq!(parse_rollup("Rss:  2048 kB\nPss:  1024 kB\n"), (2048, 1024));
    }

    #[test]
    fn tree_has_self() {
        assert!(tree(std::process::id()).contains(&std::process::id()));
    }
}
