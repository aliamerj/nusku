---
date: 2026-10-04
---

# Ep1: Setting Up The Project

This is first task, we should do is building the infrastructure for the tool and the most important one is Cli

# The cli module
A small command-line framework for Zig 0.16, modeled on Go's cobra. You describe your commands as a tree of constant structs. The module parses argv, picks the right command, fills in flag values, prints help, and turns errors into the right stream and exit code.

## What it does
 
For an app with many commands and subcommands, it handles the parts that are the same every time:
 
- Resolving `nusku agent start --listen :7000` to the `start` command under `agent`.
- Parsing flags: `--flag value`, `--flag=value`, `-f value`, `-fvalue`, bool clusters like `-abc`, `--no-flag`, and `--` to end flag parsing.
- Flag values from three places, in this order: command line, environment variable, default.
- Checking types (int, bool), required flags, and the number of positional arguments, before your code runs.
- Generating `--help` for every command from the tree.
- Keeping Unix stream rules: results on stdout, errors and progress on stderr, and meaningful exit codes.
Your command code only has to do its job and return.


## Quick start
 
`build.zig`: register the module and give it to the app.
 
```zig
const cli_mod = b.addModule("cli", .{
    .root_source_file = b.path("cli/root.zig"),
    .target = target,
    .optimize = optimize,
});

// Nusku commands
const commands_mod = b.addModule("commands", .{
    .root_source_file = b.path("src/cmd/root.zig"),
    .target = target,
    .optimize = optimize,
    .imports = &.{
        .{
            .name = "cli",
            .module = cli_mod,
        },
    },
});

const exe = b.addExecutable(.{
    .name = "nusku",
    .root_module = b.createModule(.{
    .root_source_file = b.path("src/main.zig"),
    .target = target,
    .optimize = optimize,
    .imports = &.{
        .{ .name = "commands", .module = commands_mod },
        .{ .name = "cli", .module = cli_mod },
        },
    }),
});
```

## File map
 
| File | Responsibility |
|---|---|
| `root.zig` | Public API. Re-exports the types and functions you use, and pulls in every file's tests. |
| `command.zig` | Plain data types: `Command`, `Flag`, `ArgsRule`, `FlagValues`, `Context`. No parsing logic. |
| `parser.zig` | Turns `argv` into a `Parsed` result or a parse error with a message. Never prints. |
| `help.zig` | Renders help text and the usage hint. Pure functions over the tree. |
| `errors.zig` | The two special error names and the exit code constants. |
| `execute.zig` | The entry point. Runs the parser, dispatches to `run`, maps outcomes to streams and exit codes. |

## The data model
 
Everything in `command.zig` is plain data. Commands are `const` values linked with pointers, so a whole tree can sit in static memory with no allocation.
 
### `Command`
 
| Field | Meaning |
|---|---|
| `name` | The word the user types. |
| `aliases` | Other words that select the same command (`ls` for `list`). |
| `short` | One line, shown in the parent's command list. |
| `long` | Longer text shown at the top of this command's own help. Falls back to `short`. |
| `usage_args` | Positional synopsis for the usage line, like `<file>...`. Display only. |
| `flags` | Local flags. Visible only on this command. |
| `persistent_flags` | Visible on this command and all its descendants. |
| `subcommands` | `[]const *const Command`. |
| `args` | Rule for the number of positional arguments (see below). |
| `hidden` | Left out of the parent's command list. Still runnable. |
| `run` | `?RunFn`. Null makes it a grouping command that prints help when invoked. |
 
### `Flag`
 
| Field | Meaning |
|---|---|
| `name` | Long name without dashes. `verbose` is `--verbose`. |
| `short` | Optional one-character alias, like `'v'`. |
| `kind` | `.bool`, `.string`, or `.int`. Default `.string`. |
| `help` | One line for the help output. |
| `default` | Text default. If null: `false`, empty string, or `0` depending on `kind`. |
| `env` | Environment variable to read when the flag is not on the command line. |
| `required` | Usage error if neither the command line nor the env supplies it. |
 
Defaults are written as text (`"50"`, `":7000"`) because they go through the same validation as user input.
 
### `ArgsRule`
 
Controls how many positional arguments are allowed.
 
```zig
.args = .any                                  // no check (default)
.args = .none                                 // exactly zero
.args = .{ .exact = 1 }
.args = .{ .min = 1 }
.args = .{ .max = 2 }
.args = .{ .range = .{ .min = 1, .max = 3 } }
```
 
### `FlagValues`
 
The result of flag resolution. It maps each flag name to an `Entry`:
 
```zig
Entry = struct { value: []const u8, source: ValueSource }   // source: .default, .env, .cli
```
 
Every flag visible to the chosen command gets an entry, with its default filled in. That is why the getters never return null:
 
```zig
ctx.getBool("verbose")
ctx.getString("format")
ctx.getInt("limit")
ctx.isSet("limit")      // true if the user supplied it by CLI or env, false for the default
```
 
Asking for a flag name that is not defined for the command is a programmer error and panics with a clear message.
 
### `Context`
 
What `run` receives.
 
| Field | Meaning |
|---|---|
| `allocator` | An arena that lives for this one invocation. Do not free what you allocate from it. |
| `prog` | Name of the root command. |
| `stdout` | `*std.Io.Writer`. Results only. |
| `stderr` | `*std.Io.Writer`. Diagnostics, progress, warnings. |
| `command` | The command being run. |
| `args` | Positional arguments, with command names and flags already removed. |
| `flags` | The resolved `FlagValues`. |
 
Plus the helper methods `getBool`, `getString`, `getInt`, `isSet`, `fail`, and `usageFail`.
 
## How parsing works
 
`parser.parse(arena, root, argv, opts, &diag)` makes one pass over the tokens, then finishes with a short validation step.
 
### The pass
 
The parser keeps a stack called `path`, which starts as `[root]`. The last item is the current command. For each token:
 
```
token
 |
 |- after "--"  ........................ positional
 |- does not start with "-" (or is just "-") ...
 |      |- no positional seen yet and current command has a subcommand with this name
 |      |        -> push that subcommand onto path
 |      |- no positional yet, current command has subcommands but no `run`
 |      |        -> error UnknownCommand
 |      '- otherwise -> positional
 |- exactly "--"  ...................... switch to "everything is positional"
 |- starts with "--"  .................. long flag
 '- starts with "-"  ................... short flag(s)
```
 
The key point is that flags are looked up against whichever command is current at that moment. This gives cobra's behavior:
 
```
nusku --verbose query -n5 x      ok: verbose is persistent on the root
nusku query -v -n5 x             ok: same flag, after the subcommand
nusku --limit 5 query x          error: limit belongs to query, which is not current yet
```
 
### Which flags are visible
 
When looking up a flag name, the parser checks:
 
1. Local `flags` of the current command.
2. `persistent_flags` of the current command and each ancestor, walking up to the root.
Local flags of ancestors are not visible. If a local flag and a persistent flag share a name, the local one wins on lookup.
 
### Long flags
 
`--name=value`, `--name value`, `--name` (bool). For a bool flag, `--name value` does not take `value`; the bool is set and `value` is parsed as the next token. `--no-name` sets a bool flag to false. `--help` is recognized if you did not define your own `help` flag.
 
### Short flags
 
The token body is scanned left to right:
 
- A bool flag is set and the scan continues, so `-abc` sets three flags.
- A flag that takes a value ends the cluster. The value is the rest of the token (`-n5`), the part after `=` (`-n=5`), or the next token (`-n 5`).
- `h` is treated as help if no flag claimed it.
When a flag takes a value from the next token, that token is used as is, even if it starts with `-`. So `--limit -1` gives the value `-1`.
 
### Validation at set time
 
Each value is checked when it is stored. Ints must parse with `std.fmt.parseInt(i64, v, 0)`, so `0x10` and `1_000` style input follows Zig's rules. Bools must be exactly `true` or `false`. A bad value is a parse error, so your `run` never sees it.
 
### After the pass
 
If help was requested, the parser returns right away with `help_requested = true` and skips everything below. This is why `nusku query --help` works even though `--service` is required.
 
Otherwise, for every flag visible to the final command that was not set on the command line:
 
1. If the flag has `env` and the lookup returns a value, use it (source `.env`).
2. Else if the flag is `required`, fail with `MissingRequiredFlag`.
3. Else store the default (source `.default`).
Finally the positional count is checked against the command's `ArgsRule`.
 
So the precedence is command line, then environment, then default. A required flag can be satisfied by the command line or the environment, but a default never satisfies it.
 
### Result and errors
 
On success, `Parsed` holds the chosen `command`, the full `path` (root to command), `args`, `flags`, and `help_requested`.
 
On failure, `parse` returns one of these errors and fills a `Diagnostic` with a `message`, the failing `command`, and its `path`:
 
| Error | Example message |
|---|---|
| `UnknownCommand` | `unknown command "lst" for "nusku"` |
| `UnknownFlag` | `unknown flag: --bogus` |
| `MissingValue` | `flag needs a value: --limit` |
| `InvalidValue` | `invalid value "abc" for --limit: expected an integer` |
| `MissingRequiredFlag` | `required flag not set: --service` |
| `InvalidArgCount` | `get: expects exactly 1 argument(s), got 0` |
| `OutOfMemory` | none |
 
The parser never prints. The caller decides how to show the message, which keeps it testable and reusable.
 
### Environment lookup
 
The parser does not read the process environment itself. It takes an `EnvLookup`, a small struct with a context pointer and a function. In a 0.16 `main`, `cli.EnvLookup.fromMap(init.environ_map)` builds one. In tests you pass a fake. Passing `null` turns env fallback off.
 
## How help is rendered
 
`help.zig` renders from the tree and writes to any `*std.Io.Writer`. It does no allocation.
 
Sample output for `nusku query --help`:
 
```
Query stored profiles
 
Usage:
  nusku query [flags] <expression>
 
Aliases:
  query, q
 
Flags:
  -h, --help             Show help for this command
  -s, --service string   Service to query [env: NUSKU_SERVICE] (required)
  -n, --limit int        Max rows (default 5)
  -f, --format string    table or json (default "table")
      --reverse          Oldest first
 
Global Flags:
  -v, --verbose         Print progress to stderr
  -c, --config string   Config file (default "/etc/nusku.toml") [env: NUSKU_CONFIG]
```
 
How it is built:
 
- **Intro**: `long`, or `short` if there is no `long`.
- **Usage**: one line if the command has a `run`, another with `[command]` if it has visible subcommands.
- **Aliases**: shown for non-root commands that have any.
- **Commands**: visible subcommands, names padded to the longest one.
- **Flags**: the command's own `flags` plus its own `persistent_flags`. An automatic `-h, --help` line is added unless you defined a `help` flag.
- **Global Flags**: persistent flags inherited from ancestors.
- **Footer**: a pointer to `nusku [command] --help` when there are subcommands.
Column alignment needs the widest flag label before anything is printed, and there is no allocation to collect the labels into. So each section is walked twice with a small visitor: the first pass measures, the second pass prints with that width. This is why the same traversal function serves both.
 
`writeUsageHint` prints the shorter line used after errors: `Run 'nusku query --help' for usage.`
 
## How `execute` ties it together
 
`cli.execute(&root, argv, opts)` returns the exit code. It never calls exit itself, so tests can call it and check the result.
 
Steps:
 
1. Make an arena from `opts.allocator`. It is freed when `execute` returns.
2. If `opts.version` is set and `argv` is exactly `["--version"]`, print `nusku 0.1.0` to stdout and return 0.
3. Call the parser.
   - On a parse error, write `nusku: <message>` and the usage hint to stderr, and return 2.
   - On out of memory, write a short message and return 1.
4. If help was requested, or the chosen command has no `run`, print its help to stdout and return 0.
5. Build a `Context` and call `run`.
6. Map what `run` returns (next section).
7. Flush both writers.
## Streams and exit codes
 
| Situation | Stream | Exit |
|---|---|---|
| Normal result | stdout | 0 |
| `--help`, `--version`, or a grouping command with nothing to run | stdout | 0 |
| Parse error: unknown command or flag, bad value, missing required flag, wrong argument count | stderr | 2 |
| `ctx.usageFail(...)` from a command | stderr | 2 |
| `ctx.fail(...)` from a command | stderr | 1 |
| Any other error returned from `run` | stderr, as `nusku: error: <ErrorName>` | 1 |
| `error.WriteFailed` from a writer (for example `nusku ... \| head -1`) | nothing | 1 |
 
Two details worth knowing:
 
- **Ordering.** Before writing a diagnostic to stderr, `execute` flushes stdout. On a terminal this keeps the two streams in the order things happened.
- **Broken pipes.** When the reader of a pipe exits early, writes fail with `WriteFailed`. `execute` exits quietly instead of printing an error about it.
### The special error names
 
In `errors.zig`:
 
- `error.Reported`: the message is already printed. Returned by `ctx.fail`.
- `error.InvalidUsage`: bad input. Returned by `ctx.usageFail`. `execute` adds the help hint and exits 2.
Exit codes are constants: `exit_success = 0`, `exit_failure = 1`, `exit_usage = 2`.
 
## Writing a command
 
One file per command keeps a large tree manageable. Each file exports its `Command` as a `pub const`, and the parent points at it.
 
```zig
// src/commands/query.zig
const std = @import("std");
const cli = @import("cli");
 
pub const cmd: cli.Command = .{
    .name = "query",
    .aliases = &.{"q"},
    .short = "Query stored profiles",
    .usage_args = "<expression>",
    .flags = &.{
        .{ .name = "service", .short = 's', .required = true, .env = "NUSKU_SERVICE" },
        .{ .name = "limit", .short = 'n', .kind = .int, .default = "5" },
        .{ .name = "format", .short = 'f', .default = "table" },
    },
    .args = .{ .exact = 1 },
    .run = run,
};
 
fn run(ctx: *cli.Context) anyerror!void {
    const expr = ctx.args[0];
 
    // input the parser cannot judge: exit 2 with a hint
    const format = ctx.getString("format");
    if (!std.mem.eql(u8, format, "table") and !std.mem.eql(u8, format, "json")) {
        return ctx.usageFail("unknown --format \"{s}\"", .{format});
    }
 
    // progress goes to stderr
    if (ctx.getBool("verbose")) try ctx.stderr.print("querying {s}\n", .{expr});
 
    // the result goes to stdout
    try ctx.stdout.print("{s}\n", .{expr});
 
    // an operation that failed: message plus exit 1
    // return ctx.fail("cannot open {s}", .{path});
}
```
 
Parent:
 
```zig
const query = @import("commands/query.zig");
 
const root: cli.Command = .{
    .name = "nusku",
    .subcommands = &.{ &query.cmd },
};
```
 
Rules of thumb:
 
- Never write to the process streams directly. Use `ctx.stdout` and `ctx.stderr`. This is what makes piping correct and what lets tests capture output.
- Use `usageFail` when the user's input is wrong, `fail` when the input was fine and the work failed.
- Put flags that apply to many commands in `persistent_flags` on a common ancestor.
### Grouping commands
 
A command with `subcommands` and no `run` is a group. Invoking it prints its help and exits 0. A command with both `run` and `subcommands` is allowed: an unrecognized first word becomes a positional argument instead of an unknown-command error.
 
## Memory and lifetimes
 
- `execute` creates one arena per call and frees it at the end. `ctx.allocator` is that arena, so commands can allocate freely and never free.
- `parser.parse` allocates from the arena you give it. The returned `Parsed` points into that arena and into `argv`, so both must outlive it. Inside `execute` this is already the case.
- Flag values are slices of the original `argv` strings, not copies.
- The command tree itself allocates nothing. It is `const` data.
## Testing
 
The module has 17 tests across four files. Run them with `zig build test`, or `zig test src/cli/root.zig`.
 
- `parser.zig`: subcommand and alias resolution, flag forms, clusters, negation, `--`, env precedence, every error, required flags, arg count rules, help skipping validation.
- `help.zig`: exact text of root help, subcommand help, and the usage hint.
- `execute.zig`: for each outcome, checks stdout, stderr, and the exit code together.
To test your own commands, call `execute` with in-memory writers:
 
```zig
var out: std.Io.Writer.Allocating = .init(std.testing.allocator);
defer out.deinit();
var err: std.Io.Writer.Allocating = .init(std.testing.allocator);
defer err.deinit();
 
const code = cli.execute(&root, &.{ "query", "-s", "api", "x" }, .{
    .allocator = std.testing.allocator,
    .stdout = &out.writer,
    .stderr = &err.writer,
});
 
try std.testing.expectEqual(@as(u8, 0), code);
try std.testing.expectEqualStrings("x\n", out.written());
try std.testing.expectEqualStrings("", err.written());
```
 
## Limits and known gaps
 
- **`--version`** only works as the only argument (`nusku --version`). It is not a real flag, so `nusku query --version` is an unknown flag.
- **Tokens like `-5`** are treated as flags. To pass a negative number as a positional, put it after `--`. As the value of a flag it works: `--limit -5`.
- **Shadowed flags.** If a command defines a local flag with the same name as an ancestor's persistent flag, lookup picks the local one, but both would share one key in `FlagValues`. There is no check at tree-build time yet.
- **Flag access is by string.** A misspelled name in `ctx.getInt("limt")` panics at run time. The planned fix is typed flags derived from a struct with comptime reflection, so the compiler catches it.
- **No color handling** yet, because nothing prints color. When something does, honor `NO_COLOR` and check for a terminal.
- **No shell completion** yet. The tree has what a generator needs (names, aliases, flags, kinds).
- **No "did you mean" suggestions** for mistyped commands.
Planned next: typed flags via comptime reflection, then shell completion for bash, zsh, and fish.



`main` returns the exit code as a `u8`. Zig's start code passes it to the OS.

