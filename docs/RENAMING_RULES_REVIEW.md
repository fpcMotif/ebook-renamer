# 重命名规则详细审查与更新

**日期**: 2025-11-21
**版本**: 1.1.0

## 1. 关键词列表更新

### 1.1 出版商关键词 (Case-insensitive)
原有列表基础上新增以下关键词：
- `Wiley`
- `Pearson`
- `McGraw-Hill`
- `Elsevier`
- `Taylor & Francis`

### 1.2 类型关键词 (新增分类)

#### 通用书籍类型
- `Fiction`, `Novel`
- `Handbook`, `Manual`, `Guide`, `Reference`
- `Cookbook`, `Workbook`
- `Encyclopedia`, `Dictionary`, `Atlas`
- `Anthology`
- `Biography`, `Memoir`
- `Essay`, `Poetry`, `Drama`, `Short Stories`

#### 学术类型 (扩充)
- `Thesis`, `Dissertation`
- `Proceedings`, `Conference`, `Symposium`, `Workshop`
- `Report`, `Technical Report`, `White Paper`
- `Preprint`, `Manuscript`
- `Lecture`, `Course Notes`
- `Study Guide`, `Solutions Manual`

### 1.3 版本与格式标识

#### 版本关键词
- `Revised Edition`, `Updated Edition`, `Expanded Edition`
- `Abridged`, `Unabridged`
- `Complete Edition`, `Anniversary Edition`
- `Collector's Edition`, `Special Edition`
- `1st ed`, `2nd ed`, `3rd ed`

#### 格式/质量标识
- `OCR`, `Scanned`, `Retail`, `Searchable`, `Bookmarked`
- `Optimized`, `Compressed`
- `High Quality`, `HQ`
- `DRM-free`, `No DRM`, `Cracked`
- `Kindle Edition`
- `PDF version`, `EPUB version`, `MOBI version`

### 1.4 多语言支持

#### 中文关键词
- `小说`, `教材`, `教程`, `手册`, `指南`, `参考书`
- `文集`, `论文集`, `丛书`, `系列`
- `修订版`, `第二版`, `第三版`, `增订版`

#### 日文关键词
- `小説`, `教科書`, `テキスト`, `ハンドブック`, `マニュアル`, `ガイド`
- `講義`, `シリーズ`
- `改訂版`, `第2版`, `第3版`

## 2. 模式匹配改进

### 2.1 新增正则模式

1.  **版本号匹配**:
    - Pattern: `\b(v|ver|version)\.?\s*\d+(\.\d+)*\b`
    - 示例匹配: `v1.0`, `Ver. 2.0`, `version 3.5.1`

2.  **页码匹配**:
    - Pattern: `\b\d+\s*(?:pages?|pp?\.?|P)\b`
    - 示例匹配: `123 pages`, `456pp`, `789P`

3.  **语言标签**:
    - 匹配常见语言标记如 `(English)`, `(Chinese)`, `English Edition` 等

### 2.2 噪音清理逻辑优化

针对 `libgen` 和 `Z-Library` 等噪音源，采用了更精确的匹配模式，防止误删文件名中的合法点号或字符。

## 3. 括号与标点处理

- **括号配对**: 强制检查括号平衡，移除孤立的闭括号 `)`。
- **末尾符号**: 移除文件名末尾孤立的开括号 `[` 或 `(`。
- **大小写敏感**: 所有关键词匹配改为大小写不敏感 (Case-insensitive)。

## 4. 实现细节

改进已在 Rust, Go, 和 Python 三种语言的实现中同步更新，确保逻辑一致性。

- **Rust**: `src/normalizer.rs`
- **Go**: `source_go/internal/normalizer/normalizer.go`
- **Python**: `source_py/ebook_renamer/normalizer.py`
