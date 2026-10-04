---
date: 2026-10-04
---

# Overview

## What is Nusku

Nusku is a continuous profiling system for Linux. It watches a running process and shows you what it's actually doing, live, instead of leaving you to guess from a graph of CPU percentage that goes up and down with no explanation.

Point it at a process, or launch one through it, and see CPU time per function, memory as it's allocated and by what call site, threads that are blocked waiting on a lock or a disk read or another process, syscalls that are quietly slow, and network calls that are failing or retransmitting. All of it close to the moment it actually happened, not in a report read later.

It's built around [eBPF](https://ebpf.io/), the same kernel technology every serious Linux profiling and observability tool uses, instead of guessing from periodically polling files in `/proc`. It's written in Zig, with no garbage collector getting in the way of measuring performance accurately.

Nusku is made of a few pieces that each do one job:

- something that sits next to a process and actually collects the data
- something that lets you pick what to watch and manage active sessions
- something that remembers what's been observed, so it can be looked back on later

## Why

I've always wanted to see the complete performance picture before pushing any code or building something. Not after it's slow in production, not after a user complains, before. Click a button, see exactly what that click cost.

This project is me practicing Zig and systems programming, for real. I'm challenging myself to see how far I can take this. No AI generated slop, no shortcuts, just me, textbooks, and the Linux kernel docs.

Nusku is the tool I kept wishing existed while doing that loop myself: something that shows you the truth about your code while it's running, built from the ground up, in a language with no garbage collector in the way.

There's no deadline on this and no plan to monetize it right now. The goal is depth. If it turns into something other people actually use along the way, that's a bonus, not the point.

## Goals

Nusku is being built one piece at a time, each one usable on its own before the next one starts.

**Right now: watching a single process.** The first goal is simple. Point Nusku at one running app, or launch one through it, and get a real, live, accurate picture of what it's doing. This is the part that has to be right before anything else matters, so it's getting the most care.

**Next: a proper command line experience.** Profiling a process shouldn't feel like fighting a CLI. Alongside the core profiler, a clean, well structured command framework in Zig handles commands, subcommands, and arguments properly, instead of bolting on argument parsing as an afterthought.

**After that: watching from a distance.** Once watching one process locally is solid, the next goal is doing the same thing for a process running somewhere else, securely, with the same live picture you'd get on your own machine.

**Then: remembering what happened.** Live viewing is only half the problem. The next goal is storing what's been observed so it can be looked back on later, compared over time, and queried instead of only ever watched in the moment.

**Longer term: knowing what to watch automatically.** Eventually, Nusku should be able to figure out what's running and worth watching on its own, instead of always being told.

No timeline is attached to any of this. Each goal gets finished properly before the next one starts, and the [dev log](/devlog/) tracks the real progress, not a projected one.
