# 自动清理功能说明

## 概述

新增的自动清理功能可以帮助您自动处理和清理各种问题文件，让电子书管理更加轻松自然。

## 新增的命令行选项

### 1. `--auto-cleanup` (推荐)
自动清理所有问题文件，包括：
- 异常小文件（< 1KB）
- 损坏的 PDF 文件
- 未完成的下载文件（.download, .crdownload）
- 空的下载文件夹

**使用示例：**
```bash
# 预览将要清理的内容（dry-run）
ebook-renamer /path/to/books --dry-run --auto-cleanup

# 执行清理（自动确认）
ebook-renamer /path/to/books --auto-cleanup --yes

# 执行清理（交互式确认）
ebook-renamer /path/to/books --auto-cleanup --interactive
```

### 2. `--interactive`
在删除文件前提示用户确认。显示详细的清理计划并等待用户确认。

**使用示例：**
```bash
ebook-renamer /path/to/books --auto-cleanup --interactive
```

### 3. `--yes` 或 `-y`
跳过所有确认提示，直接执行操作。适合在脚本中使用。

**使用示例：**
```bash
ebook-renamer /path/to/books --auto-cleanup -y
```

## 工作流程

### 默认行为（不使用 --auto-cleanup）
1. 扫描文件
2. 将问题文件记录到 `todo.md`
3. 用户手动检查和处理

### 使用 --auto-cleanup
1. 扫描文件
2. 从 .download/.crdownload 文件夹恢复 PDF 文件
3. 自动删除空的下载文件夹
4. 显示清理计划摘要
5. （如果使用 --interactive）等待用户确认
6. 执行清理操作
7. 显示清理结果
8. 更新 `todo.md`（已清理的文件不会出现在列表中）

## 清理计划显示

程序会清晰地显示将要执行的操作：

```
═══ 清理计划摘要 ═══

📁 3 个异常小文件 (< 1KB):
  • small1.pdf
  • small2.pdf
  • small3.pdf

🚨 1 个损坏的PDF文件:
  • corrupted.pdf

🔄 2 个未完成下载文件:
  • book1.download
  • book2.crdownload

📂 2 个空下载文件夹:
  • book1.pdf.download
  • book2.pdf.crdownload

📊 总计: 6 个文件, 2 个文件夹
```

## 使用建议

### 第一次使用
```bash
# 先预览将要做什么
ebook-renamer /path/to/books --dry-run --auto-cleanup

# 如果确认无误，使用交互式模式
ebook-renamer /path/to/books --auto-cleanup --interactive
```

### 日常使用
```bash
# 直接执行清理
ebook-renamer /path/to/books --auto-cleanup -y
```

### 脚本中使用
```bash
#!/bin/bash
# 在 cron 任务或自动化脚本中
ebook-renamer ~/Downloads/Books --auto-cleanup --yes --json > cleanup_log.json
```

## 向后兼容

旧的选项仍然可用：
- `--delete-small`: 仅删除小文件
- `--cleanup-downloads`: 仅清理下载文件夹

`--auto-cleanup` 等同于同时使用这两个选项，但提供了更好的用户体验。

## 安全特性

1. **Dry-run 模式**：始终可以使用 `--dry-run` 预览操作
2. **交互式确认**：使用 `--interactive` 在删除前确认
3. **详细日志**：所有操作都会记录到日志中
4. **错误处理**：如果删除失败，会显示详细的错误信息

## 与 todo.md 的整合

- 不使用清理选项时，问题文件会被记录到 `todo.md`
- 使用清理选项时，已清理的文件会从 `todo.md` 中移除
- `todo.md` 只包含需要人工审查的问题

## 实际场景示例

### 场景1：清理下载文件夹
```bash
# 从浏览器下载了很多电子书，想要整理
ebook-renamer ~/Downloads --auto-cleanup -y

# 结果：
# - 从 .download 文件夹提取 PDF
# - 删除小文件和损坏文件
# - 清理空文件夹
# - 重命名正常文件
```

### 场景2：定期维护
```bash
# 定期检查和清理电子书库
ebook-renamer ~/Books --auto-cleanup --interactive

# 结果：
# - 查看清理计划
# - 确认后执行
# - 保持书库整洁
```

### 场景3：批量处理
```bash
# 处理多个目录
for dir in ~/Books/*/; do
  ebook-renamer "$dir" --auto-cleanup -y --no-recursive
done
```

## 注意事项

1. 删除是永久性的，建议先使用 `--dry-run` 测试
2. 文件大小阈值为 1KB，PDF/EPUB 文件小于此值会被认为是异常的
3. 交互式模式下按 `y` 或 `yes` 或 `是` 确认操作
4. 使用 `--json` 选项可以输出机器可读的结果
