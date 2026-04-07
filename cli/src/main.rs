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

    //todo :
    // run profiling..

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
