const std = @import("std");

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Setup logging
    const timestamp = std.time.timestamp();
    std.debug.print("[{d}] INFO: Starting ebook renamer\n", .{timestamp});

    const tui = @import("tui.zig");
    var ui = tui.Tui.init();
    try ui.printTitle();

    // Parse command line arguments
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    // Default path is current directory
    const path = if (args.len > 1) args[1] else ".";
    
    // Simulate steps
    try ui.startStep("Scanning");
    std.time.sleep(500 * std.time.ns_per_ms);
    try ui.finishStep("Scanning", "Found 0 files (Placeholder)");

    try ui.startStep("Normalizing");
    std.time.sleep(500 * std.time.ns_per_ms);
    try ui.finishStep("Normalizing", "Normalized 0 files (Placeholder)");

    try ui.startStep("Checking Integrity");
    std.time.sleep(500 * std.time.ns_per_ms);
    try ui.finishStep("Checking Integrity", "Check complete");

    try ui.startStep("Detecting Duplicates");
    std.time.sleep(500 * std.time.ns_per_ms);
    try ui.finishStep("Detecting Duplicates", "Detected 0 duplicate groups");

    std.debug.print("\nZig implementation is currently a placeholder.\n", .{});
}
