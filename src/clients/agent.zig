const std = @import("std");
const log = @import("log");

const slog = log.scope("agent_client");

pub const Metric = enum { cpu, offcpu, mem, io, net, locks, sched };

pub const Agent = struct {
    pid: ?i32,
    io: std.Io,
    metrics: std.EnumSet(Metric),
    interval: i64,
    duration: i64,

    pub fn initExec(self: *Agent, allocator: std.mem.Allocator, exec: []const u8, args: []const []const u8) !void {
        slog.info("launching target process", .{
            .exec = exec,
            .args = args.len,
        });
        var child_args = try allocator.alloc([]const u8, args.len + 1);
        defer allocator.free(child_args);

        child_args[0] = exec;
        @memcpy(child_args[1..], args);

        const child = std.process.spawn(self.io, .{ .argv = child_args }) catch |err| {
            slog.err("failed to launch target process", .{
                .exec = exec,
                .cause = err,
            });
            return err;
        };

        self.pid = child.id;

        slog.info("target process started", .{
            .pid = child.id,
        });
    }

    pub fn run(self: *Agent) !void {
        const pid = self.pid orelse return error.NoProcess;

        slog.debug("checking target process", .{
            .pid = pid,
        });

        var path_buf: [64]u8 = undefined;
        const path = try std.fmt.bufPrint(
            &path_buf,
            "/proc/{d}",
            .{pid},
        );

        var dir = std.Io.Dir.openDirAbsolute(self.io, path, .{}) catch |err| {
            switch (err) {
                error.FileNotFound => {
                    slog.err("target process does not exist", .{
                        .pid = pid,
                    });
                    return error.ProcessNotFound;
                },
                else => {
                    slog.err("failed to inspect target process", .{
                        .pid = pid,
                        .cause = err,
                    });
                    return err;
                },
            }
        };

        defer dir.close(self.io);

        slog.info("agent started", .{
            .pid = pid,
        });
    }
};
