const std = @import("std");

pub const Tui = struct {
    writer: std.fs.File.Writer,

    pub fn init() Tui {
        return Tui{
            writer: std.io.getStdOut().writer(),
        };
    }

    pub fn printTitle(self: *Tui) !void {
        try self.writer.print("\x1b[1;32mEbook Renamer (Zig)\x1b[0m\n\n", .{});
    }

    pub fn startStep(self: *Tui, name: []const u8) !void {
        try self.writer.print("\x1b[36m⠋\x1b[0m {s}...", .{name});
    }

    pub fn finishStep(self: *Tui, name: []const u8, info: []const u8) !void {
        try self.writer.print("\r\x1b[K\x1b[32m✓\x1b[0m {s}: {s}\n", .{name, info});
    }
    
    pub fn printError(self: *Tui, msg: []const u8) !void {
        try self.writer.print("\x1b[31mError: {s}\x1b[0m\n", .{msg});
    }
};
