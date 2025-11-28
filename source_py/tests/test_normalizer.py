import unittest
from ebook_renamer.normalizer import Normalizer
from ebook_renamer.types import FileInfo

class TestNormalizer(unittest.TestCase):
    def setUp(self):
        self.normalizer = Normalizer()

    def test_parse_simple_filename(self):
        metadata = self.normalizer._parse_filename("John Smith - Sample Book Title.pdf", ".pdf")
        self.assertEqual(metadata.authors, "John Smith")
        self.assertEqual(metadata.title, "Sample Book Title")

    def test_parse_with_year(self):
        metadata = self.normalizer._parse_filename("Jane Doe - Another Title (2020, Publisher).pdf", ".pdf")
        self.assertEqual(metadata.authors, "Jane Doe")
        self.assertEqual(metadata.year, 2020)

    def test_parse_with_series_prefix(self):
        metadata = self.normalizer._parse_filename(
            "B. R. Tennison - Sheaf Theory (1976).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "B. R. Tennison")
        self.assertEqual(metadata.title, "Sheaf Theory")
        self.assertEqual(metadata.year, 1976)

    def test_clean_underscores(self):
        result = self.normalizer._clean_orphaned_brackets("Sample_Title_With_Underscores")
        self.assertEqual(result, "Sample Title With Underscores")

    def test_clean_orphaned_brackets(self):
        result = self.normalizer._clean_orphaned_brackets("Title ) with ( orphaned ) brackets [")
        # Should not have orphaned closing paren " ) "
        # The valid one is "( orphaned )".
        # The result should be cleaned of orphaned brackets.
        # "Title  with ( orphaned ) brackets " -> cleaned to single spaces
        self.assertEqual(result, "Title with ( orphaned ) brackets")

    def test_parse_author_before_title_with_publisher(self):
        metadata = self.normalizer._parse_filename(
            "Ernst Kunz, Richard G. Belshoff - Introduction to Plane Algebraic Curves (2005, Birkhäuser) - libgen.li.pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Ernst Kunz, Richard G. Belshoff")
        self.assertEqual(metadata.title, "Introduction to Plane Algebraic Curves")
        self.assertEqual(metadata.year, 2005)

    def test_parse_z_library_variant(self):
        metadata = self.normalizer._parse_filename(
            "Daniel Huybrechts - Fourier-Mukai transforms in algebraic geometry (z-Library).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Daniel Huybrechts")
        self.assertEqual(metadata.title, "Fourier-Mukai transforms in algebraic geometry")
        self.assertIsNone(metadata.year)

    def test_clean_parentheticals_with_publisher(self):
        result = self.normalizer._clean_parentheticals("Title (2005, Birkhäuser) - libgen.li", 2005)
        self.assertIn("Title", result)
        self.assertNotIn("2005", result)
        self.assertNotIn("Birkhäuser", result)

    def test_clean_title_comprehensive_sources(self):
        test_cases = [
            ("Title - libgen.li", "Title"),
            ("Title - Z-Library", "Title"),
            ("Title - z-Library", "Title"),
            ("Title (libgen.li)", "Title"),
            ("Title libgen.li.pdf", "Title"),
            ("Title Z-Library.pdf", "Title"),
            ("Title", "Title"),
            ("Title (auth.)", "Title"),
            ("Title with  double  spaces", "Title with double spaces"),
            ("Title -", "Title"),
            ("Title :", "Title"),
            ("Title ;", "Title"),
        ]

        for input_str, expected in test_cases:
            result = self.normalizer._clean_title(input_str)
            self.assertEqual(result, expected, f"Input: {input_str}")

    def test_multi_author_with_commas(self):
        metadata = self.normalizer._parse_filename(
            "Lectures on harmonic analysis (Thomas H. Wolff, Izabella Aba, Carol Shubin).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Thomas H. Wolff, Izabella Aba, Carol Shubin")
        self.assertEqual(metadata.title, "Lectures on harmonic analysis")

    def test_single_word_comma_removal(self):
        metadata = self.normalizer._parse_filename(
            "Higher Dimensional Categories From Double To Multiple Categories (Marco, Grandis).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Marco Grandis")

    def test_lecture_notes_removal(self):
        metadata = self.normalizer._parse_filename(
            "Introduction to Category Theory and Categorical Logic [Lecture notes] (Thomas Streicher).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Thomas Streicher")
        self.assertEqual(metadata.title, "Introduction to Category Theory and Categorical Logic")
        self.assertNotIn("lecture", metadata.title.lower())

    def test_trailing_id_noise_removal(self):
        metadata = self.normalizer._parse_filename(
            "Math History A Long-Form Mathematics Textbook (The Long-Form Math Textbook Series)-B0F5TFL6ZQ.pdf",
            ".pdf"
        )
        self.assertEqual(metadata.title, "Math History A Long-Form Mathematics Textbook")
        self.assertNotIn("B0F5TFL6ZQ", metadata.title)
        self.assertNotIn("Series", metadata.title)

    def test_cjk_author_detection(self):
        metadata = self.normalizer._parse_filename(
            "文革时期中国农村的集体杀戮 Collective Killings in Rural China during the Cultural Revolution (苏阳).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "苏阳")
        self.assertIn("文革时期中国农村的集体杀戮", metadata.title)

    def test_nested_publisher_removal(self):
        metadata = self.normalizer._parse_filename(
            "Theory of Categories (Pure and Applied Mathematics (Academic Press)) (Barry Mitchell).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Barry Mitchell")
        self.assertEqual(metadata.title, "Theory of Categories")
        self.assertNotIn("Pure", metadata.title)
        self.assertNotIn("Academic", metadata.title)

    def test_deadly_decision_beijing(self):
        metadata = self.normalizer._parse_filename(
            "Deadly Decision in Beijing. Succession Politics, Protest Repression, and the 1989 Tiananmen Massacre (Yang Su).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Yang Su")
        self.assertIn("Deadly Decision", metadata.title)

    def test_tools_for_pde(self):
        metadata = self.normalizer._parse_filename(
            "Tools for PDE Pseudodifferential Operators, Paradifferential Operators, and Layer Potentials (Michael E. Taylor).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Michael E. Taylor")
        self.assertIn("Tools for PDE", metadata.title)

    def test_quantum_cohomology(self):
        metadata = self.normalizer._parse_filename(
            "From Quantum Cohomology to Integrable Systems (Martin A. Guest).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Martin A. Guest")
        self.assertEqual(metadata.title, "From Quantum Cohomology to Integrable Systems")

    def test_kashiwara(self):
        metadata = self.normalizer._parse_filename(
            "Bases cristallines des groupes quantiques (Masaki Kashiwara).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Masaki Kashiwara")
        self.assertIn("Bases cristallines", metadata.title)

    def test_wavelets_with_multiple_authors_and_z_library(self):
        metadata = self.normalizer._parse_filename(
            "Wavelets and their applications (Michel Misiti, Yves Misiti, Georges Oppenheim etc.) (Z-Library).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Michel Misiti, Yves Misiti, Georges Oppenheim etc.")
        self.assertEqual(metadata.title, "Wavelets and their applications")
        self.assertNotIn("Z-Library", metadata.title)

    def test_systems_of_microdifferential_with_hash(self):
        metadata = self.normalizer._parse_filename(
            "Masaki Kashiwara - Systems of microdifferential equations -- 9780817631383 -- b3ab25f14db594eb0188171e0dd81250 -- Anna's Archive.pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Masaki Kashiwara")
        self.assertEqual(metadata.title, "Systems of microdifferential equations")
        self.assertNotIn("9780817631383", metadata.title)
        self.assertNotIn("b3ab25f14db594eb0188171e0dd81250", metadata.title)
        self.assertNotIn("Anna's Archive", metadata.title)

    def test_mani_mehra_wavelets(self):
        metadata = self.normalizer._parse_filename(
            "Wavelets Theory and Its Applications A First Course (Mani Mehra) (Z-Library).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Mani Mehra")
        self.assertEqual(metadata.title, "Wavelets Theory and Its Applications A First Course")
        self.assertNotIn("Z-Library", metadata.title)

    def test_graduate_texts_series_removal(self):
        metadata = self.normalizer._parse_filename(
            "Graduate Texts in Mathematics - Saunders Mac Lane - Categories for the Working Mathematician (1978).pdf",
            ".pdf"
        )
        self.assertEqual(metadata.authors, "Saunders Mac Lane")
        self.assertEqual(metadata.title, "Categories for the Working Mathematician")
        self.assertEqual(metadata.year, 1978)
        self.assertNotIn("Graduate Texts", metadata.title)
