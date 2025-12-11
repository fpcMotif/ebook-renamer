const std = @import("std");
const Allocator = std.mem.Allocator;

pub const ParsedMetadata = struct {
    authors: ?[]const u8,
    title: []const u8,
    year: ?u16,
    series: ?[]const u8,
    edition: ?[]const u8,
    volume: ?[]const u8,
};

pub fn normalizeFilename(allocator: Allocator, filename: []const u8, extension: []const u8) !ParsedMetadata {
    // Step 1: Remove extension
    var base = filename;
    if (std.mem.endsWith(u8, base, extension)) {
        base = base[0..base.len - extension.len];
    }
    if (std.mem.endsWith(u8, base, ".download")) {
        base = base[0..base.len - ".download".len];
    }
    base = std.mem.trim(u8, base, " \t\n\r");

    // Step 2: Extract series information (simplified for now)
    const series_info = extractSeriesInfo(allocator, base);
    var base_after_series = base;
    if (series_info) |_| {
        // Remove series patterns (simplified)
        base_after_series = removeSeriesPatterns(allocator, base) catch base;
    }

    // Step 3: Remove bracketed annotations
    var cleaned = try removeBrackets(allocator, base_after_series);

    // Step 4: Clean noise sources
    cleaned = try cleanNoiseSources(allocator, cleaned);

    // Step 5: Extract edition
    const edition_info = extractEdition(allocator, cleaned);
    var base_after_edition = cleaned;
    if (edition_info) |_| {
        base_after_edition = try removeEditionPatterns(allocator, cleaned);
    }

    // Step 6: Extract year
    const year = extractYear(base_after_edition);

    // Step 7: Remove parentheticals
    var base_after_parens = try cleanParentheticals(allocator, base_after_edition, year);

    // Step 8: Extract volume
    const volume_info = extractVolume(allocator, base_after_parens);
    var base_after_volume = base_after_parens;
    if (volume_info) |_| {
        base_after_volume = try normalizeVolumePatterns(allocator, base_after_parens);
    }

    // Step 9: Parse author and title (simplified)
    const authors = extractAuthor(allocator, base_after_volume);
    const title = try extractTitle(allocator, base_after_volume, authors);

    return ParsedMetadata{
        .authors = authors,
        .title = title,
        .year = year,
        .series = series_info,
        .edition = edition_info,
        .volume = volume_info,
    };
}

fn extractSeriesInfo(allocator: Allocator, s: []const u8) ?[]const u8 {
    _ = allocator;
    // Simplified: check for common series patterns
    const patterns = [_]struct { full: []const u8, abbr: []const u8 }{
        .{ .full = "Graduate Texts in Mathematics", .abbr = "GTM" },
        .{ .full = "Cambridge Studies in Advanced Mathematics", .abbr = "CSAM" },
    };

    for (patterns) |pattern| {
        if (std.mem.indexOf(u8, s, pattern.full)) |idx| {
            // Look for volume number after series name
            var rest = s[idx + pattern.full.len..];
            rest = std.mem.trimLeft(u8, rest, " \t-");
            var i: usize = 0;
            while (i < rest.len and std.ascii.isDigit(rest[i])) : (i += 1) {}
            if (i > 0) {
                const vol = rest[0..i];
                // Return formatted series info (simplified - would need proper allocation)
                return null; // Placeholder
            }
        }
    }
    return null;
}

fn removeSeriesPatterns(allocator: Allocator, s: []const u8) ![]const u8 {
    _ = allocator;
    _ = s;
    // Placeholder - would need regex or manual pattern matching
    return s;
}

fn removeBrackets(allocator: Allocator, s: []const u8) ![]const u8 {
    var result = std.ArrayList(u8).init(allocator);
    defer result.deinit();
    
    var in_bracket = false;
    for (s) |c| {
        if (c == '[') {
            in_bracket = true;
        } else if (c == ']') {
            in_bracket = false;
        } else if (!in_bracket) {
            try result.append(c);
        }
    }
    
    return result.toOwnedSlice();
}

fn cleanNoiseSources(allocator: Allocator, s: []const u8) ![]const u8 {
    var result = std.ArrayList(u8).init(allocator);
    defer result.deinit();
    
    // Simple pattern removal for common noise sources
    var i: usize = 0;
    while (i < s.len) {
        // Check for "Z-Library", "libgen", "Anna's Archive"
        if (std.mem.indexOf(u8, s[i..], "Z-Library")) |idx| {
            i += idx + "Z-Library".len;
            continue;
        }
        if (std.mem.indexOf(u8, s[i..], "libgen")) |idx| {
            i += idx + "libgen".len;
            continue;
        }
        if (std.mem.indexOf(u8, s[i..], "Anna's Archive")) |idx| {
            i += idx + "Anna's Archive".len;
            continue;
        }
        try result.append(s[i]);
        i += 1;
    }
    
    return result.toOwnedSlice();
}

fn extractEdition(allocator: Allocator, s: []const u8) ?[]const u8 {
    _ = allocator;
    // Look for patterns like "2nd Edition", "3rd ed", etc.
    if (std.mem.indexOf(u8, s, "2nd Edition")) |_| {
        return "2nd ed";
    }
    if (std.mem.indexOf(u8, s, "3rd ed")) |_| {
        return "3rd ed";
    }
    return null;
}

fn removeEditionPatterns(allocator: Allocator, s: []const u8) ![]const u8 {
    _ = allocator;
    _ = s;
    // Placeholder
    return s;
}

fn extractYear(s: []const u8) ?u16 {
    // Find last occurrence of 19xx or 20xx
    var last_year: ?u16 = null;
    var i: usize = 0;
    while (i < s.len - 3) {
        if (s[i] == '1' and s[i + 1] == '9' and std.ascii.isDigit(s[i + 2]) and std.ascii.isDigit(s[i + 3])) {
            const year_str = s[i..i + 4];
            if (std.fmt.parseInt(u16, year_str, 10)) |year| {
                last_year = year;
            } else |_| {}
        } else if (s[i] == '2' and s[i + 1] == '0' and std.ascii.isDigit(s[i + 2]) and std.ascii.isDigit(s[i + 3])) {
            const year_str = s[i..i + 4];
            if (std.fmt.parseInt(u16, year_str, 10)) |year| {
                last_year = year;
            } else |_| {}
        }
        i += 1;
    }
    return last_year;
}

fn cleanParentheticals(allocator: Allocator, s: []const u8, year: ?u16) ![]const u8 {
    _ = allocator;
    _ = year;
    // Simplified - would need proper regex or manual parsing
    return s;
}

fn extractVolume(allocator: Allocator, s: []const u8) ?[]const u8 {
    _ = allocator;
    // Look for "Vol 2", "Volume 2", "Part 2"
    if (std.mem.indexOf(u8, s, "Vol ")) |idx| {
        // Extract number after "Vol "
        var i = idx + 4;
        var start = i;
        while (i < s.len and std.ascii.isDigit(s[i])) : (i += 1) {}
        if (i > start) {
            return "Vol"; // Placeholder
        }
    }
    return null;
}

fn normalizeVolumePatterns(allocator: Allocator, s: []const u8) ![]const u8 {
    _ = allocator;
    _ = s;
    // Placeholder
    return s;
}

fn extractAuthor(allocator: Allocator, s: []const u8) ?[]const u8 {
    _ = allocator;
    // Simplified: look for "Author - Title" pattern
    if (std.mem.indexOf(u8, s, " - ")) |idx| {
        const author_part = std.mem.trim(u8, s[0..idx], " \t");
        if (author_part.len >= 2) {
            return author_part;
        }
    }
    return null;
}

fn extractTitle(allocator: Allocator, s: []const u8, authors: ?[]const u8) ![]const u8 {
    var result = std.ArrayList(u8).init(allocator);
    defer result.deinit();
    
    if (authors) |_| {
        // If author found, title is after " - "
        if (std.mem.indexOf(u8, s, " - ")) |idx| {
            const title_part = std.mem.trim(u8, s[idx + 3..], " \t");
            try result.appendSlice(title_part);
        } else {
            try result.appendSlice(s);
        }
    } else {
        // No author, entire string is title
        try result.appendSlice(std.mem.trim(u8, s, " \t"));
    }
    
    return result.toOwnedSlice();
}

pub fn generateNewFilename(allocator: Allocator, metadata: ParsedMetadata, extension: []const u8) ![]const u8 {
    var result = std.ArrayList(u8).init(allocator);
    defer result.deinit();
    
    // Author(s)
    if (metadata.authors) |authors| {
        try result.appendSlice(authors);
        try result.appendSlice(" - ");
    }
    
    // Title
    try result.appendSlice(metadata.title);
    
    // Series info in brackets
    if (metadata.series) |series| {
        try result.writer().print(" [{}]", .{series});
    }
    
    // Year and Edition in parentheses
    if (metadata.year) |year| {
        if (metadata.edition) |edition| {
            try result.writer().print(" ({}, {})", .{ year, edition });
        } else {
            try result.writer().print(" ({})", .{year});
        }
    } else if (metadata.edition) |edition| {
        try result.writer().print(" ({})", .{edition});
    }
    
    // Extension
    try result.appendSlice(extension);
    
    return result.toOwnedSlice();
}
