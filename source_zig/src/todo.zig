const std = @import("std");
const types = @import("types.zig");
const TodoItem = types.TodoItem;

pub const TodoGenerator = struct {
    allocator: std.mem.Allocator,
    items: std.ArrayList(TodoItem),

    pub fn init(allocator: std.mem.Allocator) TodoGenerator {
        return TodoGenerator{
            .allocator = allocator,
            .items = std.ArrayList(TodoItem).init(allocator),
        };
    }

    pub fn deinit(self: *TodoGenerator) void {
        for (self.items.items) |item| {
            self.allocator.free(item.file);
            self.allocator.free(item.message);
            self.allocator.free(item.category);
        }
        self.items.deinit();
    }

    pub fn add(self: *TodoGenerator, category: []const u8, filename: []const u8, message_fmt: []const u8, args: anytype) !void {
        const file = try self.allocator.dupe(u8, filename);
        const cat = try self.allocator.dupe(u8, category);
        const msg = try std.fmt.allocPrint(self.allocator, message_fmt, args);

        try self.items.append(TodoItem{
            .category = cat,
            .file = file,
            .message = msg,
        });
    }

    pub fn write(self: *TodoGenerator, path: []const u8) !void {
        var file = try std.fs.cwd().createFile(path, .{});
        defer file.close();
        var writer = file.writer();

        const timestamp = std.time.timestamp(); // Using simplified timestamp for now
        // Ideally format it properly YYYY-MM-DD HH:MM:SS

        try writer.print("# 需要检查的任务\n\n更新时间: {d}\n\n", .{timestamp});

        // Group by category
        // Simple implementation: iterate categories
        const categories = [_][]const u8{ "failed_download", "too_small", "corrupted_pdf", "read_error", "invalid_extension" };
        const headers = [_][]const u8{ "🔄 未完成下载文件（.download）", "📁 异常小文件（< 1KB）", "🚨 损坏的PDF文件", "⚠️ 其他文件问题", "⚠️ 其他文件问题" };

        for (categories, 0..) |cat, i| {
             var has_items = false;
             for (self.items.items) |item| {
                 if (std.mem.eql(u8, item.category, cat)) {
                     if (!has_items) {
                         try writer.print("## {s}\n", .{headers[i]});
                         has_items = true;
                     }
                     try writer.print("- [ ] {s}\n", .{item.message});
                 }
             }
             if (has_items) try writer.print("\n", .{});
        }

        try writer.print("---\n*此文件由 ebook renamer 自动生成*\n", .{});
    }
};
