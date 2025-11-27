import unittest
from unittest.mock import MagicMock, patch
import os
import sys

# Ensure module can be imported
sys.path.append(os.path.join(os.path.dirname(__file__), '..'))

from ebook_renamer.types import Config
from ebook_renamer import tui

class TestTUI(unittest.TestCase):

    @patch('ebook_renamer.tui.Console')
    @patch('ebook_renamer.tui.Progress')
    @patch('ebook_renamer.tui.Scanner')
    @patch('ebook_renamer.tui.Normalizer')
    @patch('ebook_renamer.tui.TodoList')
    @patch('ebook_renamer.tui.DuplicateDetector')
    @patch('os.rename')
    @patch('os.remove')
    def test_run_tui_dry_run(self, mock_remove, mock_rename, mock_dup, mock_todo, mock_norm, mock_scan, mock_progress, mock_console):
        config = Config(
            path=".",
            dry_run=True,
            max_depth=5,
            no_recursive=False,
            extensions=[".pdf"],
            no_delete=False,
            todo_file=None,
            log_file=None,
            preserve_unicode=False,
            fetch_arxiv=False,
            verbose=False,
            delete_small=False,
            auto_cleanup=False,
            json=False
        )

        # Setup mocks
        mock_scanner_instance = mock_scan.return_value
        mock_scanner_instance.scan.return_value = []

        mock_norm_instance = mock_norm.return_value
        mock_norm_instance.normalize_files.return_value = []

        mock_dup_instance = mock_dup.return_value
        mock_dup_instance.detect_duplicates.return_value = ([], [])

        # Run TUI
        ret = tui.run_tui(config)

        self.assertEqual(ret, 0)

        # Verify calls
        mock_console.return_value.print.assert_called()
        mock_scan.assert_called_with(".", config.max_depth)
        mock_norm.assert_called()
        mock_todo.assert_called()
        mock_dup.assert_called()

        # Ensure no file operations
        mock_rename.assert_not_called()
        mock_remove.assert_not_called()

    @patch('ebook_renamer.tui.Console')
    @patch('ebook_renamer.tui.Progress')
    @patch('ebook_renamer.tui.Scanner')
    @patch('ebook_renamer.tui.Normalizer')
    @patch('ebook_renamer.tui.TodoList')
    @patch('ebook_renamer.tui.DuplicateDetector')
    @patch('os.rename')
    @patch('os.remove')
    def test_run_tui_execute(self, mock_remove, mock_rename, mock_dup, mock_todo, mock_norm, mock_scan, mock_progress, mock_console):
        config = Config(
            path=".",
            dry_run=False,
            max_depth=5,
            no_recursive=False,
            extensions=[".pdf"],
            no_delete=False,
            todo_file=None,
            log_file=None,
            preserve_unicode=False,
            fetch_arxiv=False,
            verbose=False,
            delete_small=False,
            auto_cleanup=False,
            json=False
        )

        # Setup mocks
        mock_file = MagicMock()
        mock_file.new_name = "new.pdf"
        mock_file.original_path = "old.pdf"
        mock_file.new_path = "new.pdf"

        mock_scanner_instance = mock_scan.return_value
        mock_scanner_instance.scan.return_value = [mock_file]

        mock_norm_instance = mock_norm.return_value
        mock_norm_instance.normalize_files.return_value = [mock_file]

        mock_dup_instance = mock_dup.return_value
        mock_dup_instance.detect_duplicates.return_value = ([], [mock_file])

        # Run TUI
        ret = tui.run_tui(config)

        self.assertEqual(ret, 0)

        # Verify rename called
        mock_rename.assert_called_with("old.pdf", "new.pdf")

if __name__ == '__main__':
    unittest.main()
