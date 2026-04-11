mod args;
mod tui;
use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

use anyhow::{anyhow, bail, Result};
use args::Args;
use clap::Parser;
use engine::{profiler::Profiler, snapshot::Snapshot};

use crate::tui::run_tui;

fn main() -> Result<()> {
    let args = Args::parse();

    let pid = match (args.pid, args.binary, args.command) {
        (Some(pid), None, None) => check_pid(pid)?,
        (None, Some(bin), None) => launch_binary(bin)?,
        (None, None, Some(cmd)) => launch_command(cmd)?,
        _ => bail!("provide exactly one of: --pid, binary, or -c command"),
    };

    let (tx, rx) = mpsc::channel::<Snapshot>();
    let (startup_tx, startup_rx) = mpsc::channel::<Result<()>>();

    let rate = args.rate;
    let duration = args.duration;

    let stop = Arc::new(AtomicBool::new(false));
    let stop_profiler = stop.clone();
    let stop_tui = stop.clone();

    let profiler_thread = thread::spawn(move || -> Result<()> {
        let mut prof = match Profiler::new(pid, rate) {
            Ok(prof) => {
                let _ = startup_tx.send(Ok(()));
                prof
            }
            Err(e) => {
                stop_profiler.store(true, Ordering::SeqCst);
                let _ = startup_tx.send(Err(anyhow!(e.to_string())));
                return Ok(());
            }
        };

        prof.listen(duration, stop_profiler.clone(), move |snapshot| {
            let _ = tx.send(snapshot);
        })?;

        Ok(())
    });

    // Wait for profiler startup result BEFORE opening TUI
    match startup_rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            let _ = profiler_thread.join();
            return Err(e);
        }
        Err(_) => {
            let _ = profiler_thread.join();
            bail!("profiler thread exited before reporting startup status");
        }
    }

    let cmd = read_cmdline(pid);
    let tui_result = run_tui(pid, cmd, rx, stop_tui);

    stop.store(true, Ordering::SeqCst);

    let profiler_result = profiler_thread
        .join()
        .map_err(|_| anyhow!("profiler thread panicked"))?;

    tui_result?;
    profiler_result?;

    Ok(())
}

fn check_pid(pid: u32) -> Result<u32> {
    let proc_path = format!("/proc/{pid}");
    if !std::path::Path::new(&proc_path).exists() {
        bail!("Process with PID {} does not exist", pid);
    }

    let stat = fs::read_to_string(format!("{proc_path}/stat"))?;

    let status = stat.split_whitespace().nth(2).unwrap_or("?");

    if status == "Z" {
        bail!("Process {} is a zombie", pid);
    }

    Ok(pid)
}

fn read_cmdline(pid: u32) -> String {
    let path = format!("/proc/{pid}/cmdline");
    match fs::read(path) {
        Ok(bytes) => {
            let parts: Vec<String> = bytes
                .split(|b| *b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect();

            if parts.is_empty() {
                format!("pid {pid}")
            } else {
                parts.join(" ")
            }
        }
        Err(_) => format!("pid {pid}"),
    }
}

//todo
fn launch_binary(binary: String) -> Result<u32> {
    println!("launching {binary}");
    Ok(5)
}

// todo
fn launch_command(cmd: Vec<String>) -> Result<u32> {
    println!("launching {:?}", cmd);
    Ok(5)
}
