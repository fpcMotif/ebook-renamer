const std = @import("std");
const types = @import("types.zig");
const FileInfo = types.FileInfo;

pub const Scanner = struct {
    allocator: std.mem.Allocator,
    files: std.ArrayList(FileInfo),
    max_depth: usize,
    target_dir: []const u8,

    pub fn init(allocator: std.mem.Allocator, target_dir: []const u8, max_depth: usize) Scanner {
        return Scanner{
            .allocator = allocator,
            .files = std.ArrayList(FileInfo).init(allocator),
            .max_depth = max_depth,
            .target_dir = target_dir,
        };
    }

    pub fn deinit(self: *Scanner) void {
        for (self.files.items) |*f| {
            f.deinit(self.allocator);
        }
        self.files.deinit();
    }

    pub fn scan(self: *Scanner) !void {
        var dir = try std.fs.cwd().openDir(self.target_dir, .{ .iterate = true });
        defer dir.close();

        try self.walk(dir, self.target_dir, 0);
    }

    fn walk(self: *Scanner, dir: std.fs.Dir, path: []const u8, depth: usize) !void {
        if (depth > self.max_depth) return;

        var it = dir.iterate();
        while (try it.next()) |entry| {
            if (std.mem.startsWith(u8, entry.name, ".")) continue; // Skip hidden
            if (shouldSkipDir(entry.name)) continue;

            const full_path = try std.fs.path.join(self.allocator, &[_][]const u8{ path, entry.name });
            errdefer self.allocator.free(full_path);

            switch (entry.kind) {
                .directory => {
                    var sub_dir = dir.openDir(entry.name, .{ .iterate = true }) catch |err| {
                        // Permission denied or other error, just skip
                         if (err == error.AccessDenied) continue;
                         return err;
                    };
                    defer sub_dir.close();
                    try self.walk(sub_dir, full_path, depth + 1);
                    // full_path is needed for recursion, but after recursion we don't store it unless it's a file
                    // But here we alloc'd it.
                    self.allocator.free(full_path);
                },
                .file => {
                     try self.processFile(full_path, entry.name);
                },
                else => {
                    self.allocator.free(full_path);
                }
            }
        }
    }

    fn shouldSkipDir(name: []const u8) bool {
        const skips = [_][]const u8{ "Xcode", "node_modules", ".git", "__pycache__" };
        for (skips) |s| {
            if (std.mem.eql(u8, name, s)) return true;
        }
        return false;
    }

    fn processFile(self: *Scanner, full_path: []const u8, name: []const u8) !void {
        // Classify
        const extension = std.fs.path.extension(name);
        var is_failed = false;
        var is_too_small = false;
        var size: u64 = 0;
        var mtime: i128 = 0;

        // Stat file
        const stat = std.fs.cwd().statFile(full_path) catch {
            // If can't stat, skip
            self.allocator.free(full_path);
            return;
        };
        size = stat.size;
        mtime = stat.mtime;

        if (std.mem.endsWith(u8, name, ".download") or std.mem.endsWith(u8, name, ".crdownload")) {
            is_failed = true;
        } else if ((std.mem.eql(u8, extension, ".pdf") or std.mem.eql(u8, extension, ".epub")) and size < 1024) {
            is_too_small = true;
        }

        const name_dupe = try self.allocator.dupe(u8, name);
        // extension points into name usually, but since we dupe name, we can point to that.
        // Actually std.fs.path.extension returns a slice of the input.
        // So we need to re-calculate extension on the duped name.
        const ext_dupe = std.fs.path.extension(name_dupe);

        const info = FileInfo{
            .original_path = full_path, // already allocated
            .original_name = name_dupe,
            .extension = ext_dupe,
            .size = size,
            .modified_time = mtime,
            .is_failed_download = is_failed,
            .is_too_small = is_too_small,
        };

        try self.files.append(info);
    }
};
