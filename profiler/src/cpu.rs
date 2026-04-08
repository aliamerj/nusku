use crate::bpf::cpu_sampler::CpuSamplerSkelBuilder;
use anyhow::{bail, Context, Result};
use libbpf_rs::{
    skel::{OpenSkel, SkelBuilder},
    ErrorKind, MapCore, MapFlags, RingBufferBuilder,
};
use perf_event_open_sys::bindings::{
    perf_event_attr, PERF_COUNT_SW_CPU_CLOCK, PERF_FLAG_FD_CLOEXEC, PERF_TYPE_SOFTWARE,
};
use plain::Plain;
use std::{io::Error, mem::MaybeUninit, os::fd::AsFd, time::Duration};

/// Mirror of `cpu_event` in cpu_sampler.bpf.c
/// Layout must match exactly — same field order, same sizes.
#[repr(C)]
#[derive(Debug, Clone, Default)]
struct CpuEvent {
    pid: u32,
    tid: u32,
    timestamp_ns: u64,
    user_stack_id: i32,
}
unsafe impl Plain for CpuEvent {}

/// Processed sample with resolved stack addresses.
/// This is what the rest of engine works with.
#[derive(Debug, Clone)]
pub struct CpuSample {
    pub pid: u32,
    pub tid: u32,
    pub timestamp_ns: u64,
    pub stack: Vec<u64>,
}

pub struct CPU {
    pid: u32,
    rate_hz: u64,
}

impl CPU {
    pub fn new(pid: u32, rate_hz: u64) -> Self {
        Self { pid, rate_hz }
    }

    pub fn run<F>(&self, mut on_event: F) -> Result<()>
    where
        F: FnMut(CpuSample),
    {
        // 1. Load skeleton
        let skel_builder = CpuSamplerSkelBuilder::default();
        let mut open_object = MaybeUninit::uninit();
        let open_skel = skel_builder
            .open(&mut open_object)
            .context("failed to open eBPF skeleton")?;

        let skel = open_skel
            .load()
            .context("failed to load cpu_sampler into kernel")?;

        // 2. Set target PID
        let key: u32 = 0;
        skel.maps
            .target_cfg
            .update(&key.to_ne_bytes(), &self.pid.to_ne_bytes(), MapFlags::ANY)
            .context("failed to set target_pid in target_cfg map")?;

        // 3. Attach perf event
        let perf_fd = open_perf_event(self.pid, self.rate_hz)
            .context("failed to open perf event — try running with sudo")?;

        let _link = skel
            .progs
            .sample_cpu
            .attach_perf_event(perf_fd)
            .context("failed to attach eBPF program to perf event")?;

        // 4. Ring buffer — need a raw pointer to stack_traces map
        //
        // The closure captures `on_event` (mut) and needs to read from
        // `stack_traces`. We can't move `skel` into the closure because
        // we still need `skel.maps.events` to build the ring buffer.
        //
        // Solution: get a raw pointer to the stack_traces map fd.
        // The skel outlives the ring buffer so this is safe.
        let stack_traces_fd = skel.maps.stack_traces.as_fd();

        let mut rb_builder = RingBufferBuilder::new();
        rb_builder
            .add(&skel.maps.events, move |data: &[u8]| -> i32 {
                let mut raw = CpuEvent::default();
                if plain::copy_from_bytes(&mut raw, data).is_err() {
                    return 0;
                }

                // Look up the stack from the STACK_TRACE map using the id
                // the eBPF program stored.
                // STACK_TRACE maps are designed to be read from userspace.
                let stack = if raw.user_stack_id >= 0 {
                    lookup_stack(stack_traces_fd, raw.user_stack_id as u32).unwrap_or_default()
                } else {
                    Vec::new()
                };

                on_event(CpuSample {
                    pid: raw.pid,
                    tid: raw.tid,
                    timestamp_ns: raw.timestamp_ns,
                    stack,
                });
                0
            })
            .context("failed to register ring buffer callback")?;

        let ring = rb_builder.build().context("failed to build ring buffer")?;

        // 5. Poll until interrupted
        loop {
            match ring.poll(Duration::from_millis(100)) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => break,
                Err(e) => bail!("ring buffer poll error: {e}"),
            }
        }

        Ok(())
    }
}

// stack lookup
/// Read stack addresses from a STACK_TRACE map by stack_id.
/// STACK_TRACE maps CAN be read from userspace — this is their intended use.
/// The eBPF program stores the id, userspace reads the addresses.
fn lookup_stack(map_fd: std::os::fd::BorrowedFd, stack_id: u32) -> Result<Vec<u64>> {
    // We need to use the raw map fd to do a lookup.
    // libbpf_rs Map::lookup takes &self so we re-wrap the fd.
    // Simpler: use the libc bpf() syscall directly for the lookup.

    const MAX_STACK_DEPTH: usize = 127;
    let mut value = [0u8; MAX_STACK_DEPTH * 8];
    let key = stack_id.to_ne_bytes();

    // BPF_MAP_LOOKUP_ELEM syscall
    // attr layout: map_fd, key ptr, value ptr
    #[repr(C)]
    struct BpfAttr {
        map_fd: u32,
        _pad1: u32,
        key: u64,
        value: u64,
        flags: u64,
    }

    use std::os::fd::AsRawFd;
    let attr = BpfAttr {
        map_fd: map_fd.as_raw_fd() as u32,
        _pad1: 0,
        key: key.as_ptr() as u64,
        value: value.as_mut_ptr() as u64,
        flags: 0,
    };

    let ret = unsafe {
        libc::syscall(
            libc::SYS_bpf,
            1u64, // BPF_MAP_LOOKUP_ELEM = 1
            &attr as *const BpfAttr,
            std::mem::size_of::<BpfAttr>() as u64,
        )
    };

    if ret != 0 {
        // Stack id not found — this can happen if the map is full
        // or the entry was evicted. Not an error, just return empty.
        return Ok(Vec::new());
    }

    // Parse addresses — stop at first zero
    let stack: Vec<u64> = value
        .chunks_exact(8)
        .map(|b| u64::from_ne_bytes(b.try_into().unwrap()))
        .take_while(|&addr| addr != 0)
        .collect();

    Ok(stack)
}

fn open_perf_event(pid: u32, rate_hz: u64) -> Result<i32> {
    let mut attr: perf_event_attr = unsafe { std::mem::zeroed() };

    attr.type_ = PERF_TYPE_SOFTWARE;
    attr.size = std::mem::size_of::<perf_event_attr>() as u32;
    attr.config = PERF_COUNT_SW_CPU_CLOCK as u64;

    // sample_period/sample_freq is a union in the kernel API.
    // In these bindings, writing sample_freq is done through the union field.
    attr.__bindgen_anon_1.sample_freq = rate_hz;

    // Set the real freq bit from the kernel-backed struct/bindings.
    attr.set_freq(1);

    let fd = unsafe {
        libc::syscall(
            libc::SYS_perf_event_open,
            &attr as *const perf_event_attr,
            pid as i32,
            -1i32,
            -1i32,
            PERF_FLAG_FD_CLOEXEC as u64,
        )
    };

    if fd < 0 {
        bail!("perf_event_open failed: {}", Error::last_os_error());
    }

    Ok(fd as i32)
}
