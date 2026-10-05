//! The Logger: owns the output writer, the level threshold, and a lock so
//! that one record is never interleaved with another.
const std = @import("std");
const Writer = std.Io.Writer;
const Level = @import("level.zig").Level;
const encode = @import("encode.zig");
const Format = encode.Format;

pub const Options = struct {
    /// Where records go. For a CLI this is stderr.
    writer: *Writer,
    /// Used for the clock and the lock.
    io: std.Io,
    level: Level = .info,
    format: Format = .text,
    /// ANSI colors in the text format. Resolve it yourself (see `shouldColor`).
    color: bool = false,
    timestamps: bool = true,
    /// Flush the writer after every record. Leave on for stderr.
    flush: bool = true,
    /// Replace the clock. Returns nanoseconds since the Unix epoch.
    now_fn: ?*const fn () i96 = null,
};

pub const Logger = struct {
    writer: *Writer,
    io: std.Io,
    level: std.atomic.Value(Level),
    format: Format,
    color: bool,
    timestamps: bool,
    flush_each: bool,
    now_fn: ?*const fn () i96,
    mutex: std.Io.Mutex = .init,

    pub fn init(opts: Options) Logger {
        return .{
            .writer = opts.writer,
            .io = opts.io,
            .level = .init(opts.level),
            .format = opts.format,
            .color = opts.color,
            .timestamps = opts.timestamps,
            .flush_each = opts.flush,
            .now_fn = opts.now_fn,
        };
    }

    // Settings that can change while the program runs (for example after
    // flags are parsed). Safe to call from any thread.

    pub fn setLevel(self: *Logger, level: Level) void {
        self.level.store(level, .monotonic);
    }

    pub fn getLevel(self: *const Logger) Level {
        return self.level.load(.monotonic);
    }

    pub fn setFormat(self: *Logger, format: Format) void {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        self.format = format;
    }

    pub fn setColor(self: *Logger, color: bool) void {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        self.color = color;
    }

    pub fn enabled(self: *const Logger, level: Level) bool {
        return level != .off and @intFromEnum(level) >= @intFromEnum(self.getLevel());
    }

    /// The one place records are written. Logging never fails the caller:
    /// a broken pipe or full disk is swallowed.
    pub fn emit(self: *Logger, level: Level, scope_name: []const u8, msg: []const u8, fields: anytype) void {
        if (!self.enabled(level)) return;

        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);

        const ts: ?i96 = if (self.timestamps) self.now() else null;
        encode.writeRecord(self.writer, self.format, self.color, ts, level, scope_name, msg, fields) catch {};
        if (self.flush_each) self.writer.flush() catch {};
    }

    fn now(self: *const Logger) i96 {
        if (self.now_fn) |f| return f();
        return std.Io.Clock.real.now(self.io).nanoseconds;
    }

    pub fn trace(self: *Logger, msg: []const u8, fields: anytype) void {
        self.emit(.trace, "", msg, fields);
    }
    pub fn debug(self: *Logger, msg: []const u8, fields: anytype) void {
        self.emit(.debug, "", msg, fields);
    }
    pub fn info(self: *Logger, msg: []const u8, fields: anytype) void {
        self.emit(.info, "", msg, fields);
    }
    pub fn warn(self: *Logger, msg: []const u8, fields: anytype) void {
        self.emit(.warn, "", msg, fields);
    }
    pub fn err(self: *Logger, msg: []const u8, fields: anytype) void {
        self.emit(.err, "", msg, fields);
    }

    /// A handle that tags every record with a scope name.
    pub fn scope(self: *Logger, name: []const u8) Bound(struct {}) {
        return .{ .logger = self, .scope_name = name, .fields = .{} };
    }

    /// A handle that adds the same fields to every record.
    pub fn with(self: *Logger, fields: anytype) Bound(@TypeOf(fields)) {
        return .{ .logger = self, .fields = fields };
    }
};

/// A logger plus a scope name plus fixed fields. Cheap to copy and to build
/// on the stack. When `logger` is null it resolves the process-wide default
/// on every call, which is what lets `const slog = log.scope("db")` live at
/// file level before the logger exists.
pub fn Bound(comptime F: type) type {
    return struct {
        const Self = @This();

        logger: ?*Logger = null,
        scope_name: []const u8 = "",
        fields: F,

        pub fn emit(self: Self, level: Level, msg: []const u8, extra: anytype) void {
            const lg = self.logger orelse getDefault() orelse return;
            lg.emit(level, self.scope_name, msg, encode.Pair(F, @TypeOf(extra)){ .a = self.fields, .b = extra });
        }

        pub fn trace(self: Self, msg: []const u8, extra: anytype) void {
            self.emit(.trace, msg, extra);
        }
        pub fn debug(self: Self, msg: []const u8, extra: anytype) void {
            self.emit(.debug, msg, extra);
        }
        pub fn info(self: Self, msg: []const u8, extra: anytype) void {
            self.emit(.info, msg, extra);
        }
        pub fn warn(self: Self, msg: []const u8, extra: anytype) void {
            self.emit(.warn, msg, extra);
        }
        pub fn err(self: Self, msg: []const u8, extra: anytype) void {
            self.emit(.err, msg, extra);
        }

        /// Same handle with more fixed fields. Earlier fields print first.
        pub fn with(self: Self, extra: anytype) Bound(encode.Pair(F, @TypeOf(extra))) {
            return .{
                .logger = self.logger,
                .scope_name = self.scope_name,
                .fields = .{ .a = self.fields, .b = extra },
            };
        }

        pub fn scoped(self: Self, name: []const u8) Self {
            var copy = self;
            copy.scope_name = name;
            return copy;
        }
    };
}

// process default

var default_logger: std.atomic.Value(?*Logger) = .init(null);

/// Install the logger used by `log.info(...)` and by unattached `Bound`
/// handles. The logger must outlive its use. Call `clearDefault` before it
/// goes away.
pub fn setDefault(logger: *Logger) void {
    default_logger.store(logger, .release);
}

pub fn clearDefault() void {
    default_logger.store(null, .release);
}

pub fn getDefault() ?*Logger {
    return default_logger.load(.acquire);
}

/// Color only when asked for by the environment (`NO_COLOR` unset) and the
/// file is a terminal.
pub fn shouldColor(io: std.Io, file: std.Io.File, no_color_set: bool) bool {
    if (no_color_set) return false;
    return file.isTty(io) catch false;
}
