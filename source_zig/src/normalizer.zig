const std = @import("std");
const mem = std.mem;
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;

/// Series abbreviation mappings
const SeriesMapping = struct {
    full_name: []const u8,
    abbr: []const u8,
};

const series_mappings = [_]SeriesMapping{
    .{ .full_name = "Graduate Texts in Mathematics", .abbr = "GTM" },
    .{ .full_name = "Cambridge Studies in Advanced Mathematics", .abbr = "CSAM" },
    .{ .full_name = "London Mathematical Society Lecture Note Series", .abbr = "LMSLN" },
    .{ .full_name = "Progress in Mathematics", .abbr = "PM" },
    .{ .full_name = "Springer Undergraduate Mathematics Series", .abbr = "SUMS" },
    .{ .full_name = "Graduate Studies in Mathematics", .abbr = "GSM" },
    .{ .full_name = "AMS Mathematical Surveys and Monographs", .abbr = "AMS-MSM" },
    .{ .full_name = "Oxford Graduate Texts in Mathematics", .abbr = "OGTM" },
    .{ .full_name = "Springer Monographs in Mathematics", .abbr = "SMM" },
};

/// Noise source patterns to remove
const noise_patterns = [_][]const u8{
    "Z-Library",
    "z-Library",
    "Z-library",
    "libgen.li",
    "libgen",
    "Anna's Archive",
    "Annas Archive",
    "AnnaArchive",
};

/// Publisher keywords for detection
const publisher_keywords = [_][]const u8{
    "Press",
    "Publishing",
    "Academic Press",
    "Springer",
    "Cambridge",
    "Oxford",
    "MIT Press",
    "Series",
    "Textbook Series",
    "Graduate Texts",
    "Graduate Studies",
    "Lecture Notes",
    "Pure and Applied",
    "Mathematics",
    "Foundations of",
    "Monographs",
    "Studies",
    "Collection",
    "Textbook",
    "Edition",
    "Vol.",
    "Volume",
    "No.",
    "Part",
    "University",
    "Verlag",
};

/// Strict publisher keywords for suffix stripping
const strict_publisher_keywords = [_][]const u8{
    "Press",
    "Publishing",
    "Springer",
    "Cambridge",
    "Oxford",
    "MIT",
    "Wiley",
    "Elsevier",
    "Routledge",
    "Pearson",
    "McGraw",
    "Addison",
    "Prentice",
    "O'Reilly",
    "Princeton",
    "Harvard",
    "Yale",
    "Stanford",
    "Chicago",
    "California",
    "Columbia",
    "University",
    "Verlag",
    "CUP",
};

/// Non-author keywords
const non_author_keywords = [_][]const u8{
    "auth.",
    "translator",
    "translated by",
    "z-library",
    "libgen",
    "anna's archive",
    "2-library",
};

/// Parsed filename metadata
pub const ParsedMetadata = struct {
    authors: ?[]const u8,
    title: []const u8,
    year: ?u16,
    series: ?[]const u8,
    edition: ?[]const u8,
    volume: ?[]const u8,

    pub fn deinit(self: *ParsedMetadata, allocator: Allocator) void {
        if (self.authors) |a| allocator.free(a);
        allocator.free(self.title);
        if (self.series) |s| allocator.free(s);
        if (self.edition) |e| allocator.free(e);
        if (self.volume) |v| allocator.free(v);
    }
};

/// File info structure
pub const FileInfo = struct {
    original_path: []const u8,
    original_name: []const u8,
    extension: []const u8,
    size: u64,
    is_failed_download: bool,
    is_too_small: bool,
    new_name: ?[]const u8,
    new_path: ?[]const u8,

    pub fn deinit(self: *FileInfo, allocator: Allocator) void {
        allocator.free(self.original_path);
        allocator.free(self.original_name);
        allocator.free(self.extension);
        if (self.new_name) |n| allocator.free(n);
        if (self.new_path) |p| allocator.free(p);
    }
};

/// Normalize a list of files
pub fn normalizeFiles(allocator: Allocator, files: []FileInfo) !void {
    for (files) |*file| {
        if (file.is_failed_download or file.is_too_small) {
            continue;
        }

        var metadata = try parseFilename(allocator, file.original_name, file.extension);
        defer metadata.deinit(allocator);

        const new_name = try generateNewFilename(allocator, &metadata, file.extension);
        file.new_name = new_name;

        // Generate new path
        if (std.fs.path.dirname(file.original_path)) |dir| {
            file.new_path = try std.fs.path.join(allocator, &[_][]const u8{ dir, new_name });
        } else {
            file.new_path = try allocator.dupe(u8, new_name);
        }
    }
}

/// Parse a filename into metadata components
pub fn parseFilename(allocator: Allocator, filename: []const u8, extension: []const u8) !ParsedMetadata {
    // Step 1: Remove extension and .download suffix
    var base = filename;
    if (mem.endsWith(u8, base, ".download")) {
        base = base[0 .. base.len - 9];
    }
    if (mem.endsWith(u8, base, extension)) {
        base = base[0 .. base.len - extension.len];
    }
    base = trim(base);

    // Step 2: Extract series information
    const base_owned = try allocator.dupe(u8, base);
    const series_result = try extractSeriesInfo(allocator, base_owned);
    const series_info = series_result.series;
    
    // Use the remaining string if available, otherwise use base_owned
    const base_after_series = if (series_result.remaining) |r| blk: {
        allocator.free(base_owned);
        break :blk r;
    } else base_owned;
    defer allocator.free(base_after_series);

    // Step 3: Remove bracketed annotations
    const no_brackets = try removeBrackets(allocator, base_after_series);
    defer allocator.free(no_brackets);

    // Step 4: Clean noise sources
    const cleaned = try cleanNoiseSources(allocator, no_brackets);
    defer allocator.free(cleaned);

    // Step 5: Remove duplicate markers
    const no_dups = try removeDuplicateMarkers(allocator, cleaned);
    defer allocator.free(no_dups);

    // Step 6: Extract edition information
    const edition_result = try extractEdition(allocator, no_dups);
    const edition_info = edition_result.edition;
    const after_edition = edition_result.remaining;
    defer if (after_edition.ptr != no_dups.ptr) allocator.free(after_edition);

    // Step 7: Extract year
    const year = extractYear(after_edition);

    // Step 8: Remove parentheticals with year/publisher
    const no_parens = try cleanParentheticals(allocator, after_edition, year);
    defer allocator.free(no_parens);

    // Step 9: Extract volume information
    const volume_result = try extractVolume(allocator, no_parens);
    const volume_info = volume_result.volume;
    const after_volume = volume_result.remaining;
    defer if (after_volume.ptr != no_parens.ptr) allocator.free(after_volume);

    // Step 10: Parse author and title
    const parse_result = try smartParseAuthorTitle(allocator, after_volume);

    return ParsedMetadata{
        .authors = parse_result.authors,
        .title = parse_result.title,
        .year = year,
        .series = series_info,
        .edition = edition_info,
        .volume = volume_info,
    };
}

/// Extract series information from the beginning of the string
fn extractSeriesInfo(allocator: Allocator, s: []const u8) !struct { series: ?[]const u8, remaining: ?[]const u8 } {
    // Pattern 1: "Series Name Volume - Author - Title" (case insensitive)
    for (series_mappings) |mapping| {
        if (startsWithIgnoreCase(s, mapping.full_name)) {
            const after = s[mapping.full_name.len..];
            const trimmed = trimLeft(after);

            // Look for volume number
            if (trimmed.len > 0 and std.ascii.isDigit(trimmed[0])) {
                var vol_end: usize = 0;
                while (vol_end < trimmed.len and std.ascii.isDigit(trimmed[vol_end])) {
                    vol_end += 1;
                }
                if (vol_end > 0) {
                    const vol_num = trimmed[0..vol_end];
                    const series = try std.fmt.allocPrint(allocator, "{s} {s}", .{ mapping.abbr, vol_num });
                    const rest_start = vol_end;
                    var rest = trimmed[rest_start..];
                    // Skip separator
                    rest = trimLeft(rest);
                    if (rest.len > 0 and (rest[0] == '-' or rest[0] == ' ')) {
                        rest = trimLeft(rest[1..]);
                    }
                    const remaining = try allocator.dupe(u8, rest);
                    return .{ .series = series, .remaining = remaining };
                }
            }
        }
    }

    // Pattern 2: "(Series Name Volume) Author - Title"
    if (s.len > 0 and s[0] == '(') {
        if (mem.indexOf(u8, s, ")")) |paren_end| {
            const inside = s[1..paren_end];
            for (series_mappings) |mapping| {
                if (containsIgnoreCase(inside, mapping.full_name)) {
                    // Extract volume number at the end
                    const trimmed_inside = trimRight(inside);
                    var vol_start = trimmed_inside.len;
                    while (vol_start > 0 and std.ascii.isDigit(trimmed_inside[vol_start - 1])) {
                        vol_start -= 1;
                    }
                    if (vol_start < trimmed_inside.len) {
                        const vol_num = trimmed_inside[vol_start..];
                        const series = try std.fmt.allocPrint(allocator, "{s} {s}", .{ mapping.abbr, vol_num });
                        const rest = trim(s[paren_end + 1 ..]);
                        const remaining = try allocator.dupe(u8, rest);
                        return .{ .series = series, .remaining = remaining };
                    }
                }
            }
        }
    }

    return .{ .series = null, .remaining = null };
}

/// Extract edition information
fn extractEdition(allocator: Allocator, s: []const u8) !struct { edition: ?[]const u8, remaining: []const u8 } {
    // Look for patterns like "2nd Edition", "3rd ed", etc.
    const edition_markers = [_][]const u8{ "st Edition", "nd Edition", "rd Edition", "th Edition", "st ed", "nd ed", "rd ed", "th ed" };

    for (edition_markers) |marker| {
        if (mem.indexOf(u8, s, marker)) |idx| {
            if (idx > 0 and std.ascii.isDigit(s[idx - 1])) {
                // Find the start of the number
                var num_start = idx - 1;
                while (num_start > 0 and std.ascii.isDigit(s[num_start - 1])) {
                    num_start -= 1;
                }
                const num_str = s[num_start..idx];
                const suffix = getOrdinalSuffix(num_str);
                const edition = try std.fmt.allocPrint(allocator, "{s}{s} ed", .{ num_str, suffix });

                // Remove the edition part from string
                var result = ArrayList(u8).init(allocator);
                try result.appendSlice(s[0..num_start]);
                try result.appendSlice(s[idx + marker.len ..]);
                const remaining = try result.toOwnedSlice();
                return .{ .edition = edition, .remaining = remaining };
            }
        }
    }

    return .{ .edition = null, .remaining = s };
}

/// Get ordinal suffix for a number
fn getOrdinalSuffix(num_str: []const u8) []const u8 {
    if (mem.eql(u8, num_str, "1")) return "st";
    if (mem.eql(u8, num_str, "2")) return "nd";
    if (mem.eql(u8, num_str, "3")) return "rd";
    return "th";
}

/// Extract volume information
fn extractVolume(allocator: Allocator, s: []const u8) !struct { volume: ?[]const u8, remaining: []const u8 } {
    const volume_markers = [_]struct { pattern: []const u8, normalized: bool }{
        .{ .pattern = "Vol ", .normalized = true },
        .{ .pattern = "Vol. ", .normalized = true },
        .{ .pattern = "Volume ", .normalized = false },
        .{ .pattern = "Part ", .normalized = false },
    };

    for (volume_markers) |marker| {
        if (mem.indexOf(u8, s, marker.pattern)) |idx| {
            const after = s[idx + marker.pattern.len ..];
            var num_end: usize = 0;
            while (num_end < after.len and std.ascii.isDigit(after[num_end])) {
                num_end += 1;
            }
            if (num_end > 0) {
                const vol_num = after[0..num_end];
                const volume = try std.fmt.allocPrint(allocator, "Vol {s}", .{vol_num});

                if (!marker.normalized) {
                    // Replace with normalized form
                    var result = ArrayList(u8).init(allocator);
                    try result.appendSlice(s[0..idx]);
                    try result.appendSlice("Vol ");
                    try result.appendSlice(vol_num);
                    try result.appendSlice(after[num_end..]);
                    const remaining = try result.toOwnedSlice();
                    return .{ .volume = volume, .remaining = remaining };
                }

                return .{ .volume = volume, .remaining = s };
            }
        }
    }

    return .{ .volume = null, .remaining = s };
}

/// Extract the last year (19XX or 20XX) from the string
fn extractYear(s: []const u8) ?u16 {
    var last_year: ?u16 = null;
    var i: usize = 0;

    while (i + 3 < s.len) {
        if ((s[i] == '1' and s[i + 1] == '9') or (s[i] == '2' and s[i + 1] == '0')) {
            if (std.ascii.isDigit(s[i + 2]) and std.ascii.isDigit(s[i + 3])) {
                // Check word boundaries
                const at_start = i == 0 or !std.ascii.isAlphanumeric(s[i - 1]);
                const at_end = i + 4 >= s.len or !std.ascii.isAlphanumeric(s[i + 4]);
                if (at_start and at_end) {
                    if (std.fmt.parseInt(u16, s[i .. i + 4], 10)) |year| {
                        last_year = year;
                    } else |_| {}
                }
            }
        }
        i += 1;
    }

    return last_year;
}

/// Clean parentheticals containing year/publisher info
fn cleanParentheticals(allocator: Allocator, s: []const u8, year: ?u16) ![]u8 {
    var result = ArrayList(u8).init(allocator);
    var i: usize = 0;

    while (i < s.len) {
        if (s[i] == '(') {
            // Find matching closing paren
            if (mem.indexOfPos(u8, s, i + 1, ")")) |end| {
                const content = s[i + 1 .. end];
                if (isPublisherOrSeriesInfo(content)) {
                    // Skip this parenthetical
                    i = end + 1;
                    continue;
                }
                // Check if contains year
                if (year) |y| {
                    var year_buf: [4]u8 = undefined;
                    const year_str = std.fmt.bufPrint(&year_buf, "{d}", .{y}) catch "";
                    if (mem.indexOf(u8, content, year_str)) |_| {
                        // Skip this parenthetical
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        try result.append(s[i]);
        i += 1;
    }

    // Normalize spaces
    return try normalizeSpaces(allocator, result.items);
}

/// Smart parse author and title
fn smartParseAuthorTitle(allocator: Allocator, s: []const u8) !struct { authors: ?[]const u8, title: []const u8 } {
    const trimmed = trim(s);

    // Pattern 1: "Title (Author)" - author at the end in parentheses
    if (trimmed.len > 0 and trimmed[trimmed.len - 1] == ')') {
        if (mem.lastIndexOf(u8, trimmed, "(")) |paren_start| {
            const title_part = trim(trimmed[0..paren_start]);
            const author_part = trim(trimmed[paren_start + 1 .. trimmed.len - 1]);

            if (isLikelyAuthor(author_part) and !isPublisherOrSeriesInfo(author_part)) {
                const authors = try cleanAuthorName(allocator, author_part);
                const title = try cleanTitle(allocator, title_part);
                return .{ .authors = authors, .title = title };
            }
        }
    }

    // Pattern 2: "Author - Title" or "Author -- Title"
    if (mem.indexOf(u8, trimmed, " -- ")) |idx| {
        const author_part = trim(trimmed[0..idx]);
        const title_part = trim(trimmed[idx + 4 ..]);
        if (isLikelyAuthor(author_part) and title_part.len > 0) {
            const authors = try cleanAuthorName(allocator, author_part);
            const title = try cleanTitle(allocator, title_part);
            return .{ .authors = authors, .title = title };
        }
    }

    if (mem.indexOf(u8, trimmed, " - ")) |idx| {
        const author_part = trim(trimmed[0..idx]);
        const title_part = trim(trimmed[idx + 3 ..]);
        if (isLikelyAuthor(author_part) and title_part.len > 0) {
            const authors = try cleanAuthorName(allocator, author_part);
            const title = try cleanTitle(allocator, title_part);
            return .{ .authors = authors, .title = title };
        }
    }

    // Pattern 3: "Author: Title"
    if (mem.indexOf(u8, trimmed, ": ")) |idx| {
        const author_part = trim(trimmed[0..idx]);
        const title_part = trim(trimmed[idx + 2 ..]);
        if (isLikelyAuthor(author_part) and title_part.len > 0) {
            const authors = try cleanAuthorName(allocator, author_part);
            const title = try cleanTitle(allocator, title_part);
            return .{ .authors = authors, .title = title };
        }
    }

    // No clear author, treat as title only
    const title = try cleanTitle(allocator, trimmed);
    return .{ .authors = null, .title = title };
}

/// Check if string looks like an author name
fn isLikelyAuthor(s: []const u8) bool {
    const trimmed = trim(s);
    if (trimmed.len < 2) return false;

    // Check for non-author keywords
    var lower_buf: [256]u8 = undefined;
    const lower = toLower(trimmed, &lower_buf);
    for (non_author_keywords) |kw| {
        if (mem.indexOf(u8, lower, kw)) |_| {
            return false;
        }
    }

    // Check if digits only
    var all_digits = true;
    for (trimmed) |c| {
        if (!std.ascii.isDigit(c) and c != '-' and c != '_') {
            all_digits = false;
            break;
        }
    }
    if (all_digits) return false;

    // Check for uppercase letter or non-ASCII character
    for (trimmed) |c| {
        if (std.ascii.isUpper(c)) return true;
        if (c > 127) return true; // Non-ASCII (likely Unicode name)
    }

    return false;
}

/// Clean author name
fn cleanAuthorName(allocator: Allocator, s: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);
    var i: usize = 0;

    // Remove (auth.), (author), etc.
    while (i < s.len) {
        if (s[i] == '(') {
            if (mem.indexOfPos(u8, s, i + 1, ")")) |end| {
                const content = s[i + 1 .. end];
                var lower_buf: [64]u8 = undefined;
                const lower = toLower(content, &lower_buf);
                if (mem.indexOf(u8, lower, "auth") != null or
                    mem.indexOf(u8, lower, "eds") != null or
                    mem.indexOf(u8, lower, "translator") != null)
                {
                    i = end + 1;
                    continue;
                }
            }
        }
        try result.append(s[i]);
        i += 1;
    }

    // Smart comma handling - join single words
    const temp = try result.toOwnedSlice();
    defer allocator.free(temp);

    const comma_count = mem.count(u8, temp, ",");
    if (comma_count == 1) {
        if (mem.indexOf(u8, temp, ", ")) |comma_idx| {
            const before = trim(temp[0..comma_idx]);
            const after = trim(temp[comma_idx + 2 ..]);
            const before_words = countWords(before);
            const after_words = countWords(after);

            if (before_words == 1 and after_words == 1) {
                return try std.fmt.allocPrint(allocator, "{s} {s}", .{ before, after });
            }
        }
    }

    return try normalizeSpaces(allocator, temp);
}

/// Clean title
fn cleanTitle(allocator: Allocator, s: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);

    // Remove (auth.) patterns and clean underscores
    var i: usize = 0;
    while (i < s.len) {
        if (s[i] == '(') {
            if (mem.indexOfPos(u8, s, i + 1, ")")) |end| {
                const content = s[i + 1 .. end];
                var lower_buf: [64]u8 = undefined;
                const lower = toLower(content, &lower_buf);
                if (mem.indexOf(u8, lower, "auth") != null) {
                    i = end + 1;
                    continue;
                }
            }
        }
        if (s[i] == '_') {
            try result.append(' ');
        } else {
            try result.append(s[i]);
        }
        i += 1;
    }

    const temp = try result.toOwnedSlice();
    defer allocator.free(temp);

    // Clean orphaned brackets
    const cleaned = try cleanOrphanedBrackets(allocator, temp);
    defer allocator.free(cleaned);

    // Normalize spaces and trim punctuation
    const final = try normalizeSpaces(allocator, cleaned);
    defer allocator.free(final);
    const trimmed = trimPunctuation(final);

    return try allocator.dupe(u8, trimmed);
}

/// Check if string contains publisher or series info
fn isPublisherOrSeriesInfo(s: []const u8) bool {
    for (publisher_keywords) |kw| {
        if (mem.indexOf(u8, s, kw)) |_| {
            return true;
        }
    }

    // Check for hash patterns (8+ hex chars)
    var hex_count: usize = 0;
    for (s) |c| {
        if (std.ascii.isHex(c)) {
            hex_count += 1;
            if (hex_count >= 8) return true;
        } else {
            hex_count = 0;
        }
    }

    // Check for series info (mostly non-letters with numbers)
    var has_numbers = false;
    var non_letter_count: usize = 0;
    for (s) |c| {
        if (std.ascii.isDigit(c)) has_numbers = true;
        if (!std.ascii.isAlphabetic(c) and c != ' ') non_letter_count += 1;
    }
    if (has_numbers and non_letter_count > 2) return true;

    return false;
}

/// Remove bracketed annotations
fn removeBrackets(allocator: Allocator, s: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);
    var i: usize = 0;

    while (i < s.len) {
        if (s[i] == '[') {
            // Skip until ]
            while (i < s.len and s[i] != ']') {
                i += 1;
            }
            if (i < s.len) i += 1; // Skip ]
        } else {
            try result.append(s[i]);
            i += 1;
        }
    }

    return try normalizeSpaces(allocator, result.items);
}

/// Clean noise sources (Z-Library, libgen, etc.)
fn cleanNoiseSources(allocator: Allocator, s: []const u8) ![]u8 {
    var result = try allocator.dupe(u8, s);

    for (0..3) |_| {
        const before = result;
        for (noise_patterns) |pattern| {
            if (mem.indexOf(u8, result, pattern)) |idx| {
                // Find the extent to remove (including surrounding punctuation/spaces)
                var start = idx;
                var end = idx + pattern.len;

                // Extend start backwards for separators
                while (start > 0 and (result[start - 1] == ' ' or result[start - 1] == '-' or result[start - 1] == '(' or result[start - 1] == ')')) {
                    start -= 1;
                }

                // Extend end forwards for separators
                while (end < result.len and (result[end] == ' ' or result[end] == '-' or result[end] == '(' or result[end] == ')' or result[end] == '.')) {
                    end += 1;
                }

                var new_result = ArrayList(u8).init(allocator);
                new_result.appendSlice(result[0..start]) catch {};
                new_result.appendSlice(result[end..]) catch {};
                allocator.free(result);
                result = new_result.toOwnedSlice() catch result;
            }
        }
        if (mem.eql(u8, result, before)) break;
    }

    return result;
}

/// Remove duplicate markers (-2, (1), etc.)
fn removeDuplicateMarkers(allocator: Allocator, s: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);
    var i: usize = 0;

    while (i < s.len) {
        // Check for (N) at end
        if (s[i] == '(' and i + 2 < s.len) {
            var j = i + 1;
            while (j < s.len and std.ascii.isDigit(s[j])) {
                j += 1;
            }
            if (j > i + 1 and j < s.len and s[j] == ')') {
                // Check if near end
                const rest = trim(s[j + 1 ..]);
                if (rest.len == 0) {
                    // Skip this marker
                    break;
                }
            }
        }

        // Check for -N at end
        if (s[i] == '-' and i + 1 < s.len and std.ascii.isDigit(s[i + 1])) {
            var j = i + 1;
            while (j < s.len and std.ascii.isDigit(s[j])) {
                j += 1;
            }
            const rest = trim(s[j..]);
            if (rest.len == 0) {
                // Skip this marker
                break;
            }
        }

        try result.append(s[i]);
        i += 1;
    }

    return try normalizeSpaces(allocator, result.items);
}

/// Clean orphaned brackets
fn cleanOrphanedBrackets(allocator: Allocator, s: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);
    var open_parens: usize = 0;
    var open_brackets: usize = 0;

    for (s) |c| {
        switch (c) {
            '(' => {
                open_parens += 1;
                try result.append(c);
            },
            ')' => {
                if (open_parens > 0) {
                    open_parens -= 1;
                    try result.append(c);
                }
                // Skip orphaned closing paren
            },
            '[' => {
                open_brackets += 1;
                try result.append(c);
            },
            ']' => {
                if (open_brackets > 0) {
                    open_brackets -= 1;
                    try result.append(c);
                }
                // Skip orphaned closing bracket
            },
            else => try result.append(c),
        }
    }

    // Remove unclosed opening brackets from the end
    var final = result.items;
    while (final.len > 0 and (final[final.len - 1] == '(' or final[final.len - 1] == '[')) {
        final = final[0 .. final.len - 1];
    }

    return try allocator.dupe(u8, final);
}

/// Generate new filename from metadata
pub fn generateNewFilename(allocator: Allocator, metadata: *const ParsedMetadata, extension: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);

    // Author(s)
    if (metadata.authors) |authors| {
        try result.appendSlice(authors);
        try result.appendSlice(" - ");
    }

    // Title
    try result.appendSlice(metadata.title);

    // Series info in brackets
    if (metadata.series) |series| {
        try result.appendSlice(" [");
        try result.appendSlice(series);
        try result.append(']');
    }

    // Year and Edition in parentheses
    if (metadata.year != null and metadata.edition != null) {
        try result.appendSlice(" (");
        var year_buf: [4]u8 = undefined;
        const year_str = try std.fmt.bufPrint(&year_buf, "{d}", .{metadata.year.?});
        try result.appendSlice(year_str);
        try result.appendSlice(", ");
        try result.appendSlice(metadata.edition.?);
        try result.append(')');
    } else if (metadata.year) |year| {
        try result.appendSlice(" (");
        var year_buf: [4]u8 = undefined;
        const year_str = try std.fmt.bufPrint(&year_buf, "{d}", .{year});
        try result.appendSlice(year_str);
        try result.append(')');
    } else if (metadata.edition) |edition| {
        try result.appendSlice(" (");
        try result.appendSlice(edition);
        try result.append(')');
    }

    // Extension
    try result.appendSlice(extension);

    return try result.toOwnedSlice();
}

// ============= Helper Functions =============

fn trim(s: []const u8) []const u8 {
    return mem.trim(u8, s, " \t\n\r");
}

fn trimLeft(s: []const u8) []const u8 {
    return mem.trimLeft(u8, s, " \t\n\r");
}

fn trimRight(s: []const u8) []const u8 {
    return mem.trimRight(u8, s, " \t\n\r");
}

fn trimPunctuation(s: []const u8) []const u8 {
    var result = s;
    while (result.len > 0 and (result[0] == '-' or result[0] == ':' or result[0] == ',' or result[0] == ';' or result[0] == '.')) {
        result = result[1..];
    }
    while (result.len > 0 and (result[result.len - 1] == '-' or result[result.len - 1] == ':' or result[result.len - 1] == ',' or result[result.len - 1] == ';' or result[result.len - 1] == '.')) {
        result = result[0 .. result.len - 1];
    }
    return trim(result);
}

fn normalizeSpaces(allocator: Allocator, s: []const u8) ![]u8 {
    var result = ArrayList(u8).init(allocator);
    var last_was_space = true;

    for (s) |c| {
        if (c == ' ' or c == '\t' or c == '\n' or c == '\r') {
            if (!last_was_space) {
                try result.append(' ');
                last_was_space = true;
            }
        } else {
            try result.append(c);
            last_was_space = false;
        }
    }

    // Trim trailing space
    var final = result.items;
    if (final.len > 0 and final[final.len - 1] == ' ') {
        final = final[0 .. final.len - 1];
    }
    // Trim leading space
    if (final.len > 0 and final[0] == ' ') {
        final = final[1..];
    }

    return try allocator.dupe(u8, final);
}

fn toLower(s: []const u8, buf: []u8) []const u8 {
    const len = @min(s.len, buf.len);
    for (0..len) |i| {
        buf[i] = std.ascii.toLower(s[i]);
    }
    return buf[0..len];
}

fn startsWithIgnoreCase(s: []const u8, prefix: []const u8) bool {
    if (s.len < prefix.len) return false;
    for (0..prefix.len) |i| {
        if (std.ascii.toLower(s[i]) != std.ascii.toLower(prefix[i])) {
            return false;
        }
    }
    return true;
}

fn containsIgnoreCase(s: []const u8, substr: []const u8) bool {
    if (s.len < substr.len) return false;
    for (0..s.len - substr.len + 1) |i| {
        var found = true;
        for (0..substr.len) |j| {
            if (std.ascii.toLower(s[i + j]) != std.ascii.toLower(substr[j])) {
                found = false;
                break;
            }
        }
        if (found) return true;
    }
    return false;
}

fn countWords(s: []const u8) usize {
    var count: usize = 0;
    var in_word = false;

    for (s) |c| {
        if (c == ' ' or c == '\t') {
            if (in_word) {
                in_word = false;
            }
        } else {
            if (!in_word) {
                count += 1;
                in_word = true;
            }
        }
    }

    return count;
}

// ============= Tests =============

test "extract year" {
    try std.testing.expectEqual(@as(?u16, 2020), extractYear("Title (2020)"));
    try std.testing.expectEqual(@as(?u16, 1999), extractYear("Author - Book 1999"));
    try std.testing.expectEqual(@as(?u16, null), extractYear("No year here"));
    try std.testing.expectEqual(@as(?u16, 2020), extractYear("Title (2018, Publisher) (2020)")); // Last year
}

test "is likely author" {
    try std.testing.expect(isLikelyAuthor("John Smith"));
    try std.testing.expect(isLikelyAuthor("J. K. Rowling"));
    try std.testing.expect(!isLikelyAuthor("auth."));
    try std.testing.expect(!isLikelyAuthor("12345"));
    try std.testing.expect(!isLikelyAuthor("Z-Library"));
}

test "is publisher or series info" {
    try std.testing.expect(isPublisherOrSeriesInfo("Cambridge University Press"));
    try std.testing.expect(isPublisherOrSeriesInfo("Graduate Texts in Mathematics"));
    try std.testing.expect(!isPublisherOrSeriesInfo("John Smith"));
}

test "get ordinal suffix" {
    try std.testing.expectEqualStrings("st", getOrdinalSuffix("1"));
    try std.testing.expectEqualStrings("nd", getOrdinalSuffix("2"));
    try std.testing.expectEqualStrings("rd", getOrdinalSuffix("3"));
    try std.testing.expectEqualStrings("th", getOrdinalSuffix("4"));
    try std.testing.expectEqualStrings("th", getOrdinalSuffix("11"));
}
