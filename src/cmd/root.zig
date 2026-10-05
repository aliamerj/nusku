const std = @import("std");
const cli = @import("cli");
const log = @import("log");

/// Persistent flags to add to the root command.
const flags: []const cli.Flag = &.{
    .{ .name = "verbose", .short = 'v', .kind = .bool, .help = "Same as --log-level=debug" },
    .{ .name = "log-level", .help = "trace, debug, info, warn, error, off", .env = "NUSKU_LOG_LEVEL" },
    .{ .name = "log-format", .help = "text, logfmt or json", .env = "NUSKU_LOG_FORMAT" },
};

pub const root: cli.Command = .{
    .name = "nusku",
    .short = "A continuous profiling system for Linux, built in Zig, with eBPF doing the actual watching",
    .long = "Nusku is a continuous profiling system for Linux. It watches a running process and shows you what it's actually doing, live, instead of leaving you to guess from a graph of CPU percentage that goes up and down with no explanation",
    .persistent_flags = flags,
};

/// Call first thing in every command's `run`.
pub fn configure(ctx: *cli.Context) error{InvalidUsage}!void {
    const lg = log.getDefault() orelse return;

    if (ctx.getBool("verbose")) lg.setLevel(.debug);

    if (ctx.isSet("log-level")) {
        const text = ctx.getString("log-level");
        const level = log.Level.parse(text) orelse
            return ctx.usageFail("unknown --log-level \"{s}\" (trace, debug, info, warn, error, off)", .{text});
        lg.setLevel(level);
    }

    if (ctx.isSet("log-format")) {
        const text = ctx.getString("log-format");
        const format = std.meta.stringToEnum(log.Format, text) orelse
            return ctx.usageFail("unknown --log-format \"{s}\" (text, logfmt, json)", .{text});
        lg.setFormat(format);
    }
}
