const std = @import("std");
const c = @cImport({
    @cInclude("bpf/libbpf.h");
});

pub const Libbpf = struct {
    pub fn version() []const u8 {
        return std.mem.span(c.libbpf_version_string());
    }
};
