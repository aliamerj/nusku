/// A single hot function frame in a snapshot
#[derive(Debug, Clone)]
pub struct HotFrame {
    pub symbol: String,       // display string from ResolvedSymbol::display()
    pub name: String,         // raw function name only, for filtering
    pub file: Option<String>, // short filename
    pub file_full: Option<String>,
    pub line: Option<u32>,
    pub addr: u64,
    pub count: u64,
    pub percent: f64,
}

/// CPU data for one snapshot window
#[derive(Debug, Clone)]
pub struct CpuSnapshot {
    pub total_samples: u64,
    pub frames: Vec<HotFrame>, // sorted by count descending
    pub cpu_percent: f64,      // 0.0–100.0, estimated from sample rate
}

/// Memory data read from /proc/pid/status
#[derive(Debug, Clone, Default)]
pub struct MemSnapshot {
    pub rss_kb: u64,  // resident set size
    pub virt_kb: u64, // virtual memory
}

/// Everything the TUI needs for one update tick
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub timestamp_ms: u64, // unix ms, for the sparkline x-axis
    pub cpu: CpuSnapshot,
    pub mem: MemSnapshot,
}
