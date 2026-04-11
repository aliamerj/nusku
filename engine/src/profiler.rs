use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Result;
use profiler::cpu::CPU;

use crate::{
    libs::{permissions::check_environment, sample_window::SampleWindow, symbols::Symbols},
    snapshot::Snapshot,
};

pub struct Profiler {
    cpu: CPU,
    wind: SampleWindow,
    symbols: Symbols,
    last_tick: Instant,
}

impl Profiler {
    pub fn new(pid: u32, rate_hz: u64) -> Result<Self> {
        check_environment()?;

        Ok(Self {
            cpu: CPU::new(pid, rate_hz),
            wind: SampleWindow::new(pid),
            symbols: Symbols::new(pid),
            last_tick: Instant::now(),
        })
    }

    pub fn listen<F>(
        &mut self,
        duration_sec: Option<u64>,
        stop: Arc<AtomicBool>,
        mut on_profile: F,
    ) -> Result<()>
    where
        F: FnMut(Snapshot),
    {
        let start = Instant::now();

        let agg = &mut self.wind;
        let symbols = &mut self.symbols;
        let last_tick = &mut self.last_tick;

        self.cpu.run(stop.clone(), |event| {
            if let Some(secs) = duration_sec {
                if start.elapsed() > Duration::from_secs(secs) {
                    stop.store(true, Ordering::SeqCst);
                    return;
                }
            }

            agg.add(&event);

            if last_tick.elapsed() >= Duration::from_secs(1) {
                let snapshot = agg.snapshot(symbols);
                on_profile(snapshot);
                *last_tick = Instant::now();
            }
        })
    }
}
