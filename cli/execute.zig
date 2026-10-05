//! The entry point that ties parsing, help, and exit codes together.
//!
//! Rules enforced here:
//!   results         -> stdout
//!   --help, --version (asked for) -> stdout, exit 0
//!   usage problems  -> stderr, exit 2, with a "Run 'app cmd --help'" hint
//!   runtime errors  -> stderr, exit 1
//! stdout is flushed before anything goes to stderr, so a terminal shows
//! the two in the order they happened.

const std = @import("std");
const command = @import("command.zig");
const parser = @import("parser.zig");
const help = @import("help.zig");
const errors = @import("errors.zig");

pub const Options = struct {
    io: std.Io,
    /// Backing allocator. `execute` makes an arena from it per invocation.
    allocator: std.mem.Allocator,
    stdout: *std.Io.Writer,
    stderr: *std.Io.Writer,
    env: ?parser.EnvLookup = null,
    /// When set, `app --version` prints "app <version>" and exits 0.
    version: ?[]const u8 = null,
};

/// `argv` excludes the program name. Returns the process exit code; the
/// caller decides when to exit so tests can call this directly.
pub fn execute(root: *const command.Command, argv: []const []const u8, opts: Options) u8 {
    var arena: std.heap.ArenaAllocator = .init(opts.allocator);
    defer arena.deinit();

    const code = run(arena.allocator(), root, argv, opts);

    opts.stdout.flush() catch {};
    opts.stderr.flush() catch {};
    return code;
}

fn run(arena: std.mem.Allocator, root: *const command.Command, argv: []const []const u8, opts: Options) u8 {
    if (opts.version) |v| {
        if (argv.len == 1 and isVersion(argv[0])) {
            opts.stdout.print("{s} {s}\n", .{ root.name, v }) catch return errors.exit_failure;
            return errors.exit_success;
        }
    }

    var diag: parser.Diagnostic = .{};
    const parsed = parser.parse(arena, root, argv, .{ .env = opts.env }, &diag) catch |err| switch (err) {
        error.OutOfMemory => {
            emit(opts, "{s}: out of memory\n", .{root.name});
            return errors.exit_failure;
        },
        else => {
            emit(opts, "{s}: {s}\n", .{ root.name, diag.message });
            help.writeUsageHint(opts.stderr, diag.path) catch {};
            return errors.exit_usage;
        },
    };

    // Asked for help, or a grouping command with nothing to run: success.
    if (parsed.help_requested or parsed.command.run == null) {
        help.writeHelp(opts.stdout, parsed.path) catch return errors.exit_failure;
        return errors.exit_success;
    }

    var ctx: command.Context = .{
        .io = opts.io,
        .allocator = arena,
        .prog = root.name,
        .stdout = opts.stdout,
        .stderr = opts.stderr,
        .command = parsed.command,
        .args = parsed.args,
        .flags = &parsed.flags,
    };

    parsed.command.run.?(&ctx) catch |err| switch (err) {
        // Message already printed by ctx.fail.
        error.Reported => return errors.exit_failure,
        error.InvalidUsage => {
            help.writeUsageHint(opts.stderr, parsed.path) catch {};
            return errors.exit_usage;
        },
        // Writing to stdout failed, usually because the reader went away
        // (`app list | head`). Nothing useful to say.
        error.WriteFailed => return errors.exit_failure,
        else => {
            emit(opts, "{s}: error: {s}\n", .{ root.name, @errorName(err) });
            return errors.exit_failure;
        },
    };

    return errors.exit_success;
}

/// Write a diagnostic to stderr, flushing stdout first to keep ordering.
fn emit(opts: Options, comptime fmt: []const u8, args: anytype) void {
    opts.stdout.flush() catch {};
    opts.stderr.print(fmt, args) catch {};
}

fn isVersion(arg: []const u8) bool {
    if (std.mem.eql(u8, arg, "--version") or std.mem.eql(u8, arg, "-v")) {
        return true;
    }
    return false;
}
