require_relative 'types'

module EbookRenamer
  class Normalizer
    # Regex patterns
    YEAR_REGEX = /\b(?:19|20)\d{2}\b/
    AUTH_REGEX = /\s*\([Aa]uth\.?\).*/
    SPACE_REGEX = /\s{2,}/
    BRACKET_REGEX = /\s*\[[^\]]*\]/
    TRAILING_ID_REGEX = /[-_][A-Za-z0-9]{8,}$/
    SIMPLE_PAREN_REGEX = /\([^)]+\)/
    NESTED_PAREN_REGEX = /\([^()]*(?:\([^()]*\)[^()]*)*\)/
    TRAILING_AUTHOR_REGEX = /^(.+?)\s*\(([^)]+)\)\s*$/
    SEPARATOR_REGEX = /^(.+?)\s*[-:]\s+(.+)$/
    MULTI_AUTHOR_REGEX = /^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*[-:]\s+(.+)$/

    # Series patterns
    SERIES_REGEX = /\[([A-Z]+)\s+(\d+)\]/

    # Edition patterns
    EDITION_REGEX = /(\d+)(?:st|nd|rd|th)?\s*[Ee]d(?:ition)?/
    EDITION_KEYWORD_REGEX = /(?:Second|Third|Fourth|Fifth|Sixth|Seventh|Eighth|Ninth|Tenth)\s*[Ee]d(?:ition)?/

    # Volume patterns
    VOLUME_REGEX = /\b[Vv]ol(?:ume|\.)?\s*(\d+)\b/
    PART_REGEX = /\b[Pp]art\s*(\d+)\b/

    SOURCE_INDICATORS = [
      / - libgen\.li$/, / - libgen$/, / - Z-Library$/, / - z-Library$/,
      / - Anna's Archive$/, / \(Z-Library\)$/, / \(z-Library\)$/,
      / \(libgen\.li\)$/, / \(libgen\)$/, / \(Anna's Archive\)$/,
      / libgen\.li\.pdf$/, / libgen\.pdf$/, / Z-Library\.pdf$/,
      / z-Library\.pdf$/, / Anna's Archive\.pdf$/
    ]

    NON_AUTHOR_KEYWORDS = [
      "auth.", "translator", "translated by", "z-library", "libgen", "anna's archive", "2-library"
    ]

    PUBLISHER_KEYWORDS = [
      "Press", "Publishing", "Academic Press", "Springer", "Cambridge", "Oxford", "MIT Press",
      "Series", "Textbook Series", "Graduate Texts", "Graduate Studies", "Lecture Notes",
      "Pure and Applied", "Mathematics", "Foundations of", "Monographs", "Studies", "Collection",
      "Textbook", "Edition", "Vol.", "Volume", "No.", "Part", "理工", "出版社", "の"
    ]

    STRICT_PUBLISHER_KEYWORDS = [
      "Press", "Publishing", "Springer", "Cambridge", "Oxford", "MIT",
      "Wiley", "Elsevier", "Routledge", "Pearson", "McGraw", "Addison",
      "Prentice", "O'Reilly", "Princeton", "Harvard", "Yale", "Stanford",
      "Chicago", "California", "Columbia", "University", "Verlag", "Birkhäuser", "CUP"
    ]

    def normalize_files(files)
      files.map do |file|
        next file if file.is_failed_download || file.is_too_small

        metadata = parse_filename(file.original_name, file.extension)
        new_name = generate_new_filename(metadata, file.extension)

        file.new_name = new_name
        file.new_path = File.join(File.dirname(file.original_path), new_name)
        file
      end
    end

    def parse_filename(filename, extension)
      # Step 1: Remove extension
      base = filename.dup
      base = base.chomp('.download')
      base = base.chomp(extension)
      base = base.strip

      # Step 2: Extract series information (before removal)
      series_info, base = extract_series(base)

      # Step 3: Remove series prefixes (legacy)
      base = remove_series_prefixes(base)

      # Step 4: Remove ALL bracketed annotations (except what we extracted)
      # Note: If we extracted series from [], they are gone from base.
      # Now remove remaining [].
      base = base.gsub(BRACKET_REGEX, '')

      # Step 5: Clean noise sources
      base = clean_noise_sources(base)

      # Step 6: Extract edition information
      edition_info, base = extract_edition(base)

      # Step 7: Extract year FIRST (find last occurrence)
      year = extract_year(base)

      # Step 8: Remove parentheticals
      base = clean_parentheticals(base, year)

      # Step 9: Extract volume information
      # Note: We extract volume here and will inject it back into title if needed,
      # or handle it separately. The spec says "Vol N - kept in title".
      # But to normalize it (Volume 2 -> Vol 2), we might want to extract and reinject.
      # Let's try to normalize it in place or extract it.
      # If we extract it, we can append it to title later.
      volume_info, base = extract_volume(base)

      # Step 10: Parse author and title
      authors, title = smart_parse_author_title(base)

      # Re-inject volume into title if present
      if volume_info
        title = "#{title} #{volume_info}"
      end

      ParsedMetadata.new(
        authors: authors,
        title: title,
        year: year,
        series: series_info,
        volume: volume_info, # kept in title usually, but stored here for reference
        edition: edition_info
      )
    end

    def extract_series(s)
      # Look for [ABBR NUM] patterns
      # If found, return it and remove from string
      # Currently we just handle known patterns or generic [Word Num]

      # For now, let's look for known series patterns if needed,
      # or just generic [A-Z+ \d+]

      match = s.match(SERIES_REGEX)
      if match
        series_str = match[0] # [GTM 52]
        remaining = s.gsub(match[0], '').strip
        # Return format without brackets for storage? Or with?
        # Spec says output: [Series Volume]
        # Let's store "GTM 52" (content)
        return ["#{match[1]} #{match[2]}", remaining]
      end

      # Also check for Series Name in text if we want to convert to abbr,
      # but spec implies existing brackets mostly.
      # "Graduate Texts in Mathematics 52" -> "GTM 52"

      series_map = {
        "Graduate Texts in Mathematics" => "GTM",
        "Cambridge Studies in Advanced Mathematics" => "CSAM",
        "London Mathematical Society Lecture Note Series" => "LMSLN",
        "Progress in Mathematics" => "PM",
        "Springer Undergraduate Mathematics Series" => "SUMS",
        "Graduate Studies in Mathematics" => "GSM"
      }

      series_map.each do |name, abbr|
        pattern = /#{Regexp.escape(name)}\s+(\d+)/
        if m = s.match(pattern)
          return ["#{abbr} #{m[1]}", s.gsub(m[0], '').strip]
        end
      end

      [nil, s]
    end

    def extract_edition(s)
      # Look for "Nth ed" or "Nth Edition"
      match = s.match(EDITION_REGEX)
      if match
        n = match[1]
        suffix = case n
                 when '1' then 'st'
                 when '2' then 'nd'
                 when '3' then 'rd'
                 else 'th'
                 end
        # Fix 11th, 12th, 13th
        suffix = 'th' if ['11','12','13'].include?(n)

        return ["#{n}#{suffix} ed", s.gsub(match[0], '').strip]
      end

      # Word based edition
      match = s.match(EDITION_KEYWORD_REGEX)
      if match
        word = match[0].split.first
        n = case word
            when 'Second' then '2nd'
            when 'Third' then '3rd'
            when 'Fourth' then '4th'
            when 'Fifth' then '5th'
            when 'Sixth' then '6th'
            when 'Seventh' then '7th'
            when 'Eighth' then '8th'
            when 'Ninth' then '9th'
            when 'Tenth' then '10th'
            else nil
            end
        if n
          return ["#{n} ed", s.gsub(match[0], '').strip]
        end
      end

      [nil, s]
    end

    def extract_volume(s)
      # Normalize Volume 2 -> Vol 2
      # Part 3 -> Vol 3

      match = s.match(VOLUME_REGEX)
      if match
        return ["Vol #{match[1]}", s.gsub(match[0], '').strip]
      end

      match = s.match(PART_REGEX)
      if match
        return ["Vol #{match[1]}", s.gsub(match[0], '').strip]
      end

      [nil, s]
    end

    def remove_series_prefixes(s)
      prefixes = [
        "London Mathematical Society Lecture Note Series",
        "Graduate Texts in Mathematics",
        "Progress in Mathematics",
        "[Springer-Lehrbuch]",
        "[Graduate studies in mathematics",
        "[Progress in Mathematics №",
        "[AMS Mathematical Surveys and Monographs"
      ]

      result = s
      prefixes.each do |prefix|
        if result.start_with?(prefix)
          result = result[prefix.length..-1]
          result = result.sub(/^[- \]]+/, '')
          break
        end
      end

      # Generic pattern: (Series Name) Author - Title
      generic_match = result.match(/^\s*\(([^)]+)\)\s+(.+)$/)
      if generic_match
        rest_part = generic_match[2]
        sep_match = rest_part.match(/(?:--|[-:])/)
        potential_author = rest_part
        if sep_match
          potential_author = rest_part[0...sep_match.begin(0)]
        end

        if is_likely_author(potential_author)
          result = rest_part
        end
      end

      result.strip
    end

    def clean_noise_sources(s)
      result = s
      SOURCE_INDICATORS.each do |ind|
        if ind.is_a?(Regexp)
          result = result.gsub(ind, '')
        else
          # If it were a string, but we used Regexps in the array
        end
      end
      result.strip
    end

    def extract_year(s)
      matches = s.scan(YEAR_REGEX)
      matches.empty? ? nil : matches.last.to_i
    end

    def clean_parentheticals(s, year)
      result = s

      # Pattern 1: Remove (YYYY, Publisher) or (YYYY)
      if year
        pattern = /\s*\(\s*#{year}\s*(?:,\s*[^)]+)?\s*\)/
        result = result.gsub(pattern, '')
      end

      # Pattern 2: Remove nested parentheticals with publisher keywords
      loop do
        changed = false
        result = result.gsub(NESTED_PAREN_REGEX) do |match|
          if is_publisher_or_series_info(match)
            changed = true
            ''
          else
            match
          end
        end
        break unless changed
      end

      # Pattern 3: Remove simple parentheticals
      result = result.gsub(SIMPLE_PAREN_REGEX) do |match|
        is_publisher_or_series_info(match) ? '' : match
      end

      result.gsub(SPACE_REGEX, ' ').strip
    end

    def smart_parse_author_title(s)
      s = s.strip

      # Pattern 1: Title (Author)
      if match = s.match(TRAILING_AUTHOR_REGEX)
        title_part, author_part = match.captures
        if is_likely_author(author_part) && !is_publisher_or_series_info("("+author_part+")")
          return [clean_author_name(author_part), clean_title(title_part)]
        end
      end

      # Pattern 2: Author - Title
      if match = s.match(SEPARATOR_REGEX)
        author_part, title_part = match.captures
        if is_likely_author(author_part) && !title_part.empty?
          return [clean_author_name(author_part), clean_title(title_part)]
        end
      end

      # Pattern 3: Multi author
      if match = s.match(MULTI_AUTHOR_REGEX)
        author1, author2, title_part = match.captures
        if is_likely_author(author1) && is_likely_author(author2)
          authors = "#{clean_author_name(author1)}, #{clean_author_name(author2)}"
          return [authors, clean_title(title_part)]
        end
      end

      [nil, clean_title(s)]
    end

    def is_likely_author(s)
      s = s.strip
      return false if s.length < 2

      s_lower = s.downcase
      return false if NON_AUTHOR_KEYWORDS.any? { |k| s_lower.include?(k) }

      # Check if digits only
      return false if s.match?(/^[\d\-_]+$/)

      # Check if name-like (uppercase Latin OR non-Latin letter)
      has_uppercase = s.match?(/[A-Z]/)
      has_non_latin = s.match?(/[^\x00-\x7F]/)

      has_uppercase || has_non_latin
    end

    def clean_author_name(s)
      s = s.strip.gsub(AUTH_REGEX, '')

      comma_count = s.count(',')
      if comma_count == 1
        parts = s.split(', ')
        if parts.length == 2
          before, after = parts.map(&:strip)
          if before.split.length == 1 && after.split.length == 1
            s = "#{before} #{after}"
          end
        end
      end

      s.gsub(SPACE_REGEX, ' ').strip
    end

    def clean_title(s)
      s = s.strip
      s = clean_noise_sources(s)
      s = s.gsub(AUTH_REGEX, '')
      s = s.gsub(TRAILING_ID_REGEX, '')

      # Remove trailing publisher info separated by dash
      if idx = s.rindex(' - ')
        suffix = s[(idx + 3)..-1]
        if is_publisher_or_series_info(suffix)
          s = s[0...idx]
        end
      end

      # Also handle just "-" without spaces if it looks like publisher
      if idx = s.rindex('-')
        if idx > 0 && idx < s.length - 1
          suffix = s[(idx + 1)..-1].strip
          if is_strict_publisher_info(suffix)
            s = s[0...idx]
          end
        end
      end

      s = clean_orphaned_brackets(s)
      s = s.gsub(SPACE_REGEX, ' ').strip
      s.gsub(/^[-:;,\.]+|[-:;,\.]+$/, '').strip
    end

    def is_publisher_or_series_info(s)
      return true if PUBLISHER_KEYWORDS.any? { |k| s.include?(k) }

      has_numbers = s.match?(/\d/)
      non_letter_count = s.scan(/[^a-zA-Z ]/).length

      has_numbers && non_letter_count > 2
    end

    def is_strict_publisher_info(s)
      STRICT_PUBLISHER_KEYWORDS.any? { |k| s.include?(k) }
    end

    def clean_orphaned_brackets(s)
      result = ""
      open_parens = 0
      open_brackets = 0

      s.each_char do |char|
        case char
        when '('
          open_parens += 1
          result << char
        when ')'
          if open_parens > 0
            open_parens -= 1
            result << char
          end
        when '['
          open_brackets += 1
          result << char
        when ']'
          if open_brackets > 0
            open_brackets -= 1
            result << char
          end
        when '_'
          result << ' '
        else
          result << char
        end
      end

      while result.end_with?('(') || result.end_with?('[')
        result = result[0...-1]
      end
      
      result
    end

    def generate_new_filename(metadata, extension)
      parts = []

      # Author -
      parts << "#{metadata.authors} -" if metadata.authors

      # Title
      parts << metadata.title

      # [Series Volume]
      parts << "[#{metadata.series}]" if metadata.series

      # (Year, Edition)
      year_edition = []
      year_edition << metadata.year if metadata.year
      year_edition << metadata.edition if metadata.edition

      parts << "(#{year_edition.join(', ')})" unless year_edition.empty?

      # Extension
      name = parts.join(' ').gsub('  ', ' ').strip.gsub(/ \./, '.')
      name + extension
    end
  end
end
