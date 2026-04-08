// SPDX-License-Identifier: GPL-2.0

/* clang-format off */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>
/* clang-format on */

#define MAX_STACK_DEPTH 127

struct cpu_event {
  __u32 pid;
  __u32 tid;
  __u64 timestamp_ns;
  __s32 user_stack_id;
};

struct {
  __uint(type, BPF_MAP_TYPE_RINGBUF);
  __uint(max_entries, 256 * 1024);
} events SEC(".maps");

struct {
  __uint(type, BPF_MAP_TYPE_STACK_TRACE);
  __uint(key_size, sizeof(__u32));
  __uint(value_size, MAX_STACK_DEPTH * sizeof(__u64));
  __uint(max_entries, 16384);
} stack_traces SEC(".maps");

struct {
  __uint(type, BPF_MAP_TYPE_ARRAY);
  __uint(max_entries, 1);
  __type(key, __u32);
  __type(value, __u32);
} target_cfg SEC(".maps");

SEC("perf_event")
int sample_cpu(struct bpf_perf_event_data *ctx) {
  __u64 pid_tgid = bpf_get_current_pid_tgid();
  __u32 pid = pid_tgid >> 32;
  __u32 tid = (__u32)pid_tgid;

  __u32 key = 0;
  __u32 *target = bpf_map_lookup_elem(&target_cfg, &key);
  if (target && *target != 0 && pid != *target)
    return 0;

  struct cpu_event *event = bpf_ringbuf_reserve(&events, sizeof(*event), 0);
  if (!event)
    return 0;

  event->pid = pid;
  event->tid = tid;
  event->timestamp_ns = bpf_ktime_get_ns();
  event->user_stack_id = bpf_get_stackid(ctx, &stack_traces, BPF_F_USER_STACK);

  bpf_ringbuf_submit(event, 0);
  return 0;
}

char LICENSE[] SEC("license") = "GPL";
