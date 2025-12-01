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
    # Ruby supports recursive regexes! (\((?>[^()]+|\g<0>)*\))
    # But for simple nesting like ( ... ( ... ) ... ) we can use loop or recursive pattern
    NESTED_PAREN_REGEX = /\([^()]*(?:\([^()]*\)[^()]*)*\)/
    TRAILING_AUTHOR_REGEX = /^(.+?)\s*\(([^)]+)\)\s*$/
    SEPARATOR_REGEX = /^(.+?)\s*[-:]\s+(.+)$/
    MULTI_AUTHOR_REGEX = /^([A-Z][^:]+?),\s*([A-Z][^:]+?)\s*[-:]\s+(.+)$/

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
      base = filename
      base = base.chomp('.download')
      base = base.chomp(extension)
      base = base.strip

      # Step 2: Remove series prefixes (must be early)
      base = remove_series_prefixes(base)

      # Step 3: Clean noise sources
      base = clean_noise_sources(base)

      # Step 4: Remove ALL bracketed annotations
      base = base.gsub(BRACKET_REGEX, '')

      # Step 5: Extract year FIRST
      year = extract_year(base)

      # Step 6: Remove parentheticals
      base = clean_parentheticals(base, year)

      # Step 7: Parse author and title
      authors, title = smart_parse_author_title(base)

      ParsedMetadata.new(authors: authors, title: title, year: year)
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
      # If it starts with (...), check if the next part looks like an author
      generic_match = result.match(/^\s*\(([^)]+)\)\s+(.+)$/)
      if generic_match
        # series_part = generic_match[1]
        rest_part = generic_match[2]

        # Check if 'rest_part' starts with an author
        # We look for the first separator (- or :) to isolate the potential author
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
      patterns = [
        /\s*[-\(]?\s*[zZ]-?Library(?:\.pdf)?\s*[)\.]?/,
        /\s*[-\(]?\s*libgen(?:\.li)?(?:\.pdf)?\s*[)\.]?/,
        /\s*[-\(]?\s*Anna'?s?\s+Archive(?:\.pdf)?\s*[)\.]?/
      ]
      result = s
      patterns.each { |p| result = result.gsub(p, '') }
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
      has_non_latin = s.match?(/[^\x00-\x7F]/) # Basic check for non-ASCII

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
      # e.g. "Title - Publisher"
      removed_dash_suffix = false
      if idx = s.rindex(' - ')
        suffix = s[(idx + 3)..-1]
        if is_publisher_or_series_info(suffix)
          s = s[0...idx]
          removed_dash_suffix = true
        end
      end

      # Also handle just "-" without spaces if it looks like publisher
      if !removed_dash_suffix && (idx = s.rindex('-'))
        if idx > 0 && idx < s.length - 1
          suffix = s[(idx + 1)..-1].strip
          # Use stricter check for non-spaced dash to avoid stripping parts of title
          if is_strict_publisher_info(suffix)
            s = s[0...idx]
          end
        end
      end

      s = clean_orphaned_brackets(s)
      s = s.gsub(SPACE_REGEX, ' ')
      s.gsub(/^[-:;,\.]+|[-:;,\.]+$/, '').strip
    end

    def is_publisher_or_series_info(s)
      return true if PUBLISHER_KEYWORDS.any? { |k| s.include?(k) }

      has_numbers = s.match?(/\d/)
      non_letter_count = s.scan(/[^a-zA-Z ]/).length

      has_numbers && non_letter_count > 2
    end

    def is_strict_publisher_info(s)
      strict_keywords = [
        "Press", "Publishing", "Springer", "Cambridge", "Oxford", "MIT",
        "Wiley", "Elsevier", "Routledge", "Pearson", "McGraw", "Addison",
        "Prentice", "O'Reilly", "Princeton", "Harvard", "Yale", "Stanford",
        "Chicago", "California", "Columbia", "University", "Verlag", "Birkhäuser", "CUP"
      ]

      strict_keywords.any? { |k| s.include?(k) }
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

      # Remove trailing orphaned opening brackets
      while result.end_with?('(') || result.end_with?('[')
        result = result[0...-1]
      end
      
      result
    end

    def generate_new_filename(metadata, extension)
      parts = []
      parts << "#{metadata.authors} -" if metadata.authors
      parts << metadata.title
      parts << "(#{metadata.year})" if metadata.year
      parts << extension
      parts.join(' ').gsub('  ', ' ').strip.gsub(/ \./, '.')
    end
  end
end

