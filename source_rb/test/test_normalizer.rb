require 'minitest/autorun'
require_relative '../lib/normalizer'

class TestNormalizer < Minitest::Test
  def setup
    @normalizer = EbookRenamer::Normalizer.new
  end

  def test_clean_title_comprehensive_sources
    test_cases = [
      ["Title - libgen.li", "Title"],
      ["Title - Z-Library", "Title"],
      ["Title - z-Library", "Title"],
      ["Title (libgen.li)", "Title"],
      ["Title libgen.li.pdf", "Title"],
      ["Title Z-Library.pdf", "Title"],
      ["Title", "Title"],
      ["Title (auth.)", "Title"],
      ["Title with  double  spaces", "Title with double spaces"],
      ["Title -", "Title"],
      ["Title :", "Title"],
      ["Title ;", "Title"]
    ]
    test_cases.each do |input_str, expected|
      assert_equal expected, @normalizer.clean_title(input_str)
    end
  end

  def test_multi_author_with_commas
    metadata = @normalizer.parse_filename(
      "Lectures on harmonic analysis (Thomas H. Wolff, Izabella Aba, Carol Shubin).pdf",
      ".pdf"
    )
    assert_equal "Thomas H. Wolff, Izabella Aba, Carol Shubin", metadata.authors
    assert_equal "Lectures on harmonic analysis", metadata.title
  end

  def test_single_word_comma_removal
    metadata = @normalizer.parse_filename(
      "Higher Dimensional Categories From Double To Multiple Categories (Marco, Grandis).pdf",
      ".pdf"
    )
    assert_equal "Marco Grandis", metadata.authors
  end

  def test_lecture_notes_removal
    metadata = @normalizer.parse_filename(
      "Introduction to Category Theory and Categorical Logic [Lecture notes] (Thomas Streicher).pdf",
      ".pdf"
    )
    assert_equal "Thomas Streicher", metadata.authors
    assert_equal "Introduction to Category Theory and Categorical Logic", metadata.title
  end

  def test_trailing_id_noise_removal
    metadata = @normalizer.parse_filename(
      "Math History A Long-Form Mathematics Textbook (The Long-Form Math Textbook Series)-B0F5TFL6ZQ.pdf",
      ".pdf"
    )
    assert_equal "Math History A Long-Form Mathematics Textbook", metadata.title
  end

  def test_cjk_author_detection
    metadata = @normalizer.parse_filename(
      "文革时期中国农村的集体杀戮 Collective Killings in Rural China during the Cultural Revolution (苏阳).pdf",
      ".pdf"
    )
    assert_equal "苏阳", metadata.authors
    assert_includes metadata.title, "文革时期中国农村的集体杀戮"
  end

  def test_nested_publisher_removal
    metadata = @normalizer.parse_filename(
      "Theory of Categories (Pure and Applied Mathematics (Academic Press)) (Barry Mitchell).pdf",
      ".pdf"
    )
    assert_equal "Barry Mitchell", metadata.authors
    assert_equal "Theory of Categories", metadata.title
  end

  def test_deadly_decision_beijing
    metadata = @normalizer.parse_filename(
      "Deadly Decision in Beijing. Succession Politics, Protest Repression, and the 1989 Tiananmen Massacre (Yang Su).pdf",
      ".pdf"
    )
    assert_equal "Yang Su", metadata.authors
    assert_includes metadata.title, "Deadly Decision"
  end

  def test_tools_for_pde
    metadata = @normalizer.parse_filename(
      "Tools for PDE Pseudodifferential Operators, Paradifferential Operators, and Layer Potentials (Michael E. Taylor).pdf",
      ".pdf"
    )
    assert_equal "Michael E. Taylor", metadata.authors
    assert_includes metadata.title, "Tools for PDE"
  end

  def test_quantum_cohomology
    metadata = @normalizer.parse_filename(
      "From Quantum Cohomology to Integrable Systems (Martin A. Guest).pdf",
      ".pdf"
    )
    assert_equal "Martin A. Guest", metadata.authors
    assert_equal "From Quantum Cohomology to Integrable Systems", metadata.title
  end

  def test_kashiwara
    metadata = @normalizer.parse_filename(
      "Bases cristallines des groupes quantiques (Masaki Kashiwara).pdf",
      ".pdf"
    )
    assert_equal "Masaki Kashiwara", metadata.authors
    assert_includes metadata.title, "Bases cristallines"
  end
end

