const std = @import("std");

pub const FileInfo = struct {
    original_path: []const u8,
    original_name: []const u8,
    extension: []const u8,
    size: u64,
    modified_time: i128, // Nanoseconds since epoch
    is_failed_download: bool,
    is_too_small: bool,
    new_name: ?[]const u8 = null,
    new_path: ?[]const u8 = null,
    md5_hash: ?[16]u8 = null,

    pub fn deinit(self: *FileInfo, allocator: std.mem.Allocator) void {
        allocator.free(self.original_path);
        allocator.free(self.original_name);
        // extension is usually a slice of original_name, so we don't free it separately unless we duped it.
        // But scanning logic might dupe it. Let's assume we need to manage memory carefully.
        if (self.new_name) |n| allocator.free(n);
        if (self.new_path) |p| allocator.free(p);
    }
};

pub const RenameOperation = struct {
    from: []const u8,
    to: []const u8,
    reason: []const u8,
};

pub const DuplicateGroup = struct {
    keep: []const u8,
    delete: [][]const u8,
};

pub const SmallOrCorrupted = struct {
    path: []const u8,
    issue: []const u8,
};

pub const TodoItem = struct {
    category: []const u8,
    file: []const u8,
    message: []const u8,
};
