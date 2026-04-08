mod args;

use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{bail, Context, Result};
use args::Args;
use clap::Parser;
use engine::{profiler, snapshot::CpuSnapshot};

fn main() -> Result<()> {
    let args = Args::parse();

    let pid = match (args.pid, args.binary, args.command) {
        (Some(pid), None, None) => check_pid(pid)?,
        (None, Some(bin), None) => launch_binary(bin)?,
        (None, None, Some(cmd)) => launch_command(cmd)?,
        _ => {
            bail!("provide exactly one of: --pid, binary, or -c command");
        }
    };

    println!("nusku: targeting PID {pid}");

    // Ctrl+C handler — set a flag so we exit cleanly
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })
        .context("failed to set Ctrl+C handler")?;
    }

    let mut prof = profiler::Profiler::new(pid, args.rate)?;
    prof.listen(args.duration, running, |snapshot| {
        print_snapshot(pid, &snapshot.cpu);
    })?;

    println!("\nnusku: stopped.");
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

fn print_snapshot(pid: u32, snapshot: &CpuSnapshot) {
    println!("\n── PID {} ── {} samples ──", pid, snapshot.total_samples);

    if snapshot.frames.is_empty() {
        println!("No samples collected yet.");
        return;
    }

    println!(
        "{:>6}  {:>8}  {:<40}  {:>18}",
        "%", "COUNT", "SYMBOL", "ADDRESS"
    );
    println!("{}", "─".repeat(84));

    for frame in &snapshot.frames {
        println!(
            "{:>5.1}%  {:>8}  {:<40}  0x{:016x}",
            frame.percent, frame.count, frame.symbol, frame.addr
        );
    }
}
