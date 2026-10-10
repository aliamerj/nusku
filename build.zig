const std = @import("std");

const agent = @import("agent/build.zig");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{
        .default_target = .{ .cpu_arch = .x86_64, .os_tag = .linux, .abi = .musl },
    });
    const optimize = b.standardOptimizeOption(.{});

    const cli_mod = b.createModule(.{
        .root_source_file = b.path("cli/root.zig"),
        .target = target,
        .optimize = optimize,
    });

    const log_mod = b.createModule(.{
        .root_source_file = b.path("logger/root.zig"),
        .target = target,
        .optimize = optimize,
    });

    const agent_mod = agent.build(b, target, optimize);

    const exe = b.addExecutable(.{
        .name = "nusku",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    exe.root_module.addImport("cli", cli_mod);
    exe.root_module.addImport("log", log_mod);
    exe.root_module.addImport("agent", agent_mod);

    b.installArtifact(exe);

    const run_step = b.step("run", "Run the app");

    const run_cmd = b.addRunArtifact(exe);
    run_step.dependOn(&run_cmd.step);

    run_cmd.step.dependOn(b.getInstallStep());

    if (b.args) |args| {
        run_cmd.addArgs(args);
    }

    const mod_tests = b.addTest(.{ .root_module = cli_mod });
    const run_mod_tests = b.addRunArtifact(mod_tests);

    const agent_tests = b.addTest(.{ .root_module = agent_mod });
    const run_agent_tests = b.addRunArtifact(agent_tests);

    const run_ebpf_tests = agent.runTests(b, target, optimize);

    const exe_tests = b.addTest(.{ .root_module = exe.root_module });

    const run_exe_tests = b.addRunArtifact(exe_tests);

    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&run_mod_tests.step);
    test_step.dependOn(&run_exe_tests.step);
    test_step.dependOn(&run_ebpf_tests.step);
    test_step.dependOn(&run_agent_tests.step);
}
