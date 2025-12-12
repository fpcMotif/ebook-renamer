require 'fileutils'
require 'set'
require_relative 'normalizer'

module EbookRenamer
  class Renamer
    def initialize(options)
      @options = options
      @normalizer = Normalizer.new
      @seen_paths = Set.new
    end

    def process(files)
      normalized_files = @normalizer.normalize_files(files)

      # Handle duplicates (basic version: append counter)
      final_files = handle_collisions(normalized_files)

      # Execute renames
      final_files.each do |file|
        next unless file.new_path
        next if file.original_path == file.new_path

        if @options[:dry_run]
          puts "[DRY-RUN] #{file.original_name} -> #{file.new_name}"
        else
          begin
            # Ensure target directory exists (though usually we stay in same dir)
            FileUtils.mkdir_p(File.dirname(file.new_path))
            FileUtils.mv(file.original_path, file.new_path)
            puts "Renamed: #{file.original_name} -> #{file.new_name}"
          rescue StandardError => e
            puts "Error renaming #{file.original_name}: #{e.message}"
          end
        end
      end
    end

    private

    def handle_collisions(files)
      # Map new_path -> list of files
      path_map = Hash.new { |h, k| h[k] = [] }

      files.each do |file|
        next unless file.new_path
        path_map[file.new_path] << file
      end

      path_map.each do |path, collisions|
        next if collisions.size < 2

        # Sort collisions to have deterministic order (e.g., by size desc, or original name)
        # Here we just keep them all but rename duplicates
        collisions.each_with_index do |file, idx|
          next if idx == 0 # Keep first one as is

          # Append (n) to filename before extension
          ext = file.extension
          base = File.basename(file.new_path, ext)
          dir = File.dirname(file.new_path)

          new_name = "#{base} (#{idx})#{ext}"
          file.new_name = new_name
          file.new_path = File.join(dir, new_name)
        end
      end

      files
    end
  end
end
