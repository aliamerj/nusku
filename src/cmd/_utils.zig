const std = @import("std");
const cli = @import("cli");
const log = @import("log");

/// Call first thing in every command's `run`.
pub fn logConfig(ctx: *cli.Context) error{InvalidUsage}!void {
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
