const std = @import("std");
const normalizer = @import("normalizer.zig");
const tui = @import("tui.zig");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Setup logging
    const timestamp = std.time.timestamp();
    std.debug.print("[{d}] INFO: Starting ebook renamer\n", .{timestamp});

    var ui = tui.Tui.init();
    try ui.printTitle();

    // Parse command line arguments
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    // Check for flags
    var path: []const u8 = ".";
    var dry_run = false;
    var json_output = false;

    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--dry-run") or std.mem.eql(u8, arg, "-d")) {
            dry_run = true;
        } else if (std.mem.eql(u8, arg, "--json")) {
            json_output = true;
        } else if (!std.mem.startsWith(u8, arg, "-")) {
            path = arg;
        }
    }

    // Scan for files
    try ui.startStep("Scanning");
    var files = std.ArrayList(normalizer.FileInfo).init(allocator);
    defer {
        for (files.items) |*f| {
            f.deinit(allocator);
        }
        files.deinit();
    }

    var file_count: usize = 0;
    var dir = std.fs.cwd().openDir(path, .{ .iterate = true }) catch |err| {
        try ui.printError("Failed to open directory");
        std.debug.print("Error: {}\n", .{err});
        return;
    };
    defer dir.close();

    var walker = dir.iterate();
    while (try walker.next()) |entry| {
        if (entry.kind != .file) continue;

        // Skip hidden files
        if (entry.name[0] == '.') continue;

        // Get extension
        const ext = std.fs.path.extension(entry.name);
        if (ext.len == 0) continue;

        // Only process ebook files
        const valid_exts = [_][]const u8{ ".pdf", ".epub", ".txt", ".mobi" };
        var is_valid = false;
        for (valid_exts) |valid| {
            if (std.mem.eql(u8, ext, valid)) {
                is_valid = true;
                break;
            }
        }
        if (!is_valid) continue;

        // Get file stat
        const stat = dir.statFile(entry.name) catch continue;

        const full_path = try std.fs.path.join(allocator, &[_][]const u8{ path, entry.name });

        try files.append(normalizer.FileInfo{
            .original_path = full_path,
            .original_name = try allocator.dupe(u8, entry.name),
            .extension = try allocator.dupe(u8, ext),
            .size = stat.size,
            .is_failed_download = std.mem.endsWith(u8, entry.name, ".download") or std.mem.endsWith(u8, entry.name, ".crdownload"),
            .is_too_small = (std.mem.eql(u8, ext, ".pdf") or std.mem.eql(u8, ext, ".epub")) and stat.size < 1024,
            .new_name = null,
            .new_path = null,
        });
        file_count += 1;
    }

    var info_buf: [64]u8 = undefined;
    const scan_info = try std.fmt.bufPrint(&info_buf, "Found {d} files", .{file_count});
    try ui.finishStep("Scanning", scan_info);

    // Normalize files
    try ui.startStep("Normalizing");
    try normalizer.normalizeFiles(allocator, files.items);

    var rename_count: usize = 0;
    for (files.items) |file| {
        if (file.new_name != null) {
            rename_count += 1;
        }
    }

    const norm_info = try std.fmt.bufPrint(&info_buf, "Normalized {d} files", .{rename_count});
    try ui.finishStep("Normalizing", norm_info);

    // Output results
    if (json_output) {
        // JSON output
        const stdout = std.io.getStdOut().writer();
        try stdout.writeAll("{\n  \"renames\": [\n");
        var first = true;
        for (files.items) |file| {
            if (file.new_name) |new_name| {
                if (!std.mem.eql(u8, file.original_name, new_name)) {
                    if (!first) try stdout.writeAll(",\n");
                    first = false;
                    try stdout.print("    {{\"from\": \"{s}\", \"to\": \"{s}\", \"reason\": \"normalized\"}}", .{ file.original_name, new_name });
                }
            }
        }
        try stdout.writeAll("\n  ],\n");
        try stdout.writeAll("  \"duplicate_deletes\": [],\n");
        try stdout.writeAll("  \"small_or_corrupted_deletes\": [],\n");
        try stdout.writeAll("  \"todo_items\": []\n");
        try stdout.writeAll("}\n");
    } else {
        // Human-readable output
        std.debug.print("\n", .{});
        if (dry_run) {
            std.debug.print("Dry run - no changes will be made\n\n", .{});
        }

        for (files.items) |file| {
            if (file.new_name) |new_name| {
                if (!std.mem.eql(u8, file.original_name, new_name)) {
                    std.debug.print("  {s}\n  → {s}\n\n", .{ file.original_name, new_name });
                }
            }
        }

        std.debug.print("Summary: {d} files would be renamed\n", .{rename_count});
    }
}
