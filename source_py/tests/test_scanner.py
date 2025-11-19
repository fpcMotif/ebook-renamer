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
