use std::fs::read_to_string;

use anyhow::{bail, Context, Result};

pub fn check_environment() -> Result<()> {
    check_kernel()?;
    check_perf_event_paranoid()?;
    check_privileges()?;
    Ok(())
}

fn check_kernel() -> Result<()> {
    let release =
        read_to_string("/proc/sys/kernel/osrelease").context("failed to read kernel version")?;

    let version = release.split('.').take(2).collect::<Vec<_>>();

    if version.len() >= 2 {
        let major: u32 = version[0].parse().unwrap_or(0);
        let minor: u32 = version[1].parse().unwrap_or(0);

        if major < 4 || (major == 4 && minor < 9) {
            bail!("Linux kernel ≥ 4.9 required for eBPF profiling");
        }
    }

    Ok(())
}

fn check_perf_event_paranoid() -> Result<()> {
    let val = read_to_string("/proc/sys/kernel/perf_event_paranoid")
        .context("failed to read perf_event_paranoid")?;

    let level: i32 = val.trim().parse().unwrap_or(3);

    if level > 2 {
        bail!(
            "perf_event_paranoid is too restrictive ({}).\n\
             Run:\n\
             sudo sysctl -w kernel.perf_event_paranoid=1",
            level
        );
    }

    Ok(())
}

fn check_privileges() -> Result<()> {
    let euid = unsafe { libc::geteuid() };

    if euid == 0 {
        return Ok(());
    }

    // Try reading capabilities from /proc/self/status
    let status = read_to_string("/proc/self/status").context("failed to read process status")?;

    if status.contains("CapEff:\t0000000000000000") {
        bail!(
            "nusku requires root or CAP_BPF + CAP_PERFMON capabilities.\n\
             Try running with sudo."
        );
    }

    Ok(())
}
