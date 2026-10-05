---
date: 2026-10-05
---

# Ep2: Working on infrastrucure for the app

Since we are using Zig and the available libraries and packages are limited, we need to implement some of the core infrastructure code that we need for this project.

On the first day, we implemented a simple, lightweight CLI framework to help with parsing arguments and interacting with the CLI.

Today, we are starting to work on something new: the logger module. Like any logging library, you write a message along with some named fields. The module renders one line per record as readable text, logfmt, or JSON, with a timestamp and a log level, and writes it to a writer that you provide, such as stderr for a CLI application.

## Quick start

In `main`, create one logger and install it as the default:
 
```zig
const log = @import("log");
 
var err_buf: [1024]u8 = undefined;
var err = std.Io.File.stderr().writer(init.io, &err_buf);
 
var logger = log.Logger.init(.{
    .writer = &err.interface,
    .io = init.io,
    .color = log.shouldColor(init.io, std.Io.File.stderr(), init.environ_map.get("NO_COLOR") != null),
});
log.setDefault(&logger);
defer log.clearDefault();
```

Then log from anywhere:
 
```zig
log.info("server started", .{ .addr = addr, .port = 7000 });
 
// or give a file or component its own scope name:
const slog = log.scope("agent");
slog.debug("sample taken", .{ .pid = pid, .took_us = elapsed });
slog.err("write failed", .{ .path = path, .cause = error.AccessDenied });
```
 
Output:
 
```
2026-10-05T08:08:02.803Z INFO  server started addr=0.0.0.0 port=7000
2026-10-05T08:08:02.803Z DEBUG agent: sample taken pid=412 took_us=88
2026-10-05T08:08:02.804Z ERROR agent: write failed path=/x cause=AccessDenied
```

## Output formats
 
Set with `Options.format` or at run time with `logger.setFormat(...)`.
 
**`.text`** (default). For people reading a terminal.
 
```
2026-10-05T14:03:22.123Z INFO  agent: listening addr=:7000 interval_ms=100
```
 
Level is padded to five characters so columns line up. With `color = true` the level is colored (trace gray, debug cyan, info green, warn yellow, error red), the timestamp and field names are dimmed. Newlines in a message are shown as `\n` so one record is always one line.
 
**`.logfmt`**. For grep and log tools that understand `key=value`.
 
```
time=2026-10-05T14:03:22.123Z level=info scope=agent msg=listening addr=:7000 interval_ms=100
```
 
**`.json`**. One object per line, for log pipelines.
 
```json
{"time":"2026-10-05T14:03:22.123Z","level":"info","scope":"agent","msg":"listening","addr":":7000","interval_ms":100}
```
 
Color is only ever applied to `.text`. The other two never contain escape codes.
 
## Levels
 
`trace < debug < info < warn < err < off`
 
A record is written when its level is at or above the logger's threshold. `off` as a threshold silences everything. The default threshold is `info`.
 
```zig
logger.setLevel(.debug);          // any thread, any time
logger.getLevel();
logger.enabled(.trace);           // false at the moment
 
log.Level.parse("WARNING")        // ?Level: case-insensitive
```
 
`Level.parse` accepts `trace`, `debug`, `info`, `warn`, `warning`, `error`, `err`, `off`, `none`. It is meant for flags and environment variables.
 
The level check happens before any formatting or locking, so a disabled call costs one atomic load. The field values you pass are still evaluated by the caller, so avoid building expensive values just to log them at debug level.
 
## Fields
 
The second argument is an anonymous struct. Each field becomes a key and a value, in the order written.
 
```zig
log.info("request", .{ .method = "GET", .status = 200, .took_ms = 12.5, .cached = false });
```
 
Supported value types and how they print:
 
| Type | Output |
|---|---|
| integers, comptime ints | decimal |
| floats | decimal. In JSON, NaN and infinity become `null` |
| `bool` | `true` / `false` |
| `[]const u8`, string literals, `[N]u8` | text. In text and logfmt it is quoted only when needed (empty, or contains a space, `=`, `"`, `\`, or a control character). In JSON it is always quoted and escaped |
| enums and enum literals | the tag name |
| error values | the error name, `cause=AccessDenied` |
| optionals | the value, or `null` |
| anything else | its `{any}` rendering as a string, cut at 512 bytes |
 
Logging never fails the caller. If the writer errors (a closed pipe, a full disk) the record is dropped silently.
 
An empty `.{}` is fine when you have no fields. A struct with positional fields (a tuple like `.{ 1, 2 }`) is a compile error, since fields need names.
 
## Scopes and bound context
 
A **scope** is a name for the part of the program that produced the record. It prints as `agent:` in text and as `scope=agent` or `"scope":"agent"` in the others.
 
A **bound** handle carries a scope and fixed fields, so you do not repeat them on every call.
 
```zig
// attached to a specific logger
const req = logger.scope("http").with(.{ .request_id = id });
req.info("handled", .{ .status = 200 });
// -> INFO  http: handled request_id=abc status=200
 
// chain more fields; earlier ones print first
const user_req = req.with(.{ .user = "ali" });
 
// same fields, different scope name
user_req.scoped("http.slow").warn("slow", .{ .ms = 1800 });
```
 
Handles are small values. Build them on the stack, pass them around, copy them freely. They hold a pointer to the logger and no allocations, so they are only valid while the logger is.
 
The module-level `log.scope(name)` and `log.with(fields)` return handles that are not attached to a logger. They look up the default logger every time you log. That is what allows a file-level constant:
 
```zig
const slog = log.scope("db");   // at file level, before main runs
 
pub fn query() void {
    slog.debug("running", .{});  // uses whatever default is installed now
}
```
 
## The default logger
 
`log.setDefault(&logger)` installs a process-wide logger. The functions `log.info(...)`, `log.debug(...)` and so on, and every unattached handle, use it. `log.clearDefault()` removes it, and `log.getDefault()` returns `?*Logger`.
 
If no default is installed, these calls do nothing. They do not crash and they do not print. This means anything logged before `main` calls `setDefault` is lost, so install it first.
 
The default is held in an atomic pointer, so installing and reading it is thread safe. The logger itself must stay alive while it is installed. In `main`, `defer log.clearDefault()` right after `setDefault` covers it.
 
If you prefer no global at all, ignore the module-level functions and pass `*Logger` or a bound handle explicitly. Nothing else needs the global.
 
## Using it with the cli module
 
The two modules are independent. The app connects them in a small file, `src/logging.zig`, which defines the flags and applies them:
 
```zig
pub const flags: []const cli.Flag = &.{
    .{ .name = "verbose", .short = 'v', .kind = .bool, .help = "Same as --log-level=debug" },
    .{ .name = "log-level", .help = "trace, debug, info, warn, error, off", .env = "PCTL_LOG_LEVEL" },
    .{ .name = "log-format", .help = "text, logfmt or json", .env = "PCTL_LOG_FORMAT" },
};
 
pub fn configure(ctx: *cli.Context) error{InvalidUsage}!void { ... }
```
 
The root command lists `logging.flags` as persistent flags, and each command's `run` calls `try logging.configure(ctx)` first. A bad value like `--log-level=loud` becomes a usage error: exit 2 with a hint.
 
Two rules keep this tidy:
 
- **Share one stderr writer.** `main` passes the same `&err.interface` to `cli.execute` and to the logger. If they used separate buffered writers, their output could appear out of order. Because the logger flushes after every record, ordering stays correct.
- **Logs are not results.** Command output goes to `ctx.stdout`. Logging goes to stderr. So `pctl query ... | jq` is unaffected by `-v` or `--log-format json`.
Because `cli` has no pre-run hook yet, every command calls `configure`. A `persistent_pre_run` hook on `Command` would remove that repetition and is a good next change to `cli`.
 
## Routing `std.log` into it
 
Libraries and the standard library log through `std.log`. To send those records through the same logger, set this in your root source file:
 
```zig
pub const std_options: std.Options = .{ .logFn = log.stdLogFn };
```
 
Levels map one to one (`err`, `warn`, `info`, `debug`). The `std.log` scope becomes the scope name, and the default scope prints no scope. The message is formatted into a 1 KiB buffer and cut if longer. Since `std.log` has no structured fields, those records carry only a message.
 
## How it works
 
**Files**
 
| File | Responsibility |
|---|---|
| `root.zig` | Public API and the module-level convenience functions. |
| `logger.zig` | `Logger`, `Bound`, the default-logger pointer, `shouldColor`. Owns the lock and the clock. |
| `encode.zig` | Turns one record into bytes in any of the three formats. Pure: it only touches the writer it is given. |
| `level.zig` | `Level`, names, labels, `parse`. |
| `time.zig` | UTC timestamp formatting with no allocation. |
 
**What happens on a call**
 
1. `emit` compares the level against the atomic threshold and returns if disabled.
2. It takes the logger's mutex (`std.Io.Mutex`). This is why two threads can never mix their records.
3. It reads the clock (`std.Io.Clock.real`, or `now_fn` if you set one) and formats the timestamp.
4. `encode.writeRecord` writes the whole line straight into the writer. There is no intermediate buffer and no allocation, so there is no length limit on a record.
5. It flushes the writer if `flush` is on (the default), then unlocks.
**How fields are encoded without allocating.** Fields are an anonymous struct, so their names and types are known at compile time. `writeFields` walks them with `inline for`, and each value is handled by a `switch` on its type, resolved at compile time. Nothing is boxed or stored.
 
Fixed fields on a bound handle are joined to per-call fields with a small generic type, `Pair(A, B)`, which just holds both. `writeFields` recognizes it and writes `a` then `b`. Chaining `.with` nests pairs, and the order stays first-bound first.
 
**Settings that change at run time.** The level is an atomic value. Format and color are read under the mutex, so `setFormat` and `setColor` lock it too. All three are safe to call while other threads are logging.
 
## Limits and known gaps
 
- **No file output, rotation, or network sinks.** It writes to one writer. For a CLI that is stderr, and a service can redirect that to a file or a collector.
- **One output per logger.** To write text to the terminal and JSON to a file, you would create two loggers.
- **Field values are evaluated by the caller.** A disabled level skips formatting but not the work of building the arguments.
- **Fields only on the structured API.** `std.log` records have a message and a scope but no fields.
- **Records before `setDefault` are dropped.** There is no buffering of early logs.
- **The lock is held while writing.** A slow writer blocks other threads that log. For stderr that is normally fine. If it becomes a problem, put a buffered writer in front.
- **No sampling or rate limiting**, and no compile-time level floor to strip debug calls from release builds.
- **Errors are logged by name.** The error name is printed, with no error return trace.

