require 'find'
require_relative 'types'

module EbookRenamer
  class Scanner
    EXTENSIONS = %w[.pdf .epub .mobi .azw3 .djvu .txt]

    def scan(path, recursive: true)
      files = []

      if File.file?(path)
        process_file(path, files)
      elsif File.directory?(path)
        if recursive
          Find.find(path) do |f|
            process_file(f, files)
          end
        else
          Dir.glob(File.join(path, "*")) do |f|
            process_file(f, files)
          end
        end
      end

      files
    end

    private

    def process_file(path, files)
      return unless File.file?(path)

      ext = File.extname(path)
      return unless EXTENSIONS.include?(ext.downcase)

      name = File.basename(path)
      size = File.size(path)

      is_failed = name.end_with?('.download') || name.end_with?('.crdownload')
      is_small = size < 1024 * 5 # 5KB, simplistic check

      files << FileInfo.new(
        original_path: path,
        original_name: name,
        extension: ext,
        size: size,
        is_failed_download: is_failed,
        is_too_small: is_small
      )
    end
  end
end
