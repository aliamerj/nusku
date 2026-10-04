---
layout: home

hero:
  name: "Nusku"
  text: "Watch your code while it runs, not after it's already slow"
  tagline: A continuous profiling system for Linux, built in Zig, with eBPF doing the actual watching
  actions:
    - theme: brand
      text: Read the documentation
      link: /documentation/
    - theme: alt
      text: Dev log
      link: /devlog/

features:
  - title: See it live
    details: Watch CPU time, memory, and blocked threads while you actually use the app, not in a report you read an hour later.
  - title: Nothing hidden
    details: Allocations as they happen, locks that quietly stall a thread, syscalls that eat milliseconds a normal CPU graph never shows you.
  - title: Local or remote
    details: Point it at a process on your own machine with nothing to configure, or watch a server the same way over a secure connection.
  - title: Built in the open
    details: One person's systems programming project, written by hand in Zig, documented as it happens.
---

## What this actually is

Point Nusku at a running process, or launch one through it, and watch what's actually happening while you use the app. CPU time per function, live. Memory as it's allocated, not just a number going up, but what called for it and where. Threads that look idle but are actually stuck waiting on a lock, a disk read, or another process, and for how long.

It runs locally with nothing to configure. Point it at a process and watch. It can also run on a remote machine and stream the same live picture back over a secure connection, so a server can be watched the same way as something on your own laptop.

## Where it's going

Right now the focus is watching a single process well, with a clean command line to drive it. After that: watching from a distance, remembering what's been observed instead of only ever seeing it live, and eventually figuring out what's worth watching on its own. No timeline attached to any of it. The [documentation](/documentation/) covers the whole picture, and the [devlog](/devlog/) tracks the real, unpolished progress as it happens.
