use presto_ipc::ctl::CtlOp;

/// Parse a time specification: "72" (seconds), "1:12" (m:s), "1:12:00" (h:m:s),
/// or relative "+10" / "-10". Returns Seek or SeekBy.
pub fn parse_seek(s: &str) -> Result<CtlOp, String> {
    if s.is_empty() {
        return Err("invalid time \"\"".to_string());
    }

    if let Some(rest) = s.strip_prefix('+') {
        // Relative, positive
        parse_time_relative(rest).map(|ms| CtlOp::SeekBy { ms: ms as i64 })
    } else if let Some(rest) = s.strip_prefix('-') {
        // Relative, negative
        parse_time_relative(rest).map(|ms| CtlOp::SeekBy { ms: -(ms as i64) })
    } else {
        // Absolute
        parse_time(s).map(|ms| CtlOp::Seek { ms })
    }
}

fn parse_time(s: &str) -> Result<u64, String> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        1 => {
            // Just seconds
            parts[0]
                .parse::<u64>()
                .map(|s| s * 1000)
                .map_err(|_| format!("invalid time \"{s}\""))
        }
        2 => {
            // m:s
            let m = parts[0]
                .parse::<u64>()
                .map_err(|_| format!("invalid time \"{s}\""))?;
            let s_val = parts[1]
                .parse::<u64>()
                .map_err(|_| format!("invalid time \"{s}\""))?;
            if s_val >= 60 {
                return Err(format!("invalid time \"{s}\""));
            }
            Ok((m * 60 + s_val) * 1000)
        }
        3 => {
            // h:m:s
            let h = parts[0]
                .parse::<u64>()
                .map_err(|_| format!("invalid time \"{s}\""))?;
            let m = parts[1]
                .parse::<u64>()
                .map_err(|_| format!("invalid time \"{s}\""))?;
            let s_val = parts[2]
                .parse::<u64>()
                .map_err(|_| format!("invalid time \"{s}\""))?;
            if m >= 60 || s_val >= 60 {
                return Err(format!("invalid time \"{s}\""));
            }
            Ok((h * 3600 + m * 60 + s_val) * 1000)
        }
        _ => Err(format!("invalid time \"{s}\"")),
    }
}

fn parse_time_relative(s: &str) -> Result<u64, String> {
    parse_time(s)
}

/// Parse a volume specification: "0"-"100" (absolute), or "+5" / "-5" (relative).
pub fn parse_volume(s: &str) -> Result<CtlOp, String> {
    if s.is_empty() {
        return Err("volume must be 0-100".to_string());
    }

    if let Some(rest) = s.strip_prefix('+') {
        // Relative, positive
        rest.parse::<i32>()
            .map(|pct| CtlOp::VolumeBy { pct })
            .map_err(|_| "volume must be 0-100".to_string())
    } else if let Some(rest) = s.strip_prefix('-') {
        // Relative, negative
        rest.parse::<i32>()
            .map(|pct| CtlOp::VolumeBy { pct: -pct })
            .map_err(|_| "volume must be 0-100".to_string())
    } else {
        // Absolute
        match s.parse::<u8>() {
            Ok(v) if v <= 100 => Ok(CtlOp::Volume { pct: v }),
            _ => Err("volume must be 0-100".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seek_absolute_seconds() {
        assert_eq!(parse_seek("72").unwrap(), CtlOp::Seek { ms: 72000 });
    }

    #[test]
    fn parse_seek_absolute_m_s() {
        assert_eq!(parse_seek("1:12").unwrap(), CtlOp::Seek { ms: 72000 });
    }

    #[test]
    fn parse_seek_absolute_h_m_s() {
        assert_eq!(parse_seek("1:12:00").unwrap(), CtlOp::Seek { ms: 4320000 });
    }

    #[test]
    fn parse_seek_relative_positive() {
        assert_eq!(parse_seek("+10").unwrap(), CtlOp::SeekBy { ms: 10000 });
    }

    #[test]
    fn parse_seek_relative_negative() {
        assert_eq!(parse_seek("-10").unwrap(), CtlOp::SeekBy { ms: -10000 });
    }

    #[test]
    fn parse_seek_relative_time_positive() {
        assert_eq!(parse_seek("+1:00").unwrap(), CtlOp::SeekBy { ms: 60000 });
    }

    #[test]
    fn parse_seek_errors() {
        assert!(parse_seek("").is_err());
        assert!(parse_seek("abc").is_err());
        assert!(parse_seek("1:75").is_err()); // seconds >= 60
        assert!(parse_seek("1:2:3:4").is_err()); // too many parts
        assert!(parse_seek("+").is_err());
    }

    #[test]
    fn parse_volume_absolute() {
        assert_eq!(parse_volume("0").unwrap(), CtlOp::Volume { pct: 0 });
        assert_eq!(parse_volume("50").unwrap(), CtlOp::Volume { pct: 50 });
        assert_eq!(parse_volume("100").unwrap(), CtlOp::Volume { pct: 100 });
    }

    #[test]
    fn parse_volume_relative() {
        assert_eq!(parse_volume("+5").unwrap(), CtlOp::VolumeBy { pct: 5 });
        assert_eq!(parse_volume("-5").unwrap(), CtlOp::VolumeBy { pct: -5 });
    }

    #[test]
    fn parse_volume_errors() {
        assert!(parse_volume("").is_err());
        assert!(parse_volume("101").is_err());
        assert!(parse_volume("x").is_err());
        assert!(parse_volume("+").is_err());
    }
}
