import os
import tempfile
import time
import unittest
from pathlib import Path

from ebook_renamer.scanner import Scanner


class TestScanner(unittest.TestCase):
    def setUp(self):
        self.test_dir = tempfile.TemporaryDirectory()
        self.root_path = self.test_dir.name

    def tearDown(self):
        self.test_dir.cleanup()

    def create_file(self, filename, content="content"):
        path = os.path.join(self.root_path, filename)
        with open(path, "w") as f:
            f.write(content)
        return path

    def test_scanner_creates_correct_file_info(self):
        # Create content larger than 1KB
        large_content = "This is a test file that is definitely larger than 1KB. " * 50
        self.create_file("test_book.pdf", large_content)

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        file_info = files[0]
        self.assertEqual(file_info.original_name, "test_book.pdf")
        self.assertEqual(file_info.extension, ".pdf")
        self.assertFalse(file_info.is_failed_download)
        self.assertFalse(file_info.is_too_small)

    def test_scanner_detects_tar_gz(self):
        self.create_file("arXiv-2012.08669v1.tar.gz")

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertEqual(files[0].extension, ".tar.gz")

    def test_scanner_detects_download_files(self):
        self.create_file("test_book.pdf.download", "")

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertTrue(files[0].is_failed_download)

    def test_scanner_detects_small_files(self):
        self.create_file("tiny.pdf", "x")  # 1 byte

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertTrue(files[0].is_too_small)

    def test_scanner_skips_hidden_files(self):
        self.create_file(".hidden.pdf")

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 0)

    def test_scanner_skips_download_directories(self):
        download_dir = os.path.join(self.root_path, "some_book.download")
        os.mkdir(download_dir)
        with open(os.path.join(download_dir, "content.pdf"), "w") as f:
            f.write("content")

        scanner = Scanner(self.root_path, 2)
        files = scanner.scan()

        self.assertEqual(len(files), 0)

    # ========== EDGE CASE TESTS ==========

    def test_scanner_detects_crdownload_files(self):
        """Chrome partial downloads use .crdownload extension"""
        self.create_file("test_book.pdf.crdownload", "partial content")

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertTrue(files[0].is_failed_download)
        self.assertEqual(files[0].extension, ".crdownload")

    def test_scanner_empty_directory(self):
        """Empty directory should return no files"""
        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 0)

    def test_scanner_multiple_file_types(self):
        large_content = "x" * 2000

        self.create_file("book.pdf", large_content)
        self.create_file("book.epub", large_content)
        self.create_file("notes.txt", large_content)
        self.create_file("archive.tar.gz", large_content)
        self.create_file("document.mobi", large_content)

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 5)

        extensions = [f.extension for f in files]
        self.assertIn(".pdf", extensions)
        self.assertIn(".epub", extensions)
        self.assertIn(".txt", extensions)
        self.assertIn(".tar.gz", extensions)
        self.assertIn(".mobi", extensions)

    def test_scanner_nested_directories(self):
        """Test that scanner can find files in nested directories"""
        large_content = "x" * 2000

        # Create file at depth 1
        self.create_file("book1.pdf", large_content)

        # Create file at depth 2
        subdir = os.path.join(self.root_path, "subdir")
        os.mkdir(subdir)
        with open(os.path.join(subdir, "book2.pdf"), "w") as f:
            f.write(large_content)

        # Create file at depth 3
        subsubdir = os.path.join(subdir, "subsubdir")
        os.mkdir(subsubdir)
        with open(os.path.join(subsubdir, "book3.pdf"), "w") as f:
            f.write(large_content)

        # With sufficient depth, should find all files
        scanner = Scanner(self.root_path, 10)
        files = scanner.scan()
        self.assertEqual(len(files), 3)

    def test_scanner_epub_small_file_detection(self):
        self.create_file("tiny.epub", "x")

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertTrue(files[0].is_too_small)

    def test_scanner_size_threshold_exactly_1kb(self):
        # Exactly 1024 bytes should NOT be too small
        self.create_file("exact_1kb.pdf", "x" * 1024)

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertFalse(files[0].is_too_small)

    def test_scanner_size_threshold_1023_bytes(self):
        # 1023 bytes should be too small
        self.create_file("under_1kb.pdf", "x" * 1023)

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertTrue(files[0].is_too_small)

    def test_scanner_file_without_extension(self):
        self.create_file("README", "content")

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertEqual(files[0].extension, "")
        self.assertEqual(files[0].original_name, "README")

    def test_scanner_unicode_filename(self):
        large_content = "x" * 2000
        self.create_file("数学入門.pdf", large_content)

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertEqual(files[0].original_name, "数学入門.pdf")
        self.assertEqual(files[0].extension, ".pdf")

    def test_scanner_file_size_recorded(self):
        content = "Hello, World!"  # 13 bytes
        self.create_file("small.txt", content)

        scanner = Scanner(self.root_path, 1)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertEqual(files[0].size, 13)

    def test_scanner_preserves_relative_path_structure(self):
        large_content = "x" * 2000
        subdir = os.path.join(self.root_path, "books", "2024")
        os.makedirs(subdir)
        with open(os.path.join(subdir, "book.pdf"), "w") as f:
            f.write(large_content)

        scanner = Scanner(self.root_path, 10)
        files = scanner.scan()

        self.assertEqual(len(files), 1)
        self.assertIn("books", str(files[0].original_path))
        self.assertIn("2024", str(files[0].original_path))
