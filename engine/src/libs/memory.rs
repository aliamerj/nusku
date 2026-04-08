use crate::snapshot::MemSnapshot;

/// Read memory usage from /proc/pid/status.
/// Returns zeroed snapshot if the process has exited or the file is unreadable.
pub fn read_mem(pid: u32) -> MemSnapshot {
    let path = format!("/proc/{}/status", pid);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return MemSnapshot::default();
    };

    let mut rss_kb = 0u64;
    let mut virt_kb = 0u64;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("VmRSS:") {
            rss_kb = parse_kb(val);
        } else if let Some(val) = line.strip_prefix("VmSize:") {
            virt_kb = parse_kb(val);
        }
    }

    MemSnapshot { rss_kb, virt_kb }
}

fn parse_kb(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
