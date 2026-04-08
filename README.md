# nusku
Real-time native process profiler for Linux. See CPU usage and hot functions live
in your terminal. Zero configuration, single binary, works on any Linux process.

 
```
── PID 132612 ── 98 samples ── CPU  99.0% ── RSS 2.0 MiB ── VIRT 3.1 MiB ──
     %     COUNT  FUNCTION                          SOURCE                           ADDRESS
────────────────────────────────────────────────────────────────────────────────────────────────
 27.6%        27  <core::ops::range::Range<T> as …  range.rs:773          0x00005efa8cfd4b95
 11.2%        11  testing::hot_c                    main.rs:7             0x00005efa8cfd4cc7
 10.2%        10  <i32 as core::iter::range::Step…  range.rs:197          0x00005efa8cfd4acd
 10.2%        10  <core::ops::range::Range<T> as …  range.rs:775          0x00005efa8cfd4b9f
  9.2%         9  testing::hot_c                    main.rs:8             0x00005efa8cfd4cf0
  6.1%         6  <core::ops::range::Range<T> as …  range.rs:771          0x00005efa8cfd4b69
  5.1%         5  <i32 as core::iter::range::Step…  range.rs:198          0x00005efa8cfd4b0b
  4.1%         4  core::hint::black_box             hint.rs:482           0x00005efa8cfd4d5a
  4.1%         4  core::iter::range::<impl core::…  range.rs:856          0x00005efa8cfd4b40
  3.1%         3  <core::ops::range::Range<T> as …  range.rs:776          0x00005efa8cfd4bba
  2.0%         2  <core::ops::range::Range<T> as …  range.rs:772          0x00005efa8cfd4b7a
  2.0%         2  <core::ops::range::Range<T> as …  range.rs:780          0x00005efa8cfd4bce
  2.0%         2  testing::hot_c                    main.rs:9             0x00005efa8cfd4d09
  2.0%         2  <i32 as core::iter::range::Step…  range.rs:195          0x00005efa8cfd4ac4
  1.0%         1  core::hint::black_box             hint.rs:483           0x00005efa8cfd4d64

Top frame:
  <core::ops::range::Range<T> as core::iter::range::RangeIteratorImpl>::spec_next
  /home/ali/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/iter/range.rs
  line 773

```
 
---
 
## Requirements
 
- Linux kernel 5.8+ (for ring buffer support)
- Root / `CAP_BPF` to attach eBPF probes

## Problem Statement
 
There was no tool that could show live, in the terminal  
what was happening inside the process while it handled requests or do something. Tools like `perf` require
expertise, produce cryptic output, and are post-hoc. Tools like Parca are production monitoring
systems, not developer tools. Visual Studio's diagnostic panel on Windows is the closest thing
to what developers actually need, but it does not exist for Linux native development.
 
Nusku fills this gap.
 
**One sentence:** Attach to any Linux process, see CPU, memory, and hot functions live in your
terminal, with zero configuration.
 
---
 
## Goals
 
### Stage 1   Local Developer Tool (current focus)
- Attach to any running Linux process by PID, or launch a process and attach immediately
- Show live CPU usage, memory usage, and hot functions in a clean terminal UI
- Record a session and replay/analyze it after the process exits
- Zero configuration, one binary, no daemons, no config files
- Works on any Linux binary: C, C++, Zig, Go, Rust language agnostic
 
### Stage 2 Deeper Analysis
- Allocation tracking: malloc/free rate, who allocates most
- Syscall breakdown: which syscalls, frequency, blocking time
- Flamegraph rendered: in terminal
- Baseline comparison: record before/after a code change, diff them
 
