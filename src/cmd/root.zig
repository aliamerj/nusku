const cli = @import("cli");

pub const root: cli.Command = .{
    .name = "nusku",
    .short = "A continuous profiling system for Linux, built in Zig, with eBPF doing the actual watching",
    .long = "Nusku is a continuous profiling system for Linux. It watches a running process and shows you what it's actually doing, live, instead of leaving you to guess from a graph of CPU percentage that goes up and down with no explanation",
};
