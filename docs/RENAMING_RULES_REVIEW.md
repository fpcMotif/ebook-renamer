# 电子书重命名规则审查与改进建议

## 审查日期
2025-11-21

## 当前问题总结

经过对现有代码和文档的全面审查，发现以下主要问题：

### 1. 括号处理问题

#### 问题描述
- **未闭合的单个括号**：某些文件名中存在单独的未闭合括号，如 `Book Title ) extra info (Author)` 或 `Title ( incomplete`
- **连续括号处理**：多个连续的括号结构可能导致处理顺序问题

#### 当前实现
```rust
fn clean_orphaned_brackets(s: &str) -> String {
    // 逐字符遍历，跟踪开闭括号数量
    // 丢弃没有匹配的闭合括号
    // 移除末尾未闭合的开放括号
}
```

#### 改进建议
1. **增强括号配对检测**：在处理前先扫描整个字符串，识别所有不配对的括号
2. **智能括号修复**：对于明显的格式错误（如 `)（` 这种情况），尝试自动修正
3. **记录异常**：对于无法处理的复杂括号情况，记录到日志供人工审查

### 2. 类型（Genre）和分类信息处理

#### 问题描述
- **类型关键词不全**：当前关键词列表主要针对学术出版物，对于通用书籍类型（如Fiction、Novel、Textbook、Handbook）覆盖不足
- **大小写敏感性**：某些类型信息可能出现不同的大小写变体
- **多语言类型**：除了英文，还有中文、日文等类型标识

#### 当前关键词列表
```
Press, Publishing, Academic Press, Springer, Cambridge, Oxford, MIT Press,
Series, Textbook Series, Graduate Texts, Graduate Studies, Lecture Notes,
Pure and Applied, Mathematics, Foundations of, Monographs, Studies, Collection,
Textbook, Edition, Vol., Volume, No., Part, 理工, 出版社, の
```

#### 建议新增的类型关键词

**通用书籍类型**：
```
Fiction, Novel, Textbook, Handbook, Manual, Guide, Reference,
Cookbook, Workbook, Encyclopedia, Dictionary, Atlas, Anthology,
Biography, Memoir, Essay, Poetry, Drama, Short Stories
```

**学术类型补充**：
```
Thesis, Dissertation, Proceedings, Conference, Symposium, Workshop,
Report, Technical Report, White Paper, Preprint, Manuscript,
Lecture, Course Notes, Study Guide, Solutions Manual
```

**系列和版本信息**：
```
Revised Edition, Second Edition, Third Edition, Updated Edition,
Expanded Edition, Abridged, Unabridged, Complete Edition,
Anniversary Edition, Collector's Edition, Special Edition
```

**中文类型关键词**：
```
小说, 教材, 教程, 手册, 指南, 参考书, 文集, 论文集,
丛书, 系列, 修订版, 第二版, 第三版, 增订版
```

**日文类型关键词**：
```
小説, 教科書, テキスト, ハンドブック, マニュアル, ガイド,
講義, シリーズ, 改訂版, 第2版, 第3版
```

### 3. 不必要信息的识别与删除

#### 需要增强删除的模式

**3.1 版本和版本号**
```regex
# 当前可能遗漏的模式
\b(v|ver|version)\.?\s*\d+(\.\d+)*\b
\b\d+(st|nd|rd|th)\s+(ed\.|edition)\b
```

**3.2 格式和文件质量标识**
```
OCR, Scanned, Retail, Repack, Remastered, Searchable,
Bookmarked, Optimized, Compressed, High Quality, HQ
```

**3.3 语言标识（当不是书名的一部分时）**
```
\(English\), \(中文\), \(日本語\), \(Deutsch\), \(Français\)
English Edition, Chinese Edition, Japanese Edition
```

**3.4 DRM和来源标识**
```
DRM-free, DRM Free, No DRM, Cracked, Retail, Kindle Edition,
PDF version, EPUB version, MOBI version
```

**3.5 页码信息**
```
\d+\s*(?:pages?|pp?\.?|P)\b
```

### 4. 改进的处理流程

#### 建议的新处理顺序

```
1. 移除扩展名和 .download 后缀
2. 移除系列前缀（已有）
3. 移除所有方括号内容 [...]（已有）
4. 清理噪音源标记（已有，需增强）
5. **[新增]** 移除格式和质量标识
6. **[新增]** 移除语言和版本标识  
7. **[新增]** 移除DRM和来源标识
8. 移除重复标记（已有）
9. 提取年份（已有）
10. **[改进]** 清理括号内容（增强类型关键词检测）
11. **[改进]** 智能修复不配对的括号
12. 解析作者和标题（已有）
13. 清理作者名（已有）
14. 清理标题（已有，需增强）
15. 生成新文件名（已有）
```

### 5. 边缘情况处理

#### 5.1 多重括号嵌套
```
输入: Title (Series (Publisher) Volume 1) (2020) (Author)
期望: Author - Title (2020).pdf
处理: 从内到外逐层识别和删除出版信息，保留作者和年份
```

#### 5.2 括号不配对的情况
```
输入: Title ) extra stuff (Author).pdf
期望: Author - Title.pdf
处理: 移除孤立的闭括号 )，识别有效的作者括号
```

#### 5.3 类型信息在不同位置
```
输入: [Fiction] Title (Author) [2020 Edition].pdf
期望: Author - Title (2020).pdf
处理: 移除所有方括号内容，提取年份
```

#### 5.4 混合语言标题
```
输入: 书名 Book Title (作者 Author) (小说 Fiction).pdf
期望: 作者 Author - 书名 Book Title.pdf
处理: 识别CJK作者，移除类型信息（无论语言）
```

### 6. 实施优先级

#### 高优先级（立即实施）
1. ✅ 扩展类型关键词列表（包括通用书籍类型和多语言支持）
2. ✅ 改进括号配对处理逻辑
3. ✅ 添加格式/质量/语言/版本标识的删除规则

#### 中优先级（短期内实施）
4. 添加更多测试用例覆盖边缘情况
5. 实施大小写不敏感的关键词匹配
6. 增强日志记录，便于诊断问题

#### 低优先级（长期考虑）
7. 机器学习模型辅助识别作者和标题
8. 支持从在线数据库（如Google Books API）获取元数据
9. 交互模式让用户确认复杂的重命名操作

## 改进后的关键函数伪代码

### is_publisher_or_series_info_enhanced()

```rust
fn is_publisher_or_series_info_enhanced(s: &str) -> bool {
    // 1. 转换为小写进行大小写不敏感匹配
    let s_lower = s.to_lowercase();
    
    // 2. 扩展的关键词列表（英文+中文+日文）
    let keywords = [
        // 出版商...
        // 系列...
        // 类型...（新增大量通用书籍类型）
        // 版本...
        // 格式...
    ];
    
    // 3. 检查关键词
    for keyword in keywords {
        if s_lower.contains(&keyword.to_lowercase()) {
            return true;
        }
    }
    
    // 4. 正则模式检测
    // - 版本号模式: v1.0, version 2.0, 2nd edition
    // - 页码模式: 500 pages, 500pp
    // - 哈希模式（已有）
    
    // 5. 原有逻辑（哈希、系列编号等）
    // ...
    
    false
}
```

### clean_orphaned_brackets_enhanced()

```rust
fn clean_orphaned_brackets_enhanced(s: &str) -> String {
    // 阶段1: 预扫描，识别所有括号位置和配对情况
    let bracket_pairs = find_bracket_pairs(s);
    
    // 阶段2: 标记要保留的括号和要删除的括号
    let mut keep_flags = vec![true; s.len()];
    for unmatched_pos in bracket_pairs.unmatched {
        keep_flags[unmatched_pos] = false;
    }
    
    // 阶段3: 重建字符串，只保留有效的括号
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c == '(' || c == ')' || c == '[' || c == ']' {
            if keep_flags[i] {
                result.push(c);
            }
        } else if c == '_' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    // 阶段4: 清理末尾的孤立开括号
    while result.ends_with('(') || result.ends_with('[') {
        result.pop();
    }
    
    result.trim().to_string()
}
```

### clean_format_and_quality_indicators()

```rust
fn clean_format_and_quality_indicators(s: &str) -> String {
    let patterns = [
        // 格式标识
        r"\s*\(OCR\)",
        r"\s*\(Scanned\)",
        r"\s*\(Retail\)",
        r"\s*\(Searchable\)",
        r"\s*\(Bookmarked\)",
        
        // 质量标识
        r"\s*\(High Quality\)",
        r"\s*\(HQ\)",
        r"\s*\(Compressed\)",
        
        // DRM
        r"\s*\(DRM[- ]?[Ff]ree\)",
        r"\s*\(No DRM\)",
        
        // 文件格式版本
        r"\s*\(PDF [Vv]ersion\)",
        r"\s*\(EPUB [Vv]ersion\)",
        r"\s*\(Kindle Edition\)",
        
        // 页码
        r"\s*\(\d+\s*(?:pages?|pp?\.?|P)\)",
    ];
    
    let mut result = s.to_string();
    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        result = re.replace_all(&result, "").to_string();
    }
    
    result
}
```

## 测试计划

### 新增测试用例

```rust
// 测试1: 未闭合括号
"Book Title ) extra (Author).pdf" -> "Author - Book Title.pdf"

// 测试2: 通用书籍类型
"Great Novel (Fiction) (John Doe).pdf" -> "John Doe - Great Novel.pdf"
"Cooking Basics (Cookbook) (Jane Chef).pdf" -> "Jane Chef - Cooking Basics.pdf"

// 测试3: 版本信息
"Learn Python (3rd Edition) (2023).pdf" -> "Learn Python (2023).pdf"
"Data Science (v2.0) (Author).pdf" -> "Author - Data Science.pdf"

// 测试4: 格式标识
"Book Title (OCR) (Searchable) (Author).pdf" -> "Author - Book Title.pdf"
"Guide (PDF version) (DRM-free).pdf" -> "Guide.pdf"

// 测试5: 多语言类型
"故事集 (小说) (作者).pdf" -> "作者 - 故事集.pdf"
"学习手册 (教材) (第二版).pdf" -> "学习手册.pdf"

// 测试6: 复杂嵌套
"Title (Series (Publisher) Vol. 1) (2020) (Author).pdf" -> "Author - Title (2020).pdf"

// 测试7: 语言标识
"Book Title (English Edition) (Author).pdf" -> "Author - Book Title.pdf"
"书名 (中文版) (作者).pdf" -> "作者 - 书名.pdf"
```

## 实施建议

### 第一阶段：核心改进（1-2天）
1. 更新 `is_publisher_or_series_info` 函数，添加扩展关键词
2. 实现大小写不敏感匹配
3. 改进 `clean_orphaned_brackets` 函数的括号配对逻辑

### 第二阶段：新功能（2-3天）
4. 实现 `clean_format_and_quality_indicators` 新函数
5. 添加版本和语言标识清理
6. 集成到现有处理流程中

### 第三阶段：测试和验证（1-2天）
7. 编写并运行新的测试用例
8. 在实际文件集上进行测试
9. 收集反馈并微调

### 第四阶段：文档和部署（1天）
10. 更新所有语言实现的文档
11. 同步Go、Python等其他实现
12. 发布新版本

## 最佳实践建议

### 1. 保守原则
- 当不确定是否应该删除某个括号内容时，保留它
- 宁可保留一些不必要的信息，也不要错误删除书名的一部分

### 2. 可配置性
- 考虑允许用户通过配置文件自定义关键词列表
- 提供 `--aggressive` 模式进行更激进的清理
- 提供 `--conservative` 模式进行保守的清理

### 3. 透明度
- 在 verbose 模式下显示每一步的处理结果
- 记录被删除的内容，便于调试
- 提供 diff 视图显示重命名前后的对比

### 4. 跨语言一致性
- 确保Rust、Go、Python等实现行为一致
- 使用统一的测试套件验证所有实现
- 维护单一的规范文档作为权威来源

## 结论

通过以上改进，电子书重命名工具将能够：
1. ✅ **更好地处理括号不配对的情况**
2. ✅ **识别和删除更多类型的不必要信息**（包括通用书籍类型、版本、格式等）
3. ✅ **支持多语言类型标识**（英文、中文、日文）
4. ✅ **提供更准确、更一致的重命名结果**

这些改进将显著提高工具的实用性和可靠性，减少需要手动调整的情况。
