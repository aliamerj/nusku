//! Per-thread and per-core CPU usage for a single process.
//!
//! Strategy:
//! - Read /proc/pid/task/ to enumerate all threads
//! - Read /proc/pid/task/tid/stat for each thread: utime+stime and last_cpu
//! - Diff against previous reading to get delta ticks
//! - Map each thread to its last-run CPU core
//! - Aggregate per-core usage as % of one core

use std::collections::HashMap;
use std::fs;
use std::time::Instant;

/// One CPU core's usage attributed to this process
#[derive(Debug, Clone, Default)]
pub struct CoreUsage {
    pub core_id: usize,
    pub percent: f64, // 0.0–100.0, fraction of one core
}

/// Per-thread info
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    pub tid: u32,
    pub name: String,
    pub cpu_id: usize, // last CPU core this thread ran on
    pub percent: f64,  // CPU% for this thread
}

/// Full per-process CPU breakdown
#[derive(Debug, Clone, Default)]
pub struct ProcessCpuInfo {
    pub total_percent: f64,           // sum across all cores
    pub core_count: usize,            // total logical cores on machine
    pub active_cores: Vec<CoreUsage>, // only cores with >0 usage, sorted
    pub threads: Vec<ThreadInfo>,     // sorted by percent desc
}

/// Stateful reader — call `read()` repeatedly, diffs ticks each call
pub struct ProcessCpuReader {
    pid: u32,
    prev_ticks: HashMap<u32, u64>, // tid → last (utime + stime)
    prev_time: Instant,
    core_count: usize,
}

impl ProcessCpuReader {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            prev_ticks: HashMap::new(),
            prev_time: Instant::now(),
            core_count: read_core_count(),
        }
    }

    /// Call once per second. Returns None if process has exited.
    pub fn read(&mut self) -> Option<ProcessCpuInfo> {
        let elapsed = self.prev_time.elapsed().as_secs_f64();
        self.prev_time = Instant::now();

        if elapsed < 0.01 {
            return None;
        }

        // CLK_TCK is almost always 100 on Linux
        let ticks_per_sec = 100.0f64;
        let ticks_per_interval = ticks_per_sec * elapsed;

        let tasks_dir = format!("/proc/{}/task", self.pid);
        let entries = fs::read_dir(&tasks_dir).ok()?;

        let mut core_ticks: HashMap<usize, f64> = HashMap::new();
        let mut threads = Vec::new();
        let mut current_ticks: HashMap<u32, u64> = HashMap::new();

        for entry in entries.flatten() {
            let tid: u32 = entry.file_name().to_string_lossy().parse().ok()?;
            let stat_path = format!("/proc/{}/task/{}/stat", self.pid, tid);
            let stat = fs::read_to_string(&stat_path).ok()?;

            let (utime, stime, cpu_id, name) = parse_thread_stat(&stat)?;
            let total = utime + stime;
            current_ticks.insert(tid, total);

            let delta = if let Some(&prev) = self.prev_ticks.get(&tid) {
                total.saturating_sub(prev) as f64
            } else {
                0.0
            };

            let percent = (delta / ticks_per_interval * 100.0).min(100.0);

            *core_ticks.entry(cpu_id).or_insert(0.0) += delta;

            threads.push(ThreadInfo {
                tid,
                name,
                cpu_id,
                percent,
            });
        }

        self.prev_ticks = current_ticks;

        // Build per-core usage
        let mut active_cores: Vec<CoreUsage> = core_ticks
            .into_iter()
            .map(|(core_id, ticks)| {
                let percent = (ticks / ticks_per_interval * 100.0).min(100.0);
                CoreUsage { core_id, percent }
            })
            .filter(|c| c.percent > 0.1)
            .collect();

        active_cores.sort_by(|a, b| a.core_id.cmp(&b.core_id));
        threads.sort_by(|a, b| b.percent.partial_cmp(&a.percent).unwrap());

        let total_percent: f64 = threads
            .iter()
            .map(|t| t.percent)
            .sum::<f64>()
            .min(self.core_count as f64 * 100.0);

        Some(ProcessCpuInfo {
            total_percent,
            core_count: self.core_count,
            active_cores,
            threads,
        })
    }
}

/// Parse /proc/pid/task/tid/stat
/// Returns (utime, stime, last_cpu, thread_name)
fn parse_thread_stat(stat: &str) -> Option<(u64, u64, usize, String)> {
    // Format: pid (name) state ppid ... utime(14) stime(15) ... processor(39)
    // The name can contain spaces and parens — find the last ')' to split safely
    let name_start = stat.find('(')?;
    let name_end = stat.rfind(')')?;
    let name = stat[name_start + 1..name_end].to_string();
    let rest: Vec<&str> = stat[name_end + 2..].split_whitespace().collect();

    // Fields after ')': state(0) ppid(1) pgrp(2) session(3) tty(4) tpgid(5)
    // flags(6) minflt(7) cminflt(8) majflt(9) cmajflt(10) utime(11) stime(12)
    // ...processor is field 36 (0-indexed from state)
    let utime: u64 = rest.get(11)?.parse().ok()?;
    let stime: u64 = rest.get(12)?.parse().ok()?;
    let cpu_id: usize = rest.get(36)?.parse().ok()?;

    Some((utime, stime, cpu_id, name))
}

fn read_core_count() -> usize {
    fs::read_to_string("/proc/cpuinfo")
        .map(|s| s.lines().filter(|l| l.starts_with("processor")).count())
        .unwrap_or(1)
        .max(1)
}
