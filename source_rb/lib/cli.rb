require 'optparse'

module EbookRenamer
  class CLI
    attr_reader :options

    def initialize
      @options = {
        dry_run: false,
        verbose: false,
        recursive: true, # default to recursive
        mode: :standard
      }
    end

    def parse(args)
      parser = OptionParser.new do |opts|
        opts.banner = "Usage: ebook-renamer.rb [options] [path]"

        opts.on("-n", "--dry-run", "Show what would be renamed without renaming") do
          @options[:dry_run] = true
        end

        opts.on("-v", "--verbose", "Show verbose output") do
          @options[:verbose] = true
        end

        opts.on("--no-recursive", "Do not process directories recursively") do
          @options[:recursive] = false
        end

        opts.on("-h", "--help", "Show this help message") do
          puts opts
          exit
        end
      end

      remaining = parser.parse(args)
      path = remaining.first || '.'

      [path, @options]
    end
  end
end
