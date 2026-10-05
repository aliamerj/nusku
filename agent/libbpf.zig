const std = @import("std");
const c = @cImport({
    @cInclude("bpf/libbpf.h");
});

pub const Libbpf = struct {
    pub fn version() []const u8 {
        return std.mem.span(c.libbpf_version_string());
    }
};

test "libbpf is linked and reports its version" {
    try std.testing.expectEqualStrings("v1.5", Libbpf.version());
}

test "libelf and zlib are linked: libbpf rejects a non-ELF buffer" {
    _ = c.libbpf_set_print(null); // silence the expected warning
    const junk = "definitely not an ELF file".*;
    const obj = c.bpf_object__open_mem(&junk, junk.len, null);
    try std.testing.expect(obj == null);
}
