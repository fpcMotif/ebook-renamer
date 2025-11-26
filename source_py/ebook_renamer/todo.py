"""
Todo list management for the ebook renamer.
"""

import os
import re
from datetime import datetime
from typing import List, Set

from .types import FileInfo, FileIssue


class TodoList:
    """Manages todo items and file issues."""
    
    def __init__(self, todo_file_path: str, target_dir: str):
        self.todo_file_path = todo_file_path
        self.target_dir = target_dir
        self.items: List[str] = []
        self.failed_downloads: List[str] = []
        self.small_files: List[str] = []
        self.corrupted_files: List[str] = []
        self.other_issues: List[str] = []
        
        # Try to read existing todo.md to avoid duplicates
        if os.path.exists(todo_file_path):
            try:
                with open(todo_file_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                self.items = self._extract_items_from_md(content)
            except (OSError, IOError):
                pass
    
    def add_file_issue(self, file_info: FileInfo, issue: FileIssue) -> None:
        """Add a file issue to the todo list."""
        if issue == FileIssue.FAILED_DOWNLOAD:
            item = f"重新下载: {file_info.original_name} (未完成下载)"
        elif issue == FileIssue.TOO_SMALL:
            item = f"检查并重新下载: {file_info.original_name} (文件过小，仅 {file_info.size} 字节)"
        elif issue == FileIssue.CORRUPTED_PDF:
            item = f"重新下载: {file_info.original_name} (PDF文件损坏或格式无效)"
        elif issue == FileIssue.READ_ERROR:
            item = f"检查文件权限: {file_info.original_name} (无法读取文件)"
        else:
            item = f"检查文件: {file_info.original_name} (未知问题)"
        
        # Check if item already exists
        if item not in self.items:
            # Add to appropriate category list
            if issue == FileIssue.FAILED_DOWNLOAD:
                self.failed_downloads.append(item)
            elif issue == FileIssue.TOO_SMALL:
                self.small_files.append(item)
            elif issue == FileIssue.CORRUPTED_PDF:
                self.corrupted_files.append(item)
            else:
                self.other_issues.append(item)
            
            self.items.append(item)
    
    def add_failed_download(self, file_info: FileInfo) -> None:
        """Add a failed download file to the todo list."""
        if file_info.is_failed_download:
            self.add_file_issue(file_info, FileIssue.FAILED_DOWNLOAD)
        elif file_info.is_too_small:
            self.add_file_issue(file_info, FileIssue.TOO_SMALL)
    
    def analyze_file_integrity(self, file_info: FileInfo) -> None:
        """Analyze file integrity and add issues if found."""
        # Skip if already marked as failed or too small
        if file_info.is_failed_download or file_info.is_too_small:
            return
        
        # Check PDF integrity for PDF files
        if file_info.extension.lower() == ".pdf":
            if not self._validate_pdf_header(file_info.original_path):
                self.add_file_issue(file_info, FileIssue.CORRUPTED_PDF)
                return
        
        # Check file readability
        try:
            os.stat(file_info.original_path)
        except OSError:
            self.add_file_issue(file_info, FileIssue.READ_ERROR)
    
    def remove_file_from_todo(self, filename: str) -> None:
        """Remove items containing the filename from all lists."""
        filename_lower = filename.lower()
        
        # Remove from main items list
        self.items = [item for item in self.items 
                     if filename_lower not in item.lower()]
        
        # Remove from category lists
        self.failed_downloads = self._filter_list(self.failed_downloads, filename_lower)
        self.small_files = self._filter_list(self.small_files, filename_lower)
        self.corrupted_files = self._filter_list(self.corrupted_files, filename_lower)
        self.other_issues = self._filter_list(self.other_issues, filename_lower)
    
    def write(self) -> None:
        """Write the todo list to the markdown file."""
        content = self._generate_todo_md()
        os.makedirs(os.path.dirname(self.todo_file_path), exist_ok=True)
        with open(self.todo_file_path, 'w', encoding='utf-8') as f:
            f.write(content)
    
    def get_items(self) -> List[str]:
        """Return all todo items."""
        return self.items.copy()
    
    def _extract_items_from_md(self, content: str) -> List[str]:
        """Extract todo items from markdown content."""
        # Skip generic checklist items
        skip_patterns = [
            "检查所有未完成下载文件",
            "重新下载过小文件",
            "验证损坏的PDF文件",
            "处理其他文件问题",
            "MD5校验重复文件",
        ]
        
        items = []
        for line in content.split('\n'):
            line = line.strip()
            if line.startswith('- [') or line.startswith('* ['):
                # Extract item text
                item = re.sub(r'^-?\s*\[[ x]\]\s*', '', line, 1).strip()
                
                # Skip if matches any skip pattern
                should_skip = any(pattern in item for pattern in skip_patterns)
                
                if not should_skip and item:
                    items.append(item)
        
        return items
    
    def _validate_pdf_header(self, file_path: str) -> bool:
        """Validate that a PDF file has the correct header."""
        try:
            with open(file_path, 'rb') as f:
                header = f.read(5)
                return header == b'%PDF-'
        except (OSError, IOError):
            return False
    
    def _generate_todo_md(self) -> str:
        """Generate the markdown content for the todo list."""
        lines = []
        
        lines.append("# 📚 电子书文件检查清单")
        lines.append("")
        lines.append(f"**更新时间**: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        lines.append(f"**扫描目录**: `{self.target_dir}`")
        lines.append("")
        
        # Count total issues
        total_issues = (len(self.failed_downloads) + len(self.small_files) + 
                       len(self.corrupted_files) + len(self.other_issues))
        
        if total_issues > 0:
            lines.append(f"> ⚠️ 发现 **{total_issues}** 个需要处理的问题")
            lines.append("")
        
        if self.failed_downloads:
            lines.append("## 🔄 未完成下载文件")
            lines.append("")
            lines.append("> 这些文件的下载未完成，建议删除后重新下载。")
            lines.append("> 使用 `--auto-cleanup` 选项可以自动清理这些文件。")
            lines.append("")
            for item in self.failed_downloads:
                lines.append(f"- [ ] {item}")
            lines.append("")
        
        if self.small_files:
            lines.append("## 📁 异常小文件（< 1KB）")
            lines.append("")
            lines.append("> 这些文件大小异常，可能是下载失败或文件损坏。")
            lines.append("> 建议检查文件内容，如无效则删除并重新下载。")
            lines.append("")
            for item in self.small_files:
                lines.append(f"- [ ] {item}")
            lines.append("")
        
        if self.corrupted_files:
            lines.append("## 🚨 损坏的PDF文件")
            lines.append("")
            lines.append("> 这些PDF文件的头部信息无效，文件可能已损坏。")
            lines.append("> 建议删除并从原始来源重新下载。")
            lines.append("")
            for item in self.corrupted_files:
                lines.append(f"- [ ] {item}")
            lines.append("")
        
        if self.other_issues:
            lines.append("## ⚠️ 其他文件问题")
            lines.append("")
            for item in self.other_issues:
                lines.append(f"- [ ] {item}")
            lines.append("")
        
        # Add other items that don't fit in categories
        other_items = self._get_other_items()
        if other_items:
            lines.append("## 📋 其他需要处理的文件")
            lines.append("")
            for item in other_items:
                lines.append(f"- [ ] {item}")
            lines.append("")
        
        if not any([self.failed_downloads, self.small_files, 
                   self.corrupted_files, self.other_issues, other_items]):
            lines.append("## ✅ 状态")
            lines.append("")
            lines.append("所有文件已检查完毕，未发现需要处理的问题。")
            lines.append("")
        
        # Add helpful tips
        lines.append("---")
        lines.append("")
        lines.append("### 💡 使用提示")
        lines.append("")
        lines.append("- 使用 `--auto-cleanup` 自动清理未完成下载和损坏文件")
        lines.append("- 使用 `--delete-small` 同时删除异常小文件")
        lines.append("- 使用 `--dry-run` 预览操作而不执行")
        lines.append("")
        lines.append("---")
        lines.append("*此文件由 ebook-renamer 自动生成*")
        
        return '\n'.join(lines)
    
    def _get_other_items(self) -> List[str]:
        """Get items that don't fit in specific categories."""
        category_items = set()
        category_items.update(self.failed_downloads)
        category_items.update(self.small_files)
        category_items.update(self.corrupted_files)
        category_items.update(self.other_issues)
        
        return [item for item in self.items if item not in category_items]
    
    def _filter_list(self, items: List[str], filename: str) -> List[str]:
        """Remove items containing the filename from a list."""
        return [item for item in items if filename not in item.lower()]
