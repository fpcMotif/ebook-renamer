module EbookRenamer
  FileInfo = Struct.new(
    :original_path,
    :original_name,
    :extension,
    :size,
    :is_failed_download,
    :is_too_small,
    :new_name,
    :new_path,
    keyword_init: true
  )

  ParsedMetadata = Struct.new(
    :authors,
    :title,
    :year,
    keyword_init: true
  )
end

