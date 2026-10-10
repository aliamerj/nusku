---
date: 2026-10-10
---

# Ep7: How Nusku Collects CPU Samples

## 1. Introduction

Nusku uses CPU sampling to understand where a process spends its execution time.

Instead of recording every function call, Nusku periodically captures the call stack of a running process. Each sample shows which functions were active at that moment.

By collecting many samples and counting how often each stack appears, Nusku can estimate which parts of a program consume the most CPU time.

## 2. The General Algorithm

The algorithm is simple:

1. Periodically take a sample of the target process.
2. Capture its current call stack.
3. Check whether this stack has been seen before.
4. If it is new, store it. Otherwise, reuse the existing stack.
5. Increment the sample count for that stack.

### Example

Imagine profiling process `1999` and collecting 100 samples.

The first sample produces this stack:

`main → handle_request → parse`

Nusku stores the stack and sets its count to `1`.

If the same stack appears again, Nusku increments its count instead of storing another copy.

Later, another stack appears:

`main → handle_request → send`

Nusku stores this new stack separately.

After 100 samples, the results might look like this:

**Unique stacks**

| Stack ID | Call stack |
|---|---|
| 1 | `main → handle_request → parse` |
| 2 | `main → handle_request → send` |

**Sample counts**

| Stack ID | Samples |
|---|---:|
| 1 | 65 |
| 2 | 35 |

The counts add up to 100 samples. The first stack appeared in 65 samples, while the second appeared in 35.

This is the core idea: **store each unique stack once and count how often it appears.**

### Algorithm diagram

[![](https://mermaid.ink/img/pako:eNpVj1FPgzAUhf_KzX2GBecAR6Jmg23xxZhsxijs4QY6RgYtKSUyGf_dAi5qn3puv3tOT4uxSBh6eMjFZ3wkqWAXRBz0WYQ7OjEg8F9eoaKizNkeTPMBlqFPpaolg5jyHCpF8Wk_7iwHwG-3_Qwol4ySsyaEZMljNzJ-z1yexQWCcNu__HMIBodV6OtVpRNEzRXcw83-7_I7qy6wDp94LFnBNMCarFIZT0f-h10NVpvwjTIFByGBs0Zdm4zIekRGsRnEAg1MZZagd6C8YgYWTBbUa2x7LkJ11KERevqqa9WNmZA8mbHIhYww4p02KIl_CFGgp2StLaSo0-NV1GWimwUZpZJ-CcYTJv3-9-jN5oMDei02WtnuxJnazmzq3lhzrQw8o2dbE1tPLctyHWdqzW47A7-GSGty59rdNyHgkTE?type=png)](https://mermaid.live/edit#pako:eNpVj1FPgzAUhf_KzX2Ghc0Bk0TNBtviizGZxijsoYE7RgYtKSUyGf_dAi5qn3puv3NuT4uxSAg9POTiMz4yqeAliDjoswxf2ImAgf_8ChUrypz2YJr3sAp9VqpaEsQsz6FSLD7tR89qAPx218-A5ZJYctaEkJQ8dCPj98zlSVwgCHf9y7-EYEhYh762Kr1B1FzBHUz3f83vVF1gEz7yWFJBGqAmq1TG05H_YddD1DZ8Y5mCg5DAqVHXJiOyGZFRbAexRANTmSXoKVmTgQXJgvUS2x6LUB31zgg9fdWt6sZMmDyZsciFjDDinfaXjH8IUVwjpKjT41XUZaKLBRlLJfsliCck_f7z6M0XQwJ6LTZa2e7EmdnOfOZOrVutDDyjZ1sTW08ty3IdZ2bNbzoDv4aV1mTh2t03kwWQ5Q)

The resulting sample counts estimate where the process spends its CPU time. They are statistical estimates, not exact measurements of execution time.

## 3. How Linux Collects Samples

Nusku uses Linux `perf_event` and eBPF to implement this algorithm efficiently.

The collection flow looks like this:

[![](https://mermaid.ink/img/pako:eNpdkEGP0zAQhf_KyOe2Ct2mhRyQtul2dwGhCtgLSYWseJJaTewwtqGl6X_HTlIE5JTxfPPmvbmwQgtkCStr_bM4cLLwZZMr8N999tGZowNnkEzLCwReobJ7mE7fdqlWpawcIVhOFVrgSkDDW9PBOvsglTvBEUlhvR_F_psyvGlrqSooCb87VMW5gzRrkcpv-CNsGcbSfmyHJLWQBaS7l2ESO9hkuN5toSVdEW9GfhN4eMi2srZIN2-eKdCYkXnomW2W8tYGKyFfb38wDMby4niDtz38mH22OrgOLbDkj_Ev8JS9tILbMRdCoZ2yN-SxR56z94N-cB0uNXafhu5QPPd5PyEXXqKusbAowAvzDu7_OiS8y9ZO1iIkK6XfV-oQgddnI82eTVhFUrCk5LXBCWuQGh5qdgkaObMHbDBnif8lFO40FZyOU79QU85ydfUCLVdftW5YYsl5CdKuOtwK10fdSB4O_2cLoRJIaQjOkviul2DJhZ1YsohXs-U8Xi7mq1fRG19N2Nkz0Sz2r1EUrZbLebS4u07Yr35nNHu9iq-_AUxb1Yw?type=png)](https://mermaid.live/edit#pako:eNpVj1FPgzAUhf_KzX2GBecAR6Jmg23xxZhsxijs4QY6RgYtKSUyGf_dAi5qn3puv3tOT4uxSBh6eMjFZ3wkqWAXRBz0WYQ7OjEg8F9eoaKizNkeTPMBlqFPpaolg5jyHCpF8Wk_7iwHwG-3_Qwol4ySsyaEZMljNzJ-z1yexQWCcNu__HMIBodV6OtVpRNEzRXcw83-7_I7qy6wDp94LFnBNMCarFIZT0f-h10NVpvwjTIFByGBs0Zdm4zIekRGsRnEAg1MZZagd6C8YgYWTBbUa2x7LkJ11KERevqqa9WNmZA8mbHIhYww4p02KIl_CFGgp2StLaSo0-NV1GWimwUZpZJ-CcYTJv3-9-jN5oMDei02WtnuxJnazmzq3lhzrQw8o2dbE1tPLctyHWdqzW47A7-GSGty59rdNyHgkTE)

### `perf_event`: when to sample

Linux `perf_event` provides the sampling trigger. Nusku configures a sampling frequency, such as approximately 99 samples per second per CPU.

When a sampling event occurs, the attached eBPF program runs. The program checks the current process and ignores samples that do not belong to the target.

### eBPF: what to collect

The eBPF program executes in the kernel. For a sample belonging to the target process, it captures the current user-space and kernel-space call stacks.

It then updates the maps that hold stack traces and sample counts. This aggregation happens as samples arrive, rather than requiring Nusku to store every sample as a separate record.

## 4. The Three BPF Maps

The eBPF program uses three maps, each with a distinct responsibility.

| Map | What it stores | Purpose |
|---|---|---|
| `target_pid` | Target process ID | Tells the program which process to profile |
| `stacks` | Stack ID → return addresses | Stores captured stack traces |
| `counts` | Stack key → sample count | Tracks how often each unique stack appears |

The `counts` map uses a key containing the process ID and the user and kernel stack IDs. This allows samples to be aggregated by their stack combination.

The `stacks` map stores the actual stack addresses. The `counts` map refers to those stacks by their IDs, avoiding repeated storage of identical stack traces.

## 5. What Happens in Userspace?

The Nusku userspace agent prepares the profiling session, creates and configures the BPF maps, and attaches the eBPF program to the sampling events.

While profiling runs, the kernel updates the maps. The userspace agent can read their contents to retrieve the collected stacks and counts.

Nusku can then use that data to build a profile, resolve addresses to function names when symbol information is available, and make the results available for analysis and visualization.

The BPF maps hold the live collection data in kernel memory. If Nusku needs to preserve the results beyond the session, it must copy them into its own storage.

## 6. Summary

Nusku's on-CPU profiler combines three ideas:

- **Sampling:** periodically observe what a process is executing.
- **Stack capture:** record the call path active at each sample.
- **Aggregation:** store unique stacks and count their occurrences.

Linux `perf_event` triggers the samples, eBPF collects and aggregates them, and Nusku's userspace agent reads the results to build a usable CPU profile.

This approach avoids recording every function call while still providing a useful statistical picture of where a program spends its CPU time.
