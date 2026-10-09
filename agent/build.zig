const std = @import("std");

pub fn buildAgent(
    b: *std.Build,
    target: std.Build.ResolvedTarget,
    optimize: std.builtin.OptimizeMode,
) *std.Build.Module {
    const agent = b.addModule("agent", .{
        .root_source_file = b.path("agent/root.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });

    agent.addAnonymousImport("ebpf", .{
        .root_source_file = b.path("agent/ebpf/root.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });

    // Kernel-side BPF program, written in Zig and compiled for the BPF target.
    // Optimized, because the verifier rejects unoptimized code; not stripped, because
    // libbpf needs the debug info (it becomes BTF).
    const oncpu_obj = b.addObject(.{
        .name = "oncpu",
        .root_module = b.createModule(.{
            .root_source_file = b.path("agent/ebpf/kernel/oncpu.ebpf.zig"),
            .target = b.resolveTargetQuery(.{
                .cpu_arch = .bpfel,
                .os_tag = .freestanding,
            }),
            .optimize = .ReleaseSmall,
            .strip = false,
        }),
    });

    agent.addImport(
        "oncpu_bpf",
        b.createModule(.{
            .root_source_file = oncpu_obj.getEmittedBin(),
        }),
    );

    // Debug only: copy the object to zig-out/bpf/ so we can inspect it.
    b.getInstallStep().dependOn(&b.addInstallFile(
        oncpu_obj.getEmittedBin(),
        "bpf/oncpu.bpf.o",
    ).step);

    return agent;
}
