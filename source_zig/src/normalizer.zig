const std = @import("std");
const mem = std.mem;
const ascii = std.ascii;

pub const ParsedMetadata = struct {
    authors: ?[]const u8,
    title: []const u8,
    year: ?u16,
    series: ?[]const u8,
    edition: ?[]const u8,
    volume: ?[]const u8,
};

pub const Normalizer = struct {
    allocator: mem.Allocator,

    pub fn init(allocator: mem.Allocator) Normalizer {
        return Normalizer{ .allocator = allocator };
    }

    pub fn normalize(self: *Normalizer, original_name: []const u8, extension: []const u8) ![]const u8 {
        var arena = std.heap.ArenaAllocator.init(self.allocator);
        defer arena.deinit();
        const allocator = arena.allocator();

        const metadata = try self.parseFilename(allocator, original_name, extension);
        return self.generateNewFilename(self.allocator, metadata, extension);
    }

    fn parseFilename(self: *Normalizer, allocator: mem.Allocator, filename: []const u8, extension: []const u8) !ParsedMetadata {
        // Step 1: Remove extension
        var base = filename;
        if (mem.endsWith(u8, base, ".download")) {
            base = base[0 .. base.len - ".download".len];
        }
        if (mem.endsWith(u8, base, extension)) {
            base = base[0 .. base.len - extension.len];
        }
        base = mem.trim(u8, base, " ");

        // Step 2: Remove series prefixes
        base = try self.removeSeriesPrefixes(allocator, base);

        // Step 3: Clean noise
        base = try self.cleanNoiseSources(allocator, base);

        // Step 4: Extract Series
        var series: ?[]const u8 = null;
        const series_res = try self.extractSeries(allocator, base);
        if (series_res.match) |s| {
            series = s;
            base = series_res.rest;
        }

        // Step 5: Remove brackets
        base = try self.removeBrackets(allocator, base);

        // Step 6: Extract Edition
        var edition: ?[]const u8 = null;
        const edition_res = try self.extractEdition(allocator, base);
        if (edition_res.match) |e| {
            edition = e;
            base = edition_res.rest;
        }

        // Step 7: Duplicate markers (skip for brevity in this Zig implementation, or simple impl)
        // base = removeDuplicateMarkers(base);

        // Step 8: Extract Year
        const year_res = self.extractYear(base);
        const year = year_res.year;

        // Step 9: Remove parentheticals
        base = try self.cleanParentheticals(allocator, base, year);

        // Step 10: Parse Author/Title
        const at_res = try self.smartParseAuthorTitle(allocator, base);

        // Step 11: Volume normalization in title
        const title = try self.normalizeVolume(allocator, at_res.title);

        return ParsedMetadata{
            .authors = at_res.authors,
            .title = title,
            .year = year,
            .series = series,
            .edition = edition,
            .volume = null, // kept in title
        };
    }

    fn removeSeriesPrefixes(self: *Normalizer, allocator: mem.Allocator, s: []const u8) ![]const u8 {
        // Implementing GTM generic pattern.
        // If starts with "Graduate Texts in Mathematics 52 - ", return "[GTM 52] " + rest

        const prefixes = [_]struct{ full: []const u8, abbr: []const u8 }{
            .{ .full = "Graduate Texts in Mathematics", .abbr = "GTM" },
            .{ .full = "London Mathematical Society Lecture Note Series", .abbr = "LMSLN" },
            .{ .full = "Progress in Mathematics", .abbr = "PM" },
            .{ .full = "Springer Undergraduate Mathematics Series", .abbr = "SUMS" },
            .{ .full = "Graduate Studies in Mathematics", .abbr = "GSM" },
            .{ .full = "Cambridge Studies in Advanced Mathematics", .abbr = "CSAM" },
        };

        for (prefixes) |p| {
            if (mem.startsWith(u8, s, p.full)) {
                var rest = s[p.full.len..];
                rest = mem.trimLeft(u8, rest, " ");

                // Check for number
                var num_len: usize = 0;
                while (num_len < rest.len and ascii.isDigit(rest[num_len])) {
                    num_len += 1;
                }

                if (num_len > 0) {
                     const number = rest[0..num_len];
                     // Check if followed by separator
                     var after_num = rest[num_len..];
                     after_num = mem.trimLeft(u8, after_num, " ");

                     if (mem.startsWith(u8, after_num, "-") or mem.startsWith(u8, after_num, ":")) {
                         // Skip separator
                         var sep_len: usize = 1;
                         if (mem.startsWith(u8, after_num, "--")) sep_len = 2;

                         const content = after_num[sep_len..];
                         const clean_content = mem.trim(u8, content, " ");

                         // Construct [GTM 52] Content
                         return try std.fmt.allocPrint(allocator, "[{s} {s}] {s}", .{p.abbr, number, clean_content});
                     }
                }
            }
        }

        _ = self;
        return s;
    }

    fn cleanNoiseSources(self: *Normalizer, allocator: mem.Allocator, s: []const u8) ![]const u8 {
        // Simple noise removal
        var result = s;
        const noises = [_][]const u8{ "Z-Library", "libgen", "Anna's Archive" };

        for (noises) |noise| {
            if (mem.indexOf(u8, result, noise)) |idx| {
                 // Find boundaries
                 var start = idx;
                 if (start > 0 and (result[start-1] == '-' or result[start-1] == '(')) start -= 1;

                 var end = idx + noise.len;
                 if (end < result.len and (result[end] == ')' or mem.startsWith(u8, result[end..], ".pdf"))) {
                     if (result[end] == ')') end += 1;
                     // .pdf part handled by ext removal usually, but noise inside name might keep it?
                 }

                 const part1 = result[0..start];
                 const part2 = result[end..];
                 result = try std.fmt.allocPrint(allocator, "{s}{s}", .{part1, part2});
            }
        }
        return mem.trim(u8, result, " ");
    }

    const ExtractResult = struct {
        match: ?[]const u8,
        rest: []const u8,
    };

    fn extractSeries(self: *Normalizer, allocator: mem.Allocator, s: []const u8) !ExtractResult {
        // Look for [GTM 52] pattern: [ + uppercase + space + digits + ]
        var i: usize = 0;
        while (i < s.len) {
            if (s[i] == '[') {
                // Scan content
                const start = i;
                var j = i + 1;
                var has_upper = false;
                var has_space = false;
                var has_digit = false;

                while (j < s.len and s[j] != ']') {
                    if (ascii.isUpper(s[j])) has_upper = true;
                    if (s[j] == ' ') has_space = true;
                    if (ascii.isDigit(s[j])) has_digit = true;
                    j += 1;
                }

                if (j < s.len and s[j] == ']') {
                    if (has_upper and has_space and has_digit) {
                        const match_content = s[start+1 .. j]; // content inside []
                        // Extract "GTM 52"
                        const match_str = try allocator.dupe(u8, match_content);

                        // Construct rest
                        const part1 = s[0..start];
                        const part2 = s[j+1..];
                        const rest = try std.fmt.allocPrint(allocator, "{s}{s}", .{part1, part2});
                        return ExtractResult{ .match = match_str, .rest = mem.trim(u8, rest, " ") };
                    }
                    i = j;
                }
            }
            i += 1;
        }
        return ExtractResult{ .match = null, .rest = s };
    }

    fn removeBrackets(self: *Normalizer, allocator: mem.Allocator, s: []const u8) ![]const u8 {
        // Remove [...] content
        var result = std.ArrayList(u8).init(allocator);
        var i: usize = 0;
        while (i < s.len) {
            if (s[i] == '[') {
                while (i < s.len and s[i] != ']') {
                    i += 1;
                }
                if (i < s.len) i += 1; // skip ]
            } else {
                try result.append(s[i]);
                i += 1;
            }
        }
        return mem.trim(u8, try result.toOwnedSlice(), " ");
    }

    fn extractEdition(self: *Normalizer, allocator: mem.Allocator, s: []const u8) !ExtractResult {
        // Look for "2nd ed", "3rd Edition"
        // Simple heuristic: Look for digit followed by "nd ed", "rd ed", "th ed", "st ed"
        const suffixes = [_][]const u8{ "st", "nd", "rd", "th" };

        var i: usize = 0;
        while (i < s.len) {
            if (ascii.isDigit(s[i])) {
                const num_start = i;
                while (i < s.len and ascii.isDigit(s[i])) i += 1;
                const number = s[num_start..i];

                // Check what follows
                const rest = s[i..];
                var matched = false;
                var end_idx = i;

                // Handle "2nd"
                for (suffixes) |suf| {
                    if (mem.startsWith(u8, rest, suf)) {
                        // Check if followed by " ed" or " Edition"
                        const after_suf = rest[suf.len..];
                        if (mem.startsWith(u8, after_suf, " ed") or mem.startsWith(u8, after_suf, " Ed")) {
                            matched = true;
                            // Find end of " ed..."
                            // assume " ed" or " Edition" ends at next space or paren
                            // Rough: find end of "Edition"
                            if (mem.indexOf(u8, after_suf, "ition")) |it_idx| {
                                end_idx = i + suf.len + it_idx + "ition".len;
                            } else {
                                end_idx = i + suf.len + " ed".len;
                            }
                        }
                    }
                }

                if (matched) {
                    // Extract normalized edition
                    var norm_suf: []const u8 = "th";
                    if (mem.eql(u8, number, "1") and !mem.endsWith(u8, number, "11")) norm_suf = "st";
                    if (mem.eql(u8, number, "2") and !mem.endsWith(u8, number, "12")) norm_suf = "nd";
                    if (mem.eql(u8, number, "3") and !mem.endsWith(u8, number, "13")) norm_suf = "rd";

                    const edition_str = try std.fmt.allocPrint(allocator, "{s}{s} ed", .{number, norm_suf});

                    const part1 = s[0..num_start];
                    const part2 = s[end_idx..];
                    const new_s = try std.fmt.allocPrint(allocator, "{s}{s}", .{part1, part2});

                    return ExtractResult{ .match = edition_str, .rest = mem.trim(u8, new_s, " ") };
                }
            }
            i += 1;
        }

        _ = self;
        return ExtractResult{ .match = null, .rest = s };
    }

    const YearResult = struct {
        year: ?u16,
    };

    fn extractYear(self: *Normalizer, s: []const u8) YearResult {
        // Find last 19xx or 20xx
        var last_year: ?u16 = null;
        var i: usize = 0;
        while (i + 3 < s.len) {
            if (ascii.isDigit(s[i]) and ascii.isDigit(s[i+1]) and ascii.isDigit(s[i+2]) and ascii.isDigit(s[i+3])) {
                 const slice = s[i..i+4];
                 const val = std.fmt.parseInt(u16, slice, 10) catch 0;
                 if (val >= 1900 and val <= 2099) {
                     const before_ok = if (i == 0) true else !ascii.isDigit(s[i-1]);
                     const after_ok = if (i+4 == s.len) true else !ascii.isDigit(s[i+4]);
                     if (before_ok and after_ok) {
                         last_year = val;
                     }
                 }
            }
            i += 1;
        }
        _ = self;
        return YearResult{ .year = last_year };
    }

    fn cleanParentheticals(self: *Normalizer, allocator: mem.Allocator, s: []const u8, year: ?u16) ![]const u8 {
        var result = s;
        if (year) |y| {
            var buf: [32]u8 = undefined;
            const year_str = try std.fmt.bufPrint(&buf, "({d})", .{y});
            if (mem.indexOf(u8, result, year_str)) |idx| {
                 const part1 = result[0..idx];
                 const part2 = result[idx+year_str.len..];
                 result = try std.fmt.allocPrint(allocator, "{s}{s}", .{part1, part2});
            }

            // Try (YYYY, Publisher) - simplistic check
             const year_pub_start = try std.fmt.bufPrint(&buf, "({d}, ", .{y});
             if (mem.indexOf(u8, result, year_pub_start)) |idx| {
                 // Find matching close paren
                 var j = idx + year_pub_start.len;
                 while (j < result.len and result[j] != ')') j += 1;
                 if (j < result.len) {
                     const part1 = result[0..idx];
                     const part2 = result[j+1..];
                     result = try std.fmt.allocPrint(allocator, "{s}{s}", .{part1, part2});
                 }
             }
        }
        return mem.trim(u8, result, " ");
    }

    const AuthorTitleResult = struct {
        authors: ?[]const u8,
        title: []const u8,
    };

    fn smartParseAuthorTitle(self: *Normalizer, allocator: mem.Allocator, s: []const u8) !AuthorTitleResult {
        // Try "Author - Title"
        if (mem.indexOf(u8, s, " - ")) |idx| {
             const potential_author = s[0..idx];
             const title = s[idx+3..];

             if (self.isLikelyAuthor(potential_author)) {
                 return AuthorTitleResult{
                     .authors = try allocator.dupe(u8, potential_author),
                     .title = try allocator.dupe(u8, title)
                 };
             }
        }

        // Try "Title (Author)"
        if (mem.endsWith(u8, s, ")")) {
             if (mem.lastIndexOf(u8, s, " (")) |idx| {
                 const title = s[0..idx];
                 const author_part = s[idx+2 .. s.len-1];
                 if (self.isLikelyAuthor(author_part)) {
                      return AuthorTitleResult{
                          .authors = try allocator.dupe(u8, author_part),
                          .title = try allocator.dupe(u8, title)
                      };
                 }
             }
        }

        return AuthorTitleResult{ .authors = null, .title = try allocator.dupe(u8, s) };
    }

    fn isLikelyAuthor(self: *Normalizer, s: []const u8) bool {
        if (s.len < 2) return false;
        var has_upper = false;
        for (s) |c| {
            if (ascii.isUpper(c)) has_upper = true;
        }
        _ = self;
        return has_upper;
    }

    fn normalizeVolume(self: *Normalizer, allocator: mem.Allocator, title: []const u8) ![]const u8 {
        const needle = "Volume ";
        if (mem.indexOf(u8, title, needle)) |idx| {
             const part1 = title[0..idx];
             const part2 = "Vol ";
             const part3 = title[idx+needle.len..];
             return try std.fmt.allocPrint(allocator, "{s}{s}{s}", .{part1, part2, part3});
        }
        _ = self;
        return title;
    }

    fn generateNewFilename(self: *Normalizer, allocator: mem.Allocator, metadata: ParsedMetadata, extension: []const u8) ![]const u8 {
        var list = std.ArrayList(u8).init(allocator);

        if (metadata.authors) |auth| {
            try list.appendSlice(auth);
            try list.appendSlice(" - ");
        }

        try list.appendSlice(metadata.title);

        if (metadata.series) |ser| {
            try list.appendSlice(" [");
            try list.appendSlice(ser);
            try list.appendSlice("]");
        }

        if (metadata.year) |y| {
            try list.appendSlice(" (");
            var buf: [16]u8 = undefined;
            const y_str = try std.fmt.bufPrint(&buf, "{d}", .{y});
            try list.appendSlice(y_str);

            if (metadata.edition) |ed| {
                try list.appendSlice(", ");
                try list.appendSlice(ed);
            }
            try list.appendSlice(")");
        }

        try list.appendSlice(extension);

        return list.toOwnedSlice();
    }
};
