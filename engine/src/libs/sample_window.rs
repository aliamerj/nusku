use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use profiler::cpu::CpuSample;

use crate::{
    libs::{memory::read_mem, symbols::Symbols},
    snapshot::{CpuSnapshot, HotFrame, Snapshot},
};

/// Counts how many times each function appears across all samples in a window.
#[derive(Debug, Clone)]
pub struct SampleWindow {
    pid: u32,
    counts: HashMap<u64, u64>, // top-of-stack address → sample count
    total: u64,
}

impl SampleWindow {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            counts: HashMap::new(),
            total: 0,
        }
    }

    pub fn add(&mut self, sample: &CpuSample) {
        self.total += 1;
        if let Some(&top) = sample.stack.first() {
            *self.counts.entry(top).or_insert(0) += 1;
        }
    }

    pub fn reset(&mut self) {
        self.counts.clear();
        self.total = 0;
    }

    /// Build a snapshot and reset the window.
    /// Call this once per second.
    pub fn snapshot(&mut self, symbols: &mut Symbols) -> Snapshot {
        let mem = read_mem(self.pid);
        let cpu = self.build_cpu_snapshot(symbols);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.reset();

        Snapshot {
            timestamp_ms,
            cpu,
            mem,
        }
    }

    fn build_cpu_snapshot(&self, symbols: &mut Symbols) -> CpuSnapshot {
        let mut grouped: HashMap<
            String,
            (
                u64,
                u64,
                String,
                Option<String>,
                Option<String>,
                Option<u32>,
            ),
        > = HashMap::new();

        for (&addr, &count) in &self.counts {
            let (display, name, file, file_full, line) = match symbols.resolve(addr) {
                Ok(sym) => (
                    sym.display(),
                    sym.name.clone(),
                    sym.file.clone(),
                    sym.file_full.clone(),
                    sym.line,
                ),
                Err(_) => (
                    format!("0x{addr:016x}"),
                    format!("0x{addr:016x}"),
                    None,
                    None,
                    None,
                ),
            };

            let entry = grouped
                .entry(display)
                .or_insert((0, addr, name, file, file_full, line));
            entry.0 += count;
        }

        let mut frames: Vec<HotFrame> = grouped
            .into_iter()
            .map(|(symbol, (count, addr, name, file, file_full, line))| {
                let percent = if self.total == 0 {
                    0.0
                } else {
                    (count as f64 / self.total as f64) * 100.0
                };
                HotFrame {
                    symbol,
                    name,
                    file,
                    file_full,
                    line,
                    addr,
                    count,
                    percent,
                }
            })
            .collect();

        frames.sort_by(|a, b| b.count.cmp(&a.count));

        CpuSnapshot {
            total_samples: self.total,
            frames,
        }
    }
}
