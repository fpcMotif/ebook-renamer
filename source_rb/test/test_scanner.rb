require 'test_helper'
require 'minitest/autorun'
require 'fileutils'
require 'tempfile'

class TestScanner < Minitest::Test
  def setup
    @test_dir = Dir.mktmpdir
    @scanner = Scanner.new(@test_dir, 5)
  end

  def teardown
    FileUtils.rm_rf(@test_dir)
  end

  def test_scan_pdf_files
    File.write(File.join(@test_dir, "test.pdf"), "%PDF-1.4 content")
    File.write(File.join(@test_dir, "book.epub"), "epub content")
    File.write(File.join(@test_dir, "notes.txt"), "text content")
    
    files = @scanner.scan
    
    assert_equal 3, files.length
    assert_includes files.map { |f| f[:original_name] }, "test.pdf"
    assert_includes files.map { |f| f[:original_name] }, "book.epub"
    assert_includes files.map { |f| f[:original_name] }, "notes.txt"
  end

  def test_skip_hidden_files
    File.write(File.join(@test_dir, ".hidden.pdf"), "hidden content")
    File.write(File.join(@test_dir, "visible.pdf"), "visible content")
    
    files = @scanner.scan
    
    assert_equal 1, files.length
    assert_includes files.map { |f| f[:original_name] }, "visible.pdf"
    refute_includes files.map { |f| f[:original_name] }, ".hidden.pdf"
  end

  def test_max_depth_limit
    FileUtils.mkdir_p(File.join(@test_dir, "level1"))
    FileUtils.mkdir_p(File.join(@test_dir, "level1", "level2"))
    FileUtils.mkdir_p(File.join(@test_dir, "level1", "level2", "level3"))
    
    File.write(File.join(@test_dir, "root.pdf"), "root level")
    File.write(File.join(@test_dir, "level1", "file1.pdf"), "level 1")
    File.write(File.join(@test_dir, "level1", "level2", "file2.pdf"), "level 2")
    File.write(File.join(@test_dir, "level1", "level2", "level3", "file3.pdf"), "level 3")
    
    # Test with max depth 2
    scanner = Scanner.new(@test_dir, 2)
    files = scanner.scan
    
    assert_equal 3, files.length
    assert_includes files.map { |f| f[:original_name] }, "root.pdf"
    assert_includes files.map { |f| f[:original_name] }, "file1.pdf"
    assert_includes files.map { |f| f[:original_name] }, "file2.pdf"
    refute_includes files.map { |f| f[:original_name] }, "file3.pdf"
  end

  def test_detect_file_extensions
    File.write(File.join(@test_dir, "book.pdf"), "pdf content")
    File.write(File.join(@test_dir, "document.epub"), "epub content")
    File.write(File.join(@test_dir, "notes.txt"), "text content")
    File.write(File.join(@test_dir, "archive.tar.gz"), "archive content")
    File.write(File.join(@test_dir, "incomplete.download"), "download content")
    
    files = @scanner.scan
    
    assert_equal 6, files.length
    
    file_extensions = files.map { |f| f[:extension] }
    assert_includes file_extensions, ".pdf"
    assert_includes file_extensions, ".epub"
    assert_includes file_extensions, ".txt"
    assert_includes file_extensions, ".tar.gz"
    assert_includes file_extensions, ".download"
  end

  def test_file_size_detection
    # Large PDF (should not be too small)
    large_content = "x" * 2000  # ~2KB
    File.write(File.join(@test_dir, "large.pdf"), large_content)
    
    # Small PDF (should be too small)
    File.write(File.join(@test_dir, "small.pdf"), "x")  # 1 byte
    
    # TXT file (should not be checked for size)
    File.write(File.join(@test_dir, "notes.txt"), "x")
    
    files = @scanner.scan
    
    pdf_files = files.select { |f| f[:extension] == ".pdf" }
    small_files = pdf_files.select { |f| f[:is_too_small] }
    txt_files = files.select { |f| f[:extension] == ".txt" }
    
    assert_equal 2, pdf_files.length
    assert_equal 1, small_files.length
    assert_equal 1, small_files.first[:original_name], "small.pdf"
    assert_equal 0, txt_files.select { |f| f[:is_too_small] }.length
  end

  def test_failed_download_detection
    File.write(File.join(@test_dir, "book.pdf"), "valid pdf")
    File.write(File.join(@test_dir, "incomplete.download"), "partial download")
    File.write(File.join(@test_dir, "stuck.crdownload"), "chrome download")
    
    files = @scanner.scan
    
    download_files = files.select { |f| f[:is_failed_download] }
    
    assert_equal 2, download_files.length
    download_names = download_files.map { |f| f[:original_name] }
    assert_includes download_names, "incomplete.download"
    assert_includes download_names, "stuck.crdownload"
  end

  def test_nested_directory_scanning
    FileUtils.mkdir_p(File.join(@test_dir, "subdir1", "subdir2"))
    File.write(File.join(@test_dir, "subdir1", "file1.pdf"), "content 1")
    File.write(File.join(@test_dir, "subdir2", "file2.pdf"), "content 2")
    
    files = @scanner.scan
    
    assert_equal 2, files.length
    assert_includes files.map { |f| f[:original_name] }, "file1.pdf"
    assert_includes files.map { |f| f[:original_name] }, "file2.pdf"
  end

  def test_invalid_path_handling
    # Test that scanner handles various path issues gracefully
    assert_raises(StandardError) do
      Scanner.new("/nonexistent/path/that/does/not/exist", 1)
    end
    
    assert_raises(StandardError) do
      Scanner.new(nil, 1)
    end
  end

  def test_unicode_filenames
    # Test with Unicode filenames
    File.write(File.join(@test_dir, "数学入門.pdf"), "unicode content")
    File.write(File.join(@test_dir, "éléphant.pdf"), "accented content")
    
    files = @scanner.scan
    
    assert_equal 2, files.length
    unicode_names = files.map { |f| f[:original_name] }
    assert_includes unicode_names, "数学入門.pdf"
    assert_includes unicode_names, "éléphant.pdf"
  end

  def test_symlink_handling
    # Create a symlink and test if scanner handles it safely
    real_file = File.join(@test_dir, "real.pdf")
    File.write(real_file, "real content")
    
    symlink_file = File.join(@test_dir, "symlink.pdf")
    begin
      File.symlink(real_file, symlink_file)
    rescue NotImplementedError
      skip "Symlinks not supported on this platform"
    end
    
    files = @scanner.scan
    
    # Should either skip or resolve symlinks to real files
    if File.symlink?(symlink_file)
      # This test ensures we don't count symlinks twice
      real_files = files.select { |f| f[:original_name] == "real.pdf" }
      symlink_files = files.select { |f| f[:original_name] == "symlink.pdf" }
      
      assert_equal 1, real_files.length
      assert_operator :<=, symlink_files.length, 1
    end
  end

  def test_file_info_structure
    File.write(File.join(@test_dir, "test.pdf"), "pdf content")
    
    files = @scanner.scan
    file_info = files.first
    
    # Ensure file info has expected structure
    assert_respond_to :original_name, file_info
    assert_respond_to :extension, file_info
    assert_respond_to :size, file_info
    assert_respond_to :original_path, file_info
    assert_respond_to :is_too_small, file_info
    assert_respond_to :is_failed_download, file_info
  end

  def test_empty_directory
    # Test scanning empty directory
    files = @scanner.scan
    
    assert_equal [], files
  end

  def test_permissions_error_handling
    # Test that scanner handles permission errors gracefully
    begin
      # Create a directory we can't read
      restricted_dir = File.join(@test_dir, "restricted")
      FileUtils.mkdir_p(restricted_dir)
      
      # Remove read permissions
      FileUtils.chmod(0000, restricted_dir)
      
      files = @scanner.scan
      
      # Should skip or handle the restricted directory
      assert files.all? { |f| !f[:original_path].include?(restricted_dir) }
    ensure
      # Restore permissions for cleanup
      FileUtils.chmod(0755, restricted_dir)
    end
  end

  private

  def assert_respond_to(method, file_info)
    assert_respond_to method, file_info, "Expected to respond to #{method}"
  end
end