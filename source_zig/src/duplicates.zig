const std = @import("std");
const types = @import("types.zig");
const FileInfo = types.FileInfo;
const Md5 = std.crypto.hash.Md5;

pub const Duplicates = struct {

    pub fn computeHash(allocator: std.mem.Allocator, file_path: []const u8) ![16]u8 {
        var file = try std.fs.cwd().openFile(file_path, .{});
        defer file.close();

        var md5 = Md5.init({});
        var buf: [8192]u8 = undefined;

        while (true) {
            const n = try file.read(&buf);
            if (n == 0) break;
            md5.update(buf[0..n]);
        }

        var out: [16]u8 = undefined;
        md5.final(&out);
        return out;
    }

    pub fn isSimilar(a: []const u8, b: []const u8) bool {
        // Simple Levenshtein distance
        // We only need to know if distance is small enough.
        // Threshold: 0.85 similarity => distance <= 0.15 * max_len
        const max_len = @max(a.len, b.len);
        if (max_len == 0) return true;

        const threshold = (max_len * 15 + 99) / 100; // ceil(max_len * 0.15)

        // Optimization: if length difference is too big, return false
        const len_diff = if (a.len > b.len) a.len - b.len else b.len - a.len;
        if (len_diff > threshold) return false;

        // Implement Levenshtein
        // Using two rows
        // We need an allocator? Or use stack buffer if small enough?
        // Filenames can be 255 chars. Stack is fine.
        if (b.len > 255) return false; // Too long for stack buffer approach safely/simply

        var v0: [256]usize = undefined;
        var v1: [256]usize = undefined;

        for (0..b.len + 1) |i| {
            v0[i] = i;
        }

        for (0..a.len) |i| {
            v1[0] = i + 1;
            for (0..b.len) |j| {
                const cost: usize = if (a[i] == b[j]) 0 else 1;
                v1[j + 1] = @min(@min(v1[j] + 1, v0[j + 1] + 1), v0[j] + cost);
            }
            for (0..b.len + 1) |j| {
                v0[j] = v1[j];
            }
        }

        const dist = v0[b.len];
        return dist <= threshold;
    }
};
