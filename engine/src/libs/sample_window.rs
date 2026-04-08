use std::collections::HashMap;

use profiler::cpu::CpuSample;

use crate::{
    libs::symbols::Symbols,
    snapshot::{CpuSnapshot, HotFrame, Snapshot},
};

/// Counts how many times each function appears across all samples in a window.
#[derive(Debug, Clone)]
pub struct SampleWindow {
    pub counts: HashMap<u64, u64>,
    pub total: u64,
}

impl SampleWindow {
    pub fn new() -> Self {
        Self {
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

    pub fn snapshot(&self, symbols: &mut Symbols) -> Snapshot {
        let mut grouped: HashMap<String, (u64, u64)> = HashMap::new();

        for (&addr, &count) in &self.counts {
            let symbol = symbols
                .resolve(addr)
                .map(|sym| match &sym.module {
                    Some(module) => format!("{} ({})", sym.name, module),
                    None => sym.name.clone(),
                })
                .unwrap_or_else(|_| format!("0x{addr:016x}"));

            let entry = grouped.entry(symbol).or_insert((0, addr));
            entry.0 += count;
        }

        let mut frames: Vec<HotFrame> = grouped
            .into_iter()
            .map(|(symbol, (count, addr))| {
                let percent = if self.total == 0 {
                    0.0
                } else {
                    (count as f64 / self.total as f64) * 100.0
                };

                HotFrame {
                    count,
                    percent,
                    addr,
                    symbol,
                }
            })
            .collect();

        frames.sort_by(|a, b| b.count.cmp(&a.count));

        Snapshot {
            cpu: CpuSnapshot {
                total_samples: self.total,
                frames,
            },
        }
    }
}
