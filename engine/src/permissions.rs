use anyhow::{bail, Result};

pub fn check() -> Result<()> {
    // Simplest check: try reading /proc/kallsyms which requires root
    // A more correct check would inspect capabilities, but this is good enough
    let euid = unsafe { libc::geteuid() };
    if euid != 0 {
        bail!(
            "nusku requires root privileges to attach eBPF probes.\n\
             Run with: sudo nusku ..."
        );
    }
    Ok(())
}
