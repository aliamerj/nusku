//! Command tree, flag definitions, and the Context handed to `run`.
//!
//! Everything here is plain data. Commands are declared as `const` values and
//! wired together with pointers, so a whole tree can live in static memory.

const std = @import("std");
const errors = @import("errors.zig");

pub const FlagKind = enum { bool, string, int };

pub const Flag = struct {
    /// Long name without dashes: `verbose` for `--verbose`.
    name: []const u8,
    /// Optional one-character alias: 'v' for `-v`.
    short: ?u8 = null,
    kind: FlagKind = .string,
    help: []const u8 = "",
    /// Textual default. When null: false / "" / 0 depending on `kind`.
    default: ?[]const u8 = null,
    /// Environment variable consulted when the flag is not on the command line.
    env: ?[]const u8 = null,
    /// Usage error if the flag is not supplied by CLI or env.
    required: bool = false,

    pub fn defaultText(self: Flag) []const u8 {
        if (self.default) |d| return d;
        return switch (self.kind) {
            .bool => "false",
            .string => "",
            .int => "0",
        };
    }
};

pub const ArgsRule = union(enum) {
    any,
    none,
    exact: usize,
    min: usize,
    max: usize,
    range: struct { min: usize, max: usize },

    pub fn check(self: ArgsRule, n: usize) bool {
        return switch (self) {
            .any => true,
            .none => n == 0,
            .exact => |k| n == k,
            .min => |k| n >= k,
            .max => |k| n <= k,
            .range => |r| n >= r.min and n <= r.max,
        };
    }
};

pub const RunFn = *const fn (ctx: *Context) anyerror!void;

pub const Command = struct {
    name: []const u8,
    aliases: []const []const u8 = &.{},
    /// One line, shown in the parent's command list.
    short: []const u8 = "",
    /// Longer description, shown in this command's own help.
    long: ?[]const u8 = null,
    /// Positional argument synopsis for the usage line, e.g. "<file>...".
    usage_args: []const u8 = "",
    /// Local flags: only visible on this command.
    flags: []const Flag = &.{},
    /// Flags visible on this command and every descendant.
    persistent_flags: []const Flag = &.{},
    subcommands: []const *const Command = &.{},
    args: ArgsRule = .any,
    hidden: bool = false,
    /// Null for pure grouping commands (they print help when invoked).
    run: ?RunFn = null,

    pub fn matches(self: *const Command, word: []const u8) bool {
        if (std.mem.eql(u8, self.name, word)) return true;
        for (self.aliases) |a| if (std.mem.eql(u8, a, word)) return true;
        return false;
    }

    pub fn findSubcommand(self: *const Command, word: []const u8) ?*const Command {
        for (self.subcommands) |sub| if (sub.matches(word)) return sub;
        return null;
    }
};

pub const ValueSource = enum { default, env, cli };

/// Resolved flag values for one invocation. Every flag visible to the
/// selected command has an entry (defaults are filled in by the parser).
pub const FlagValues = struct {
    pub const Entry = struct {
        value: []const u8,
        source: ValueSource,
    };

    map: std.StringHashMapUnmanaged(Entry) = .empty,

    pub fn get(self: *const FlagValues, name: []const u8) ?Entry {
        return self.map.get(name);
    }

    fn must(self: *const FlagValues, name: []const u8) Entry {
        return self.map.get(name) orelse
            std.debug.panic("cli: flag --{s} is not defined for this command", .{name});
    }

    /// True when the user supplied the flag (CLI or env), false for defaults.
    pub fn isSet(self: *const FlagValues, name: []const u8) bool {
        return self.must(name).source != .default;
    }

    pub fn getBool(self: *const FlagValues, name: []const u8) bool {
        return std.mem.eql(u8, self.must(name).value, "true");
    }

    pub fn getString(self: *const FlagValues, name: []const u8) []const u8 {
        return self.must(name).value;
    }

    pub fn getInt(self: *const FlagValues, name: []const u8) i64 {
        const v = self.must(name).value;
        return std.fmt.parseInt(i64, v, 0) catch
            std.debug.panic("cli: flag --{s} holds non-integer default \"{s}\"", .{ name, v });
    }
};

/// What a command's `run` function receives. All output goes through the two
/// writers so commands never touch the process streams directly.
pub const Context = struct {
    io: std.Io,
    /// Arena that lives for the duration of this invocation.
    allocator: std.mem.Allocator,
    /// Name of the root command, used as the "prog:" prefix on messages.
    prog: []const u8,
    /// Results only. This is what a pipe or redirect captures.
    stdout: *std.Io.Writer,
    /// Diagnostics, progress, warnings.
    stderr: *std.Io.Writer,
    command: *const Command,
    /// Positional arguments, after the command path and flags are removed.
    args: []const []const u8,
    flags: *const FlagValues,

    /// Print "prog: message" to stderr and return an error that `execute`
    /// maps to exit code 1 without printing anything more.
    pub fn fail(self: *const Context, comptime fmt: []const u8, args: anytype) error{Reported} {
        self.stderr.print("{s}: " ++ fmt ++ "\n", .{self.prog} ++ args) catch {};
        return error.Reported;
    }

    /// Like `fail`, but for bad input the parser could not catch. `execute`
    /// adds the "Run 'app cmd --help'" hint and exits with code 2.
    pub fn usageFail(self: *const Context, comptime fmt: []const u8, args: anytype) error{InvalidUsage} {
        self.stderr.print("{s}: " ++ fmt ++ "\n", .{self.prog} ++ args) catch {};
        return error.InvalidUsage;
    }

    pub fn getBool(self: *const Context, name: []const u8) bool {
        return self.flags.getBool(name);
    }
    pub fn getString(self: *const Context, name: []const u8) []const u8 {
        return self.flags.getString(name);
    }
    pub fn getInt(self: *const Context, name: []const u8) i64 {
        return self.flags.getInt(name);
    }
    pub fn isSet(self: *const Context, name: []const u8) bool {
        return self.flags.isSet(name);
    }
};
