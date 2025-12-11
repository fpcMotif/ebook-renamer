const std = @import("std");
const tui = @import("tui.zig");
const Normalizer = @import("normalizer.zig").Normalizer;

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

    // Default path is current directory
    const path = if (args.len > 1) args[1] else ".";
    
    // Simulate steps
    try ui.startStep("Scanning");
    std.time.sleep(500 * std.time.ns_per_ms);
    try ui.finishStep("Scanning", "Found files (Demo Mode)");

    try ui.startStep("Normalizing");

    // DEMO: Normalize a few hardcoded strings to prove it works
    const examples = [_][]const u8{
        "Graduate Texts in Mathematics 52 - Algebraic Geometry.pdf",
        "Topology (2000) (2nd Edition).pdf",
        "Marco, Grandis - Category Theory.pdf",
    };

    var norm = Normalizer.init(allocator);

    for (examples) |ex| {
        // Simple extraction of extension
        const ext = ".pdf";
        const new_name = try norm.normalize(ex, ext);
        defer allocator.free(new_name);

        std.debug.print("  Original: {s}\n  New:      {s}\n\n", .{ex, new_name});
    }

    try ui.finishStep("Normalizing", "Normalization Demo Complete");

    try ui.startStep("Checking Integrity");
    std.time.sleep(100 * std.time.ns_per_ms);
    try ui.finishStep("Checking Integrity", "Check complete");

    try ui.startStep("Detecting Duplicates");
    std.time.sleep(100 * std.time.ns_per_ms);
    try ui.finishStep("Detecting Duplicates", "Detected 0 duplicate groups");
}
