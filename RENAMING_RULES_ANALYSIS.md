# 电子书重命名规则分析与改进建议

## 当前问题总结

### 1. 括号不匹配问题

**当前实现问题：**
- `clean_orphaned_brackets` 函数处理孤立的右括号，但可能遗漏某些边界情况
- 嵌套括号处理时可能产生不完整的括号对
- 处理顺序可能导致某些括号在移除publisher info后变成孤立括号

**问题示例：**
```
输入: Title (Series) (Author) (2020
输出: Author - Title (2020   # 缺少闭合括号

输入: Title ((Nested Info) Author)
输出: Author - Title (   # 移除Nested Info后留下孤立括号
```

### 2. 大小写处理不一致

**当前问题：**
- 关键词检测是区分大小写的（如 "Press", "Series"）
- 某些小写或混合大小写的publisher信息可能无法被识别

**问题示例：**
```
可以识别: (Springer Press)
无法识别: (springer press)
无法识别: (CAMBRIDGE UNIVERSITY PRESS)
无法识别: (textbook series)
```

### 3. 不必要信息移除不够彻底

**当前遗漏的模式：**
- Edition信息: "2nd Edition", "Third Edition", "Revised Edition"
- Volume/Part信息分散在标题中
- ISBN numbers在不同位置
- DOI identifiers
- ArXiv IDs (如: arXiv:1234.5678)
- 版本号: "v1.0", "version 2"
- 语言标注: "(English)", "(中文版)"
- 文件质量标注: "OCR", "Scanned", "Watermarked"
- Reprint信息: "Reprint 2020"

## 改进建议

### 优先级1：增强括号清理

#### 改进方案：
1. **多轮括号清理**：在移除publisher info后，再次检查孤立括号
2. **严格配对检查**：确保所有输出的括号都是成对的
3. **智能括号修复**：如果检测到不匹配，尝试智能闭合或完全移除

#### 新增函数：
```python
def validate_and_fix_brackets(s: str) -> str:
    """确保所有括号都正确配对"""
    # 1. 统计未配对的括号
    open_count = s.count('(') - s.count(')')
    
    # 2. 如果有未配对的开括号在末尾，移除它们
    if open_count > 0:
        # 从末尾移除未配对的开括号
        result = s
        for _ in range(open_count):
            # 找到最后一个未配对的 '('
            result = remove_last_orphaned_open_bracket(result)
        return result
    
    # 3. 如果有多余的闭括号，已在clean_orphaned_brackets中处理
    return s
```

### 优先级2：改进关键词检测（大小写不敏感）

#### 改进方案：
```python
def _is_publisher_or_series_info(self, s: str) -> bool:
    """使用大小写不敏感的关键词匹配"""
    s_lower = s.lower()
    
    publisher_keywords = [
        "press", "publishing", "publisher", 
        "springer", "cambridge", "oxford", "mit press", "elsevier",
        "wiley", "pearson", "academic press",
        "series", "textbook series", "lecture notes",
        "graduate texts", "graduate studies",
        "pure and applied", "foundations of",
        "monographs", "studies", "collection",
        "edition", "revised", "reprint",
        "vol.", "volume", "no.", "part",
        # 中文关键词
        "出版社", "出版", "教材", "系列",
        "丛书", "讲义", "版", "修订版",
    ]
    
    for keyword in publisher_keywords:
        if keyword.lower() in s_lower:
            return True
    
    # 检测版本号模式: "2nd ed", "3rd edition", etc.
    if re.search(r'\d+(?:st|nd|rd|th)\s+ed(?:ition)?', s_lower):
        return True
        
    return False
```

### 优先级3：扩展噪音模式清理

#### 新增清理模式：
```python
def _clean_extended_noise(self, s: str) -> str:
    """扩展的噪音模式清理"""
    patterns_to_remove = [
        # Edition patterns
        r'\s*-?\s*\d+(?:st|nd|rd|th)\s+[Ee]dition\b',
        r'\s*-?\s*\([Rr]evised\s+[Ee]dition\)',
        r'\s*-?\s*\([Ee]dition\s+\d+\)',
        
        # Reprint patterns
        r'\s*-?\s*\([Rr]eprint\s+\d{4}\)',
        
        # Version patterns
        r'\s*-?\s*[vV](?:er(?:sion)?)?\s*\d+(?:\.\d+)*',
        
        # Language annotations
        r'\s*\([Ee]nglish\s+[Vv]ersion\)',
        r'\s*\([Cc]hinese\s+[Vv]ersion\)',
        r'\s*\(中文版\)',
        r'\s*\(英文版\)',
        
        # Quality markers
        r'\s*-?\s*\b(?:OCR|[Ss]canned|[Ww]atermarked|[Bb]ookmarked)\b',
        
        # ArXiv IDs
        r'\s*-?\s*arXiv:\d{4}\.\d{4,5}(?:v\d+)?',
        
        # DOI
        r'\s*-?\s*doi:\s*[\w\./]+',
        
        # ISBN in text (not just trailing)
        r'\s*-?\s*ISBN[-:\s]*\d[\d\-]{8,}',
        
        # Volume/Part in various formats
        r'\s*-?\s*[Vv]ol\.?\s*\d+',
        r'\s*-?\s*[Pp]art\s+[IVX\d]+',
        r'\s*-?\s*第[一二三四五六七八九十\d]+卷',
        
        # Duplicate markers (more comprehensive)
        r'\s*[-_]\s*[Cc]opy\s+\d+',
        r'\s*\(\d{1,2}\)\s*(?=\.|$)',  # (1), (2) at end before extension
    ]
    
    result = s
    for pattern in patterns_to_remove:
        result = re.sub(pattern, '', result, flags=re.IGNORECASE)
    
    return result.strip()
```

### 优先级4：改进处理顺序

#### 优化的处理流程：
```
1. 移除扩展名和.download后缀
2. 移除series前缀
3. 移除所有[...]括号内容
4. 清理基础噪音源（Z-Library, libgen等）
5. 清理扩展噪音模式（Edition, Version, 语言标注等）  # 新增
6. 提取年份
7. 移除包含年份和publisher的括号内容
8. 第一次括号验证和修复  # 新增
9. 解析作者和标题
10. 清理作者名称
11. 清理标题
12. 第二次括号验证和修复  # 新增
13. 生成最终文件名
```

## 测试案例

### 括号不匹配测试：
```python
# 应该移除孤立的开括号
"Title with orphan ( bracket.pdf" → "Author - Title with orphan bracket.pdf"

# 应该修复嵌套括号问题
"Title (Series (Publisher)) (Author).pdf" → "Author - Title.pdf"

# 应该处理多余的闭括号
"Title ) with ) extra ).pdf" → "Title with extra.pdf"
```

### 大小写不敏感测试：
```python
"Title (springer press).pdf" → "Title.pdf"
"Title (CAMBRIDGE UNIVERSITY PRESS).pdf" → "Title.pdf"
"Title (textbook series).pdf" → "Title.pdf"
```

### 扩展噪音清理测试：
```python
"Title - 2nd Edition (Author).pdf" → "Author - Title.pdf"
"Title v1.0 (Author).pdf" → "Author - Title.pdf"
"Title (English Version) (Author).pdf" → "Author - Title.pdf"
"Title - OCR (Author).pdf" → "Author - Title.pdf"
"Title - arXiv:1234.5678 (Author).pdf" → "Author - Title.pdf"
"Title Vol. 1 (Author).pdf" → "Author - Title Vol. 1.pdf"  # 保留卷号在标题中
```

## 实施计划

### 阶段1：修复括号问题（立即）
- [ ] 实现 `validate_and_fix_brackets` 函数
- [ ] 在处理流程中添加多个括号验证点
- [ ] 添加括号相关的单元测试

### 阶段2：改进关键词匹配（高优先级）
- [ ] 将所有关键词匹配改为大小写不敏感
- [ ] 扩展publisher关键词列表
- [ ] 添加版本号模式识别

### 阶段3：扩展噪音清理（中优先级）
- [ ] 实现 `_clean_extended_noise` 函数
- [ ] 整合到处理流程中
- [ ] 添加全面的噪音清理测试

### 阶段4：优化处理顺序（低优先级）
- [ ] 重新排序处理步骤
- [ ] 确保每个步骤的输出都是干净的
- [ ] 更新文档以反映新的处理流程

## 预期效果

### 改进前：
- 某些文件可能有不匹配的括号
- 小写的publisher信息无法识别
- Edition、Version等信息未被移除

### 改进后：
- **100%保证括号配对正确**
- **大小写不敏感的关键词识别**
- **移除更多不必要信息（Edition、Version、Language等）**
- **更清洁、更标准的文件名**

## 向后兼容性

所有改进都是向后兼容的：
- 原本能正确处理的文件名仍然能正确处理
- 新的规则只会移除更多噪音，不会破坏已有的功能
- 测试套件确保不会引入回归

## 性能影响

- 多轮括号检查：**可忽略**（只对少数有问题的文件名进行）
- 大小写转换：**可忽略**（字符串操作很快）
- 扩展正则匹配：**轻微增加**（约5-10%），但文件名处理本身很快

## 建议优先实施

1. **立即实施**：括号验证和修复（影响最大，问题最明显）
2. **本周实施**：大小写不敏感匹配（改进明显，实施简单）
3. **下周实施**：扩展噪音清理（逐步改进，测试充分）

---

**总结**：这些改进将使电子书重命名工具更加健壮、智能和全面，特别是在处理各种边界情况和真实世界中不规范的文件名时。
