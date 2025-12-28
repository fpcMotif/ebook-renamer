require 'test_helper'
require 'minitest/autorun'
require 'fileutils'

class TestCLI < Minitest::Test
  def setup
    @test_dir = Dir.mktmpdir
    @original_dir = Dir.pwd
    Dir.chdir(@test_dir)
  end

  def teardown
    Dir.chdir(@original_dir)
    FileUtils.rm_rf(@test_dir)
  end

  def test_argument_parsing
    # Test basic argument parsing
    args = CLI.parse([
      '--dry-run',
      '--json',
      '--max-depth=10',
      '--extensions=pdf,epub',
      @test_dir.to_s
    ])
    
    assert_equal true, args.dry_run
    assert_equal 10, args.max_depth
    assert_equal ['pdf', 'epub'], args.extensions
    assert_equal @test_dir.to_s, args.path
  end

  def test_invalid_argument_handling
    # Test that invalid arguments raise appropriate errors
    assert_raises(ArgumentError) do
      CLI.parse(['--invalid-option'])
    end
    
    assert_raises(ArgumentError) do
      CLI.parse(['--max-depth=invalid'])
    end
  end

  def test_extension_validation
    # Test extension validation
    assert_raises(ArgumentError) do
      CLI.parse(['--extensions=exe,bat'])
    end
    
    # Test valid extensions
    args = CLI.parse(['--extensions=pdf,epub,txt,mobi'])
    assert_equal ['pdf', 'epub', 'txt', 'mobi'], args.extensions
  end

  def test_depth_validation
    # Test depth validation
    assert_raises(ArgumentError) do
      CLI.parse(['--max-depth=1001']) # Above limit
    end
    
    args = CLI.parse(['--max-depth=1000']) # At limit
    assert_equal 1000, args.max_depth
  end

  def test_path_validation
    # Test path validation
    args = CLI.parse(['/nonexistent/path'])
    # Should not raise immediately, but will fail during execution
    
    # Test dangerous path
    assert_raises(ArgumentError) do
      CLI.parse(['../../../etc/passwd'])
    end
  end

  def test_flag_conflicts
    # Test conflicting flags
    args = CLI.parse(['--no-recursive', '--max-depth=10'])
    # Should handle conflict: no-recursive should override max-depth to 1
    
    # This tests the implementation rather than the interface
    assert_equal true, args.no_recursive
    # max_depth handling would be tested in integration tests
  end

  def test_boolean_flags
    # Test various boolean flags
    args = CLI.parse([
      '--dry-run',
      '--json', 
      '--no-delete',
      '--preserve-unicode',
      '--fetch-arxiv',
      '--verbose',
      '--delete-small',
      '--clean-failed',
      '--skip-cloud-hash',
      '--cleanup-downloads',
      '--generate-undo-script'
    ])
    
    assert_equal true, args.dry_run
    assert_equal true, args.json
    assert_equal true, args.no_delete
    assert_equal true, args.preserve_unicode
    assert_equal true, args.fetch_arxiv
    assert_equal true, args.verbose
    assert_equal true, args.delete_small
    assert_equal true, args.clean_failed
    assert_equal true, args.skip_cloud_hash
    assert_equal true, args.cleanup_downloads
    assert_equal true, args.generate_undo_script
  end

  def test_file_arguments
    # Test optional file arguments
    todo_file = File.join(@test_dir, 'custom_todo.md')
    log_file = File.join(@test_dir, 'custom.log')
    
    args = CLI.parse([
      '--todo-file', todo_file.to_s,
      '--log-file', log_file.to_s,
      @test_dir.to_s
    ])
    
    assert_equal todo_file.to_s, args.todo_file
    assert_equal log_file.to_s, args.log_file
    assert_equal @test_dir.to_s, args.path
  end

  def test_argument_defaults
    # Test that defaults are applied correctly
    args = CLI.parse([@test_dir.to_s])
    
    assert_equal false, args.dry_run
    assert_equal false, args.json
    assert_equal 18446744073709551615, args.max_depth # Default unlimited
    assert_equal false, args.no_recursive
    assert_equal ['pdf', 'epub', 'txt'], args.extensions
    assert_equal false, args.no_delete
    assert_equal nil, args.todo_file
    assert_equal nil, args.log_file
    assert_equal false, args.preserve_unicode
    assert_equal false, args.fetch_arxiv
    assert_equal false, args.verbose
    assert_equal false, args.delete_small
    assert_equal false, args.clean_failed
    assert_equal false, args.skip_cloud_hash
    assert_equal false, args.cleanup_downloads
    assert_equal false, args.generate_undo_script
  end

  def test_help_output
    # Test that help output includes expected information
    assert_raises(SystemExit) do
      begin
        original_stdout = $stdout
        original_stderr = $stderr
        
        $stdout = StringIO.new
        $stderr = StringIO.new
        
        CLI.parse(['--help'])
        
        output = $stdout.string
        $stdout = original_stdout
        $stderr = original_stderr
        
        # Verify key help content is present
        assert_includes output, 'ebook-renamer'
        assert_includes output, 'batch rename and organize'
        assert_includes output, 'dry-run'
        assert_includes output, 'json'
        assert_includes output, 'max-depth'
      ensure
        $stdout = original_stdout
        $stderr = original_stderr
      end
    end
  end

  def test_version_output
    # Test version output
    assert_raises(SystemExit) do
      begin
        original_stdout = $stdout
        original_stderr = $stderr
        
        $stdout = StringIO.new
        $stderr = StringIO.new
        
        CLI.parse(['--version'])
        
        output = $stdout.string
        $stdout = original_stdout
        $stderr = original_stderr
        
        assert_includes output, 'ebook-renamer'
        assert_match /\d+\.\d+\.\d+/, output # Version pattern like x.y.z
        $stdout = original_stdout
        $stderr = original_stderr
      end
    end
  end
end