# nusku
Real-time native process profiler for Linux. See CPU usage and hot functions live
in your terminal. Zero configuration, single binary, works on any Linux process.

 
```
── PID 4821 ── 99 samples ─────────────────────────
 %CPU  FUNCTION
──────────────────────────────────────────────────────────────────────
38.2%  handle_request  proxy.zig:142
21.4%  ssl_handshake   wolfssl/ssl.c:891
18.1%  thread_pool_dispatch  pool.zig:67
 9.3%  epoll_wait  [kernel]
 7.0%  parse_headers  proxy.zig:89
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
 
