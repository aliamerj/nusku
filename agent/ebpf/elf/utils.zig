const std = @import("std");

pub const Error = error{
    NotElf,
    Unsupported,
    Truncated,
    SectionNotFound,
    SymbolNotFound,
};

/// reads an integer from a specific position in the byte array.
pub fn readInt(comptime T: type, bytes: []const u8, offset: usize) Error!T {
    const end = std.math.add(usize, offset, @sizeOf(T)) catch return error.Truncated;
    if (end > bytes.len) return error.Truncated;

    // The first slice starts at the requested offset;
    // the second limits the slice to exactly the number of bytes needed for T
    return std.mem.readInt(T, bytes[offset..][0..@sizeOf(T)], .little);
}
