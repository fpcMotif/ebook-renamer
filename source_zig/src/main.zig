const std = @import("std");
const tui = @import("tui.zig");
const Normalizer = @import("normalizer.zig").Normalizer;
const Scanner = @import("scanner.zig").Scanner;
const types = @import("types.zig");
const Duplicates = @import("duplicates.zig").Duplicates;
const TodoGenerator = @import("todo.zig").TodoGenerator;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Parse command line arguments
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    var path: []const u8 = ".";
    var dry_run = false;
    var json_mode = false;
    var skip_cloud_hash = false;

    // Basic arg parsing
    var i: usize = 1;
    while (i < args.len) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--dry-run") or std.mem.eql(u8, arg, "-d")) {
            dry_run = true;
        } else if (std.mem.eql(u8, arg, "--json")) {
            json_mode = true;
        } else if (std.mem.eql(u8, arg, "--skip-cloud-hash")) {
            skip_cloud_hash = true;
        } else if (!std.mem.startsWith(u8, arg, "-")) {
            path = arg;
        }
        i += 1;
    }
    
    // Auto-detect cloud storage path
    if (!skip_cloud_hash) {
        if (std.mem.indexOf(u8, path, "Dropbox") != null or
            std.mem.indexOf(u8, path, "Google Drive") != null or
            std.mem.indexOf(u8, path, "OneDrive") != null) {
            skip_cloud_hash = true;
        }
    }

    var ui = tui.Tui.init(json_mode);
    if (!json_mode) try ui.printTitle();

    if (!json_mode) try ui.startStep("Scanning");
    var scanner = Scanner.init(allocator, path, std.math.maxInt(usize));
    defer scanner.deinit();

    scanner.scan() catch |err| {
        if (!json_mode) try ui.printError("Scan failed");
        return err;
    };

    if (!json_mode) {
        var buf: [64]u8 = undefined;
        const msg = try std.fmt.bufPrint(&buf, "Found {d} files", .{scanner.files.items.len});
        try ui.finishStep("Scanning", msg);
    }

    // Normalization
    if (!json_mode) try ui.startStep("Normalizing");
    var norm = Normalizer.init(allocator);
    var todo = TodoGenerator.init(allocator);
    defer todo.deinit();

    for (scanner.files.items) |*file| {
        if (file.is_failed_download) {
             try todo.add("failed_download", file.original_name, "重新下载: {s} (未完成下载)", .{file.original_name});
             continue;
        }
        if (file.is_too_small) {
             try todo.add("too_small", file.original_name, "检查并重新下载: {s} (文件过小，仅 {d} 字节)", .{file.original_name, file.size});
             continue;
        }

        const ext = file.extension;
        if (std.mem.eql(u8, ext, ".pdf") or std.mem.eql(u8, ext, ".epub") or std.mem.eql(u8, ext, ".txt")) {
            const new_name = norm.normalize(file.original_name, ext) catch |err| {
                _ = err;
                continue;
            };

            if (!std.mem.eql(u8, file.original_name, new_name)) {
                file.new_name = new_name;
            } else {
                allocator.free(new_name);
            }
        }
    }
    if (!json_mode) try ui.finishStep("Normalizing", "Complete");

    // Integrity/Duplicate Check
    if (!json_mode) try ui.startStep("Detecting Duplicates");

    // Sort files by size
    std.sort.block(types.FileInfo, scanner.files.items, {}, struct {
        fn less(_: void, a: types.FileInfo, b: types.FileInfo) bool {
            return a.size < b.size;
        }
    }.less);

    var duplicates_found: usize = 0;
    var duplicate_groups = std.ArrayList(types.DuplicateGroup).init(allocator);
    defer {
         for (duplicate_groups.items) |g| {
             allocator.free(g.delete);
         }
         duplicate_groups.deinit();
    }

    if (!skip_cloud_hash) {
        // MD5 Hashing Mode
        var start_idx: usize = 0;
        while (start_idx < scanner.files.items.len) {
            var end_idx = start_idx + 1;
            while (end_idx < scanner.files.items.len and scanner.files.items[end_idx].size == scanner.files.items[start_idx].size) {
                end_idx += 1;
            }

            if (end_idx - start_idx > 1) {
                for (scanner.files.items[start_idx..end_idx]) |*f| {
                    if (f.is_failed_download or f.is_too_small) continue;
                    if (std.mem.eql(u8, f.extension, ".pdf") or std.mem.eql(u8, f.extension, ".epub") or std.mem.eql(u8, f.extension, ".txt")) {
                         f.md5_hash = Duplicates.computeHash(allocator, f.original_path) catch null;
                    }
                }
            }
            start_idx = end_idx;
        }

        var hash_map = std.AutoHashMap([16]u8, std.ArrayList(*types.FileInfo)).init(allocator);
        defer {
            var it = hash_map.valueIterator();
            while (it.next()) |list| {
                list.deinit();
            }
            hash_map.deinit();
        }

        for (scanner.files.items) |*f| {
            if (f.md5_hash) |h| {
                const res = try hash_map.getOrPut(h);
                if (!res.found_existing) {
                    res.value_ptr.* = std.ArrayList(*types.FileInfo).init(allocator);
                }
                try res.value_ptr.append(f);
            }
        }

        var it = hash_map.iterator();
        while (it.next()) |entry| {
            const list = entry.value_ptr.*;
            if (list.items.len > 1) {
                duplicates_found += 1;
                std.sort.block(*types.FileInfo, list.items, {}, struct {
                    fn less(_: void, a: *types.FileInfo, b: *types.FileInfo) bool {
                        const a_norm = if (a.new_name) |_| true else false;
                        const b_norm = if (b.new_name) |_| true else false;
                        if (a_norm != b_norm) return a_norm;

                        const a_len = a.original_path.len;
                        const b_len = b.original_path.len;
                        if (a_len != b_len) return a_len < b_len;

                        return a.modified_time > b.modified_time;
                    }
                }.less);

                const keep = list.items[0];
                var deletes = try allocator.alloc([]const u8, list.items.len - 1);
                for (list.items[1..], 0..) |del, idx| {
                    deletes[idx] = del.original_path;
                }

                try duplicate_groups.append(types.DuplicateGroup{
                    .keep = keep.original_path,
                    .delete = deletes,
                });
            }
        }

    } else {
        // Cloud Mode (Fuzzy Match + Size)
        var start_idx: usize = 0;
        while (start_idx < scanner.files.items.len) {
            var end_idx = start_idx + 1;
            while (end_idx < scanner.files.items.len and scanner.files.items[end_idx].size == scanner.files.items[start_idx].size) {
                end_idx += 1;
            }

            if (end_idx - start_idx > 1) {
                const group = scanner.files.items[start_idx..end_idx];

                std.sort.block(types.FileInfo, group, {}, struct {
                    fn less(_: void, a: types.FileInfo, b: types.FileInfo) bool {
                        const a_norm = if (a.new_name) |_| true else false;
                        const b_norm = if (b.new_name) |_| true else false;
                        if (a_norm != b_norm) return a_norm;

                        if (a.original_path.len != b.original_path.len) return a.original_path.len < b.original_path.len;
                        return a.modified_time > b.modified_time;
                    }
                }.less);

                var assigned = try allocator.alloc(bool, group.len);
                defer allocator.free(assigned);
                @memset(assigned, false);

                for (0..group.len) |i| {
                    if (assigned[i]) continue;

                    var current_deletes = std.ArrayList([]const u8).init(allocator);
                    const candidate = &group[i];
                    if (candidate.is_failed_download or candidate.is_too_small) continue;
                    if (!std.mem.eql(u8, candidate.extension, ".pdf") and !std.mem.eql(u8, candidate.extension, ".epub") and !std.mem.eql(u8, candidate.extension, ".txt")) continue;

                    for (i+1..group.len) |j| {
                        if (assigned[j]) continue;
                        const other = &group[j];

                        if (other.is_failed_download or other.is_too_small) continue;
                        if (!std.mem.eql(u8, candidate.extension, other.extension)) continue;

                        const name1 = if (candidate.new_name) |n| n else candidate.original_name;
                        const name2 = if (other.new_name) |n| n else other.original_name;

                        if (Duplicates.isSimilar(name1, name2)) {
                            assigned[j] = true;
                            try current_deletes.append(other.original_path);
                        }
                    }

                    if (current_deletes.items.len > 0) {
                        duplicates_found += 1;
                        assigned[i] = true;
                        try duplicate_groups.append(types.DuplicateGroup{
                            .keep = candidate.original_path,
                            .delete = try current_deletes.toOwnedSlice(),
                        });
                    } else {
                        current_deletes.deinit();
                    }
                }
            }
            start_idx = end_idx;
        }
    }

    if (!json_mode) {
        var buf: [64]u8 = undefined;
        const msg = try std.fmt.bufPrint(&buf, "Detected {d} duplicate groups", .{duplicates_found});
        try ui.finishStep("Detecting Duplicates", msg);
    }

    var renames = std.ArrayList(types.RenameOperation).init(allocator);
    defer {
        for (renames.items) |r| {
            allocator.free(r.to);
        }
        renames.deinit();
    }

    // Execute Renames
    if (!json_mode) try ui.startStep("Applying Changes");
    for (scanner.files.items) |f| {
        if (f.new_name) |new_n| {
            var is_deleted = false;
            for (duplicate_groups.items) |g| {
                for (g.delete) |d| {
                    if (std.mem.eql(u8, d, f.original_path)) {
                        is_deleted = true;
                        break;
                    }
                }
                if (is_deleted) break;
            }
            if (is_deleted) continue;

            const dir = std.fs.path.dirname(f.original_path) orelse ".";
            const new_path = try std.fs.path.join(allocator, &[_][]const u8{ dir, new_n });

            // Check if destination exists
            var dest_exists = false;
            if (!dry_run) {
                 std.fs.cwd().access(new_path, .{}) catch |err| {
                     if (err == error.FileNotFound) {
                         dest_exists = false;
                     } else {
                         dest_exists = true; // assume exists on other errors to be safe
                     }
                 };
                 // But wait, access returns void if exists.
                 // Correct usage:
                 // if (std.fs.cwd().access(new_path, .{})) |_| { dest_exists = true; } else |_| { dest_exists = false; }
                 // Since I'm in !void block I need to handle error.
            }

            // Checking logic correction:
            if (!dry_run) {
                const check_result = std.fs.cwd().access(new_path, .{});
                if (check_result) |_| {
                    dest_exists = true;
                } else |err| {
                    if (err == error.FileNotFound) {
                        dest_exists = false;
                    } else {
                        dest_exists = true; // Safety
                    }
                }
            }

            if (dest_exists) {
                if (!json_mode) {
                     var buf: [256]u8 = undefined;
                     const msg = try std.fmt.bufPrint(&buf, "Skipping rename {s} -> {s} (Target exists)", .{f.original_name, new_n});
                     try ui.printError(msg);
                }
                allocator.free(new_path);
                continue;
            }

            if (!dry_run) {
                // Perform rename
                std.fs.cwd().rename(f.original_path, new_path) catch |err| {
                    if (!json_mode) try ui.printError("Rename failed");
                    allocator.free(new_path);
                    _ = err;
                    continue; // Skip adding to renames list
                };
            }

            try renames.append(types.RenameOperation{
                .from = f.original_path,
                .to = new_path,
                .reason = "normalized",
            });
        }
    }

    // Execute Deletes
    for (duplicate_groups.items) |g| {
        for (g.delete) |d| {
            if (!dry_run) {
                std.fs.cwd().deleteFile(d) catch {};
            }
        }
    }

    if (!json_mode) try ui.finishStep("Applying Changes", "Done");

    const todo_path = try std.fs.path.join(allocator, &[_][]const u8{ path, "todo.md" });
    defer allocator.free(todo_path);
    try todo.write(todo_path);

    if (json_mode) {
        const Output = struct {
            renames: []types.RenameOperation,
            duplicate_deletes: []types.DuplicateGroup,
            small_or_corrupted_deletes: []types.SmallOrCorrupted = &.{},
            todo_items: []types.TodoItem,
        };

        const out = Output{
            .renames = renames.items,
            .duplicate_deletes = duplicate_groups.items,
            .todo_items = todo.items.items,
        };

        const stdout = std.io.getStdOut().writer();
        try std.json.stringify(out, .{ .whitespace = .indent_2 }, stdout);
        try stdout.print("\n", .{});
    }
}
