const std = @import("std");
const cli = @import("cli");
const commands = @import("commands");

pub fn main(init: std.process.Init) u8 {
    const root = commands.root;
    const arena = init.arena.allocator();

    // Drop argv[0]: the framework only wants what the user typed after it.
    const raw = init.minimal.args.toSlice(arena) catch return 1;
    const argv = arena.alloc([]const u8, raw.len - 1) catch return 1;
    for (raw[1..], argv) |src, *dst| dst.* = src;

    var out_buf: [4096]u8 = undefined;
    var err_buf: [1024]u8 = undefined;
    var out = std.Io.File.stdout().writer(init.io, &out_buf);
    var err = std.Io.File.stderr().writer(init.io, &err_buf);

    return cli.execute(&root, argv, .{
        .allocator = init.gpa,
        .stderr = &err.interface,
        .stdout = &out.interface,
        .env = cli.EnvLookup.fromMap(init.environ_map),
        .version = "0.1.0",
    });
}
