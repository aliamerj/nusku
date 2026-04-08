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
use engine::{profiler, snapshot::Snapshot};

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
        print_snapshot(pid, &snapshot);
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

fn format_kb(kb: u64) -> String {
    const MB: f64 = 1024.0;
    const GB: f64 = 1024.0 * 1024.0;

    if kb as f64 >= GB {
        format!("{:.2} GiB", kb as f64 / GB)
    } else {
        format!("{:.1} MiB", kb as f64 / MB)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out = s.chars().take(max.saturating_sub(1)).collect::<String>();
        out.push('…');
        out
    }
}

fn format_source(file: Option<&str>, line: Option<u32>) -> String {
    match (file, line) {
        (Some(file), Some(line)) => format!("{file}:{line}"),
        (Some(file), None) => file.to_string(),
        _ => "-".to_string(),
    }
}

pub fn print_snapshot(pid: u32, snapshot: &Snapshot) {
    let cpu = &snapshot.cpu;

    println!(
        "\n── PID {} ── {} samples ── CPU {:>5.1}% ── RSS {} ── VIRT {} ──",
        pid,
        cpu.total_samples,
        cpu.cpu_percent,
        format_kb(snapshot.mem.rss_kb),
        format_kb(snapshot.mem.virt_kb),
    );

    if cpu.frames.is_empty() {
        println!("No samples collected yet.");
        return;
    }

    println!(
        "{:>6}  {:>8}  {:<32}  {:<20}  {:>18}",
        "%", "COUNT", "FUNCTION", "SOURCE", "ADDRESS"
    );
    println!("{}", "─".repeat(96));

    for frame in cpu.frames.iter().take(15) {
        let function = truncate(&frame.name, 32);
        let source = truncate(&format_source(frame.file.as_deref(), frame.line), 20);

        println!(
            "{:>5.1}%  {:>8}  {:<32}  {:<20}  0x{:016x}",
            frame.percent, frame.count, function, source, frame.addr
        );
    }

    // Optional details block for the hottest frame, so file_full is actually used.
    if let Some(top) = cpu.frames.first() {
        if let Some(full) = &top.file_full {
            println!("\nTop frame:");
            println!("  {}", top.name);
            println!("  {}", full);
            if let Some(line) = top.line {
                println!("  line {}", line);
            }
        }
    }
}
