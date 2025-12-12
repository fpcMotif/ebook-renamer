# This is a dummy test file to ensure structure is correct.
# Real tests cannot be run as Ruby is not installed.

require 'minitest/autorun'
require_relative '../lib/normalizer'
require_relative '../lib/types'

class TestNormalizer < Minitest::Test
  def setup
    @normalizer = EbookRenamer::Normalizer.new
  end

  def test_basic_normalization
    filename = "Differential Geometry (Paulo Ventura Araujo).pdf"
    ext = ".pdf"

    metadata = @normalizer.parse_filename(filename, ext)
    assert_equal "Paulo Ventura Araujo", metadata.authors
    assert_equal "Differential Geometry", metadata.title

    new_name = @normalizer.generate_new_filename(metadata, ext)
    assert_equal "Paulo Ventura Araujo - Differential Geometry.pdf", new_name
  end

  def test_series_extraction
    filename = "Categories for the Working Mathematician [GTM 52] (1978).pdf"
    ext = ".pdf"

    metadata = @normalizer.parse_filename(filename, ext)
    assert_equal "GTM 52", metadata.series
    assert_equal 1978, metadata.year
  end
end
