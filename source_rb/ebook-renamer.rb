#!/usr/bin/env ruby
# frozen_string_literal: true

require_relative 'lib/cli'
require_relative 'lib/scanner'
require_relative 'lib/renamer'

def main
  cli = EbookRenamer::CLI.new
  path, options = cli.parse(ARGV)

  puts "Ebook Renamer (Ruby)"
  puts "Mode: #{options[:dry_run] ? 'DRY RUN' : 'LIVE'}"
  puts "Scanning: #{path}"
  
  scanner = EbookRenamer::Scanner.new
  files = scanner.scan(path, recursive: options[:recursive])
  
  puts "Found #{files.size} files."
  
  renamer = EbookRenamer::Renamer.new(options)
  renamer.process(files)
  
  puts "Done."
end

if __FILE__ == $PROGRAM_NAME
  main
end
