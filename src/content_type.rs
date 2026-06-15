use std::path::Path;

use crate::error::MfError;

/// 检测结果包含内容类型和置信度评分
#[derive(Debug, Clone)]
struct DetectionResult {
    content_type: ContentType,
    confidence: f64,               // 置信度: 0.0 - 1.0
    #[allow(dead_code)]
    signals: Vec<String>,         // 匹配到的信号列表（用于调试）
}

impl DetectionResult {
    /// 创建新的检测结果
    fn new(content_type: ContentType, confidence: f64, signals: Vec<&str>) -> Self {
        DetectionResult {
            content_type,
            confidence,
            signals: signals.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Json,
    Xml,
    Html,
    Python,
    JavaScript,
    TypeScript,
    Sql,
    Shell,
    Batch,
    PowerShell,
    Ruby,
    Php,
    C,
    Cpp,
    Rust,
    Go,
    Java,
    CSharp,
    Yaml,
    Toml,
    Csv,
    Markdown,
    PlainText,
    Binary,
}

impl ContentType {
    pub fn suggested_extensions(&self) -> Vec<&'static str> {
        match self {
            ContentType::Json => vec!["json"],
            ContentType::Xml => vec!["xml", "html", "svg"],
            ContentType::Html => vec!["html", "htm"],
            ContentType::Python => vec!["py"],
            ContentType::JavaScript => vec!["js", "mjs", "cjs"],
            ContentType::TypeScript => vec!["ts"],
            ContentType::Sql => vec!["sql"],
            ContentType::Shell => vec!["sh", "bash"],
            ContentType::Batch => vec!["bat", "cmd"],
            ContentType::PowerShell => vec!["ps1"],
            ContentType::Ruby => vec!["rb"],
            ContentType::Php => vec!["php"],
            ContentType::C => vec!["c", "h"],
            ContentType::Cpp => vec!["cpp", "hpp", "cc", "cxx"],
            ContentType::Rust => vec!["rs"],
            ContentType::Go => vec!["go"],
            ContentType::Java => vec!["java"],
            ContentType::CSharp => vec!["cs"],
            ContentType::Yaml => vec!["yaml", "yml"],
            ContentType::Toml => vec!["toml"],
            ContentType::Csv => vec!["csv"],
            ContentType::Markdown => vec!["md", "markdown"],
            ContentType::PlainText => vec!["txt", "md"],
            ContentType::Binary => vec![],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ContentType::Json => "JSON",
            ContentType::Xml => "XML",
            ContentType::Html => "HTML",
            ContentType::Python => "Python",
            ContentType::JavaScript => "JavaScript",
            ContentType::TypeScript => "TypeScript",
            ContentType::Sql => "SQL",
            ContentType::Shell => "Shell",
            ContentType::Batch => "Batch",
            ContentType::PowerShell => "PowerShell",
            ContentType::Ruby => "Ruby",
            ContentType::Php => "PHP",
            ContentType::C => "C",
            ContentType::Cpp => "C++",
            ContentType::Rust => "Rust",
            ContentType::Go => "Go",
            ContentType::Java => "Java",
            ContentType::CSharp => "C#",
            ContentType::Yaml => "YAML",
            ContentType::Toml => "TOML",
            ContentType::Csv => "CSV",
            ContentType::Markdown => "Markdown",
            ContentType::PlainText => "Plain Text",
            ContentType::Binary => "Binary",
        }
    }
}

fn shebang_to_content_type(shebang: &str) -> Option<ContentType> {
    let lower = shebang.to_lowercase();
    if lower.contains("python") {
        return Some(ContentType::Python);
    }
    if lower.contains("node") || lower.contains("deno") || lower.contains("bun") {
        return Some(ContentType::JavaScript);
    }
    if lower.contains("bash") || lower.contains("sh") || lower.contains("zsh") || lower.contains("dash") {
        return Some(ContentType::Shell);
    }
    if lower.contains("ruby") {
        return Some(ContentType::Ruby);
    }
    if lower.contains("php") {
        return Some(ContentType::Php);
    }
    if lower.contains("perl") {
        return Some(ContentType::PlainText);
    }
    if lower.contains("lua") {
        return Some(ContentType::PlainText);
    }
    None
}

/// 增强版内容类型检测函数 - 使用置信度评分机制
///
/// # 改进点 (v2.0)
/// 1. **多信号加权评分**：不再"首次匹配即返回"，而是收集所有信号并计算置信度
/// 2. **特有信号优先**：使用语言特有特征（如PS的$变量、PHP的->操作符）而非通用关键字
/// 3. **置信度阈值**：只有高置信度的结果才被接受，否则返回PlainText避免误判
/// 4. **冲突解决**：当多个语言都可能时，选择置信度最高的
pub fn detect(content: &str) -> ContentType {
    let trimmed = content.trim();

    // 快速路径：空内容
    if trimmed.is_empty() {
        return ContentType::PlainText;
    }

    // 快速路径：二进制内容
    if content.contains('\0') {
        return ContentType::Binary;
    }

    // 收集所有候选检测结果
    let candidates = collect_all_candidates(content, trimmed);

    // 如果只有一个候选或没有候选，直接返回
    if candidates.is_empty() {
        return ContentType::PlainText;
    }
    if candidates.len() == 1 {
        return candidates[0].content_type.clone();
    }

    // 多个候选时，选择置信度最高的
    let best = candidates
        .into_iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .unwrap();

    // 如果最佳结果的置信度过低，返回PlainText以避免误判
    // 放宽阈值至0.15以适应短文本片段
    if best.confidence < 0.15 {
        return ContentType::PlainText;
    }

    best.content_type
}

/// 收集所有可能的候选检测结果及其置信度
fn collect_all_candidates(content: &str, trimmed: &str) -> Vec<DetectionResult> {
    let mut candidates = Vec::new();

    // 1. Shebang检测（最高优先级 - 置信度0.95）
    if let Some(ct) = detect_shebang(content) {
        candidates.push(DetectionResult::new(ct, 0.95, vec!["shebang"]));
    }

    // 2. 结构化数据格式检测（JSON/XML/HTML/YAML/TOML/CSV）
    let has_structured_format = if let Some(result) = detect_structured_formats(trimmed, content) {
        candidates.push(result);
        true
    } else {
        false
    };

    // 3. 脚本语言检测（PowerShell/Batch/Shell）- 在编程语言之前以利用特有信号
    if let Some(result) = detect_script_languages(content) {
        candidates.push(result);
    }

    // 4. 编程语言检测
    if let Some(result) = detect_programming_languages(content) {
        candidates.push(result);
    }

    // 5. Markdown检测（只有当没有检测到结构化格式时才添加，避免误判）
    // 例如：包含---的内容可能是YAML或日志，不应再被Markdown检测干扰
    if !has_structured_format {
        if let Some(result) = detect_markdown_enhanced(content, trimmed) {
            candidates.push(result);
        }
    }

    // 特殊规则：包含代码围栏的内容应优先为Markdown
    if content.contains("```") {
        let has_high_confidence_md = candidates.iter()
            .any(|c| c.content_type == ContentType::Markdown && c.confidence >= 0.7);
        if !has_high_confidence_md {
            candidates.push(DetectionResult::new(
                ContentType::Markdown,
                0.78, // 高置信度以确保胜出
                vec!["code_fence_detected"],
            ));
        }
    }

    candidates
}

/// Shebang行检测
fn detect_shebang(content: &str) -> Option<ContentType> {
    if let Some(first_line) = content.lines().next() {
        let fl = first_line.trim();
        if fl.starts_with("#!") {
            return shebang_to_content_type(fl);
        }
    }
    None
}

/// 结构化数据格式检测（JSON/XML/HTML/YAML/TOML/CSV）
fn detect_structured_formats(trimmed: &str, content: &str) -> Option<DetectionResult> {
    let first_char = trimmed.chars().next()?;

    // JSON检测 - 使用严格的语法验证
    if first_char == '{' || first_char == '[' {
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return Some(DetectionResult::new(ContentType::Json, 0.9, vec!["json_syntax_validated"]));
        }
    }

    // XML / HTML检测 - 增强HTML片段识别 + 严格XML验证
    if first_char == '<' {
        let has_closing_tag = trimmed.contains("</") || trimmed.ends_with("/>");

        if has_closing_tag {
            // HTML检测：增加常见HTML标签识别，不再严格要求<html>标记
            let is_html = trimmed.contains("<html")
                || trimmed.contains("<!DOCTYPE html")
                || trimmed.contains("<head")
                || ((trimmed.contains("<div") || trimmed.contains("<p ") || trimmed.contains("<span")
                    || trimmed.contains("<table") || trimmed.contains("<a ")
                    || trimmed.contains("<form") || trimmed.contains("<input"))
                    && (trimmed.contains("class=") || trimmed.contains("id=")
                        || trimmed.contains("href=") || trimmed.contains("src=")
                        || trimmed.contains("type=") || trimmed.contains("name=")));

            // XML验证：只接受标准XML文档格式
            // 策略：必须有XML声明或DOCTYPE，或者有完整的单一根元素结构
            let has_xml_structure =
                // 必须以标准XML声明或DOCTYPE开头
                trimmed.starts_with("<?xml") || 
                trimmed.starts_with("<!DOCTYPE") ||
                // 或者有明确的单一根元素结构
                (trimmed.starts_with("<") && {
                    let after_open = trimmed[1..].trim();
                    // 检查是否有合法的元素名（字母开头）
                    if after_open.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) {
                        // 提取标签名
                        let tag_name: String = after_open.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
                        if !tag_name.is_empty() {
                            // 检查是否有对应的闭合标签
                            let has_closing = trimmed.contains(&format!("</{}>", tag_name));
                            
                            if has_closing {
                                // 关键检查：确保内容中只有一个根级别的标签对
                                // 如果有多个根标签（如 <user>...</user> said <message>...</message>）则不是有效XML
                                
                                // 方法：检查</tag_name>之后是否还有>符号
                                // 如果有，说明可能还有其他标签
                                if let Some(first_closing_pos) = trimmed.find(&format!("</{}>", tag_name)) {
                                    // 检查</tag_name>之后是否还有内容（不包括可能的空白）
                                    let after_first_closing = &trimmed[first_closing_pos + tag_name.len() + 3..]; // +3 for </ >
                                    let after_trimmed = after_first_closing.trim();
                                    
                                    // 如果</tag_name>之后还有非空白内容，说明有多个顶级元素
                                    #[cfg(test)]
                                    eprintln!("[XML DEBUG] after_first_closing={:?}", after_trimmed);
                                    
                                    // 只有当</tag_name>之后为空或只有空白时，才认为是有效的单一根元素
                                    after_trimmed.is_empty()
                                } else {
                                    false
                                }
                            } else {
                                // 自闭合标签 - 暂时不接受为XML
                                false
                            }
                        } else { false }
                    } else { false }
                });
            
            #[cfg(test)]
            eprintln!("[XML DEBUG] has_xml_structure={}", has_xml_structure);

            // 只有当内容具有XML结构特征时才进行解析验证
            let is_valid_xml = if has_xml_structure {
                // 尝试读取多个事件来判断格式是否有效
                let mut xml_reader = quick_xml::Reader::from_str(trimmed);
                xml_reader.config_mut().trim_text(true);
                let mut buf = Vec::new();
                let mut event_count = 0;
                let max_events = 10;
                
                while event_count < max_events {
                    match xml_reader.read_event_into(&mut buf) {
                        Ok(quick_xml::events::Event::Eof) => {
                            break;
                        }
                        Ok(_) => {
                            event_count += 1;
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                
                event_count > 0
            } else {
                false
            };

            if is_valid_xml {
                if is_html {
                    return Some(DetectionResult::new(ContentType::Html, 0.88, vec!["html_and_xml_validated"]));
                }
                return Some(DetectionResult::new(ContentType::Xml, 0.85, vec!["xml_validated"]));
            }

            // 验证失败，不返回结果
            #[cfg(test)]
            eprintln!("[DEBUG] XML/HTML解析验证失败");
        }
    }

    // YAML检测 - 使用严格解析验证 + 格式特征检查
    // 策略：对于以---开头的内容，要求必须有明确的YAML结构特征，否则直接返回PlainText
    if trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n") {
        // 检查是否有YAML特有的格式特征
        // 更严格的检查：必须是真正的键值对或列表结构
        let has_yaml_structure = 
            // 方法1：明确的键值对（冒号+空格，且冒号前是标识符）
            trimmed.lines().skip(1).any(|line| {
                let t = line.trim();
                if let Some(pos) = t.find(": ") {
                    if pos > 0 {
                        let before_colon = &t[..pos];
                        // 确保冒号前是有效的YAML键（包含字母）
                        before_colon.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }) ||
            // 方法2：列表项（- 开头）
            trimmed.lines().skip(1).any(|line| {
                let t = line.trim();
                t.starts_with("- ")
            }) ||
            // 方法3：嵌套结构（缩进+冒号）
            trimmed.lines().skip(1).any(|line| {
                line.starts_with("  ") && line.contains(": ")
            });

        #[cfg(test)]
        eprintln!("[YAML DEBUG] 内容前60字符: {:?}", &trimmed[..std::cmp::min(60, trimmed.len())]);
        #[cfg(test)]
        eprintln!("[YAML DEBUG] has_yaml_structure: {}", has_yaml_structure);

        if has_yaml_structure {
            // 二次验证：尝试用YAML解析器解析内容
            // 策略：先尝试单文档解析，如果失败再尝试多文档检测
            match serde_yaml::from_str::<serde_yaml::Value>(trimmed) {
                Ok(_) => {
                    return Some(DetectionResult::new(
                        ContentType::Yaml,
                        0.92,
                        vec!["yaml_validated_by_parser"],
                    ));
                }
                Err(e) => {
                    #[cfg(test)]
                    eprintln!("[YAML DEBUG] serde_yaml(from_str)解析失败: {:?}", e);
                    
                    // 检查是否是YAML多文档（通过手动检测---分隔符）
                    // YAML多文档格式：文档1内容---文档2内容---...
                    let doc_count = trimmed.matches("---").count();
                    if doc_count >= 2 {
                        #[cfg(test)]
                        eprintln!("[YAML DEBUG] 检测到YAML多文档，分隔符数量: {}", doc_count);
                        return Some(DetectionResult::new(
                            ContentType::Yaml,
                            0.92,
                            vec!["yaml_multi_doc_detected"],
                        ));
                    }
                    // YAML解析失败，但有YAML特征，继续让Markdown检测
                }
            }
        } else {
            // 没有YAML结构特征，这是伪YAML（如日志），直接返回PlainText
            eprintln!("[YAML DEBUG] 没有YAML结构特征，返回PlainText");
            return Some(DetectionResult::new(
                ContentType::PlainText,
                0.5,
                vec!["looks_like_yaml_but_not_valid"],
            ));
        }
    }

    // TOML检测 - 使用严格解析验证（防止日志等误判）
    if first_char == '[' && !trimmed.starts_with('{') {
        let has_keyval = trimmed.contains('=');
        let has_section = trimmed.contains('[') && trimmed.contains(']');
        if has_keyval && has_section {
            match toml::from_str::<toml::Table>(trimmed) {
                Ok(_) => {
                    return Some(DetectionResult::new(ContentType::Toml, 0.88, vec!["toml_validated"]));
                }
                Err(_) => {}
            }
        }
    }

    // CSV检测 - 增强引号处理
    if content.contains(',') {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() >= 2 {
            let first_comma_count = count_csv_commas(lines[0]);
            if first_comma_count > 0
                && lines[1..]
                    .iter()
                    .all(|l| count_csv_commas(l) == first_comma_count)
            {
                return Some(DetectionResult::new(
                    ContentType::Csv,
                    0.8,
                    vec!["csv_consistent_columns"],
                ));
            }
        }
    }

    None
}

/// 计算CSV行的逗号数（正确处理引号内的逗号）
fn count_csv_commas(line: &str) -> usize {
    let mut in_quotes = false;
    let mut count = 0;

    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes; // 切换引号状态
            }
            ',' if !in_quotes => {
                count += 1;
            }
            _ => {}
        }
    }
    count
}

/// 脚本语言检测 - PowerShell/Batch/Shell（优先于编程语言以利用$等特有信号）
fn detect_script_languages(content: &str) -> Option<DetectionResult> {
    let mut ps_score = 0.0; // PowerShell得分
    let mut batch_score = 0.0; // Batch得分
    let mut shell_score = 0.0; // Shell得分

    // PowerShell特有信号（高权重特征）
    let ps_strong_signals = [
        (content.contains("$"), 0.15), // $变量语法
        (content.contains("$env:"), 0.25), // 环境变量
        (
            content.contains("$PSScriptRoot") || content.contains("$PSVersionTable"),
            0.3,
        ), // PS自动变量
        (content.contains("Write-Host") || content.contains("Write-Output"), 0.2), // PS cmdlet
        (content.contains("[CmdletBinding(") || content.contains("[Parameter("), 0.25), // PS属性
        (content.contains("$_") || content.contains("| %"), 0.15), // 管道变量
        (
            content.contains("-Force")
                || content.contains("-ErrorAction")
                || content.contains("-WarningAction"),
            0.15,
        ), // PS通用参数
        (
            content.contains("Get-")
                || content.contains("Set-")
                || content.contains("New-")
                || content.contains("Remove-"),
            0.2,
        ), // 动词-名词模式
        (content.contains("param("), 0.2), // 参数块
        (content.starts_with("#requires"), 0.25), // #requires语句
    ];

    for (signal, weight) in ps_strong_signals.iter() {
        if *signal {
            ps_score += weight;
        }
    }

    // PowerShell弱信号（可能与JS冲突的function关键字）
    if content.contains("function ") && ps_score > 0.2 {
        // 只有当已有其他PS信号时才计入function
        ps_score += 0.1;
    }

    // Batch文件信号
    let batch_signals = [
        (content.contains("@echo off") || content.contains("@echo on"), 0.35),
        (content.contains("%errorlevel%"), 0.3),
        (content.contains("%~") || content.contains("%1") || content.contains("%*"), 0.25),
        (content.contains("goto "), 0.2),
        (content.contains(":label") || content.matches(":").count() > 2, 0.15), // 标签
        (content.contains("if not ") || content.contains("if exist "), 0.2),
    ];

    for (signal, weight) in batch_signals.iter() {
        if *signal {
            batch_score += weight;
        }
    }

    // Shell脚本信号（区分于PS的#注释）
    let shell_signals = [
        (content.contains("#!/bin/bash") || content.contains("#!/bin/sh"), 0.4), // shebang最明确
        (content.contains("echo $") && !content.contains("$("), 0.2), // shell echo
        (content.contains("chmod +x") || content.contains("./"), 0.15), // Unix模式
        (content.contains("&& ") || content.contains("|| "), 0.1), // 链接命令
        (content.contains("export "), 0.15), // export命令
        (content.contains("2>&1") || content.contains("/dev/null"), 0.2), // 重定向
    ];

    for (signal, weight) in shell_signals.iter() {
        if *signal {
            shell_score += weight;
        }
    }

    // 选择得分最高的脚本语言
    let results = vec![
        (ps_score, ContentType::PowerShell, "powershell_specific_signals"),
        (batch_score, ContentType::Batch, "batch_cmd_commands"),
        (shell_score, ContentType::Shell, "shell_unix_patterns"),
    ];

    let best = results
        .into_iter()
        .filter(|(score, _, _)| *score >= 0.25) // 降低阈值以适应简单脚本
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    best.map(|(score, ct, sigs)| DetectionResult::new(ct, (0.5_f64 + score).min(0.92), vec![sigs]))
}

/// 编程语言检测 - 使用多信号加权评分
///
/// 关键改进：使用语言特有信号组合，减少关键字冲突
fn detect_programming_languages(content: &str) -> Option<DetectionResult> {
    let mut scores: Vec<(ContentType, f64, Vec<&str>)> = Vec::new();

    // ===== Rust检测 =====
    let rust_signals = vec![
        (content.contains("fn "), 0.2), // 单独的fn也应给较高权重
        (content.contains("fn ") && (content.contains("->") || content.contains("let mut ")), 0.15),
        (content.contains("use ") && content.contains("::"), 0.2),
        (content.contains("impl "), 0.2),
        (content.contains("&mut"), 0.15),
        (content.contains(".unwrap()") || content.contains(".expect("), 0.15),
        (content.contains("let Ok(") || content.contains("let Err("), 0.15),
        (content.contains("Vec<") || content.contains("HashMap<"), 0.15),
        (content.contains("pub fn ") || content.contains("pub struct "), 0.1),
        (content.contains("println!"), 0.25), // Rust宏调用（高度特有）
    ];
    let rust_conf = calculate_weighted_score(&rust_signals);
    if rust_conf >= 0.15 { // 降低阈值
        scores.push((
            ContentType::Rust,
            (0.58 + rust_conf).min(0.93), // 提高基础分
            vec!["rust_patterns"],
        ));
    }

    // ===== Go检测 =====
    let go_signals = vec![
        (content.contains("func "), 0.2),
        (content.contains("package "), 0.2),
        (content.contains("import ("), 0.25),
        (content.contains(":= "), 0.25), // Go特有的短变量声明
        (content.contains("range "), 0.15),
        (content.contains("go func("), 0.2), // goroutine
        (content.contains("defer "), 0.15),
        (content.contains("chan "), 0.2), // channel
        (content.contains("error != nil"), 0.2), // Go错误处理
    ];
    let go_conf = calculate_weighted_score(&go_signals);
    if go_conf > 0.0 {
        scores.push((ContentType::Go, (0.55 + go_conf).min(0.9), vec!["go_patterns"]));
    }

    // ===== Java检测 =====
    let java_signals = vec![
        (
            (content.contains("public static void") || content.contains("public class"))
                && content.contains("String[] args"),
            0.35,
        ),
        (content.contains("System.out.") || content.contains("System.err."), 0.2),
        (content.contains("@Override") || content.contains("@SuppressWarnings"), 0.15),
        (content.contains("import java."), 0.2),
        (content.contains("new ArrayList<") || content.contains("new HashMap<"), 0.15),
        (content.contains("public class ") && content.contains("extends "), 0.15),
    ];
    let java_conf = calculate_weighted_score(&java_signals);
    if java_conf > 0.0 {
        scores.push((
            ContentType::Java,
            (0.6 + java_conf).min(0.92),
            vec!["java_patterns"],
        ));
    }

    // ===== C#检测 =====
    let csharp_signals = vec![
        (content.contains("using System"), 0.25),
        (
            content.contains("namespace ") && content.contains("class "),
            0.2,
        ),
        (
            content.contains("static void Main") && content.contains("string[] args"),
            0.3,
        ),
        (content.contains("Console."), 0.2),
        (content.contains("public class ") && content.contains("{"), 0.1),
        (content.contains("var ") && content.contains(";"), 0.1),
    ];
    let csharp_conf = calculate_weighted_score(&csharp_signals);
    if csharp_conf >= 0.45 {
        // C#需要较高的阈值以避免与Java混淆
        scores.push((
            ContentType::CSharp,
            (0.65 + csharp_conf).min(0.9),
            vec!["csharp_patterns"],
        ));
    }

    // ===== C/C++检测 =====
    let c_signals = vec![
        (content.contains("int main("), 0.3),
        (content.contains("#include <"), 0.25),
        (
            content.contains("void ") && (content.contains("printf") || content.contains("scanf")),
            0.2,
        ),
        (content.contains("std::cout") || content.contains("std::cin"), 0.2),
        (content.contains("return 0;"), 0.1),
    ];
    let c_conf = calculate_weighted_score(&c_signals);
    if c_conf > 0.0 {
        scores.push((ContentType::C, (0.55 + c_conf).min(0.88), vec!["c_patterns"])); // 降低基础分

        // C++额外特征
        let cpp_extra = vec![
            (content.contains("std::"), 0.25),
            (content.contains("template <"), 0.3),
            (
                content.contains("class ") && (content.contains("public:") || content.contains("private:")),
                0.25,
            ),
            (content.contains("vector<") || content.contains("string "), 0.2),
            (content.contains("cout<<") || content.contains("cin>>"), 0.25),
        ];
        let cpp_conf = calculate_weighted_score(&cpp_extra);
        if cpp_conf >= 0.2 { // 降低阈值
            scores.push((
                ContentType::Cpp,
                (0.65 + cpp_conf).min(0.9), // 提高基础分，确保高于C
                vec!["cpp_patterns"],
            ));
        }
    }

    // ===== Python检测 =====
    // 改进：明确区分于Ruby（两者都有def关键字）
    let python_signals = vec![
        (content.contains("def "), 0.12),
        (content.contains("import "), 0.12),
        (content.contains("if __name__"), 0.2),
        (content.contains("elif "), 0.18),
        (content.contains("except "), 0.18),
        (content.contains(":") && content.contains("self"), 0.22), // Python特有：self参数
        (content.contains("print("), 0.08), // Python 3风格
        (content.contains("class ") && content.contains("(object)"), 0.15), // Python 2风格类
        (content.contains("[...] for ... in "), 0.15), // 列表推导式
        (content.contains("None") || content.contains("True") || content.contains("False"), 0.1), // Python布尔值
    ];
    let python_conf = calculate_weighted_score(&python_signals);

    // Ruby信号（用于排除）
    let ruby_exclusion = vec![
        (content.contains("puts "), 0.15),
        (content.contains("end"), 0.12), // Ruby的end关键字
        (content.contains("do |"), 0.2), // Ruby块语法
        (content.contains("#{"), 0.18), // 字符串插值
    ];
    let ruby_conf_for_exclusion = calculate_weighted_score(&ruby_exclusion);

    // 只有当Python信号明显多于Ruby，或者没有明显Ruby特征时才判定为Python
    if python_conf > 0.0 && (python_conf > ruby_conf_for_exclusion + 0.15 || ruby_conf_for_exclusion < 0.25) {
        scores.push((
            ContentType::Python,
            (0.55 + python_conf).min(0.88),
            vec!["python_patterns_distinct_from_ruby"],
        ));
    }

    // ===== Ruby检测 =====
    // 明确区分于Python
    let ruby_signals = vec![
        (content.contains("puts "), 0.2),
        (content.contains("end"), 0.12),
        (content.contains("do |"), 0.25),
        (content.contains("#{"), 0.22),
        (content.contains("def ") && content.matches("end").count() > 0, 0.2), // 方法+end模式
        (content.contains("require "), 0.15),
        (content.contains("attr_accessor") || content.contains("attr_reader"), 0.2),
        (content.contains("||="), 0.15), // Ruby或等于
        (content.contains("each do"), 0.2), // 迭代器
    ];
    let ruby_conf = calculate_weighted_score(&ruby_signals);

    // 验证不是Python（Python也有def）
    let is_likely_python = content.contains(":")
        && (content.contains("import ") || content.contains("self") || content.contains("elif"));

    if ruby_conf >= 0.4 && (!is_likely_python || ruby_conf > python_conf + 0.15) {
        scores.push((
            ContentType::Ruby,
            (0.58 + ruby_conf).min(0.88),
            vec!["ruby_patterns_distinct_from_python"],
        ));
    }

    // ===== PHP检测（新增专用检测）=====
    // 解决之前被误判为JavaScript的问题
    let php_signals = vec![
        (content.contains("<?php") || content.contains("<?="), 0.35), // PHP标签（最强信号）
        (
            content.contains("$") && content.contains("->") && !content.contains("//"),
            0.25,
        ), // $var->method() 模式
        (content.contains("function ") && content.contains("$"), 0.15), // 函数中的$参数
        (content.contains("echo ") && content.contains("$"), 0.15), // echo + 变量
        (content.contains("array("), 0.12),
        (content.contains("=>") && content.contains("$"), 0.15), // 关联数组
        (content.contains("$_POST") || content.contains("$_GET") || content.contains("$_SERVER"), 0.25), // 超全局变量
        (content.contains("namespace ") && content.contains("\\\\"), 0.2), // 命名空间
        (content.contains("class ") && content.contains("private $"), 0.2), // 类属性
    ];
    let php_conf = calculate_weighted_score(&php_signals);
    if php_conf >= 0.35 {
        // PHP需要较高阈值以确保准确性
        scores.push((
            ContentType::Php,
            (0.62 + php_conf).min(0.91),
            vec!["php_specific_patterns"],
        ));
    }

    // ===== TypeScript检测 =====
    // 在JavaScript之前，但需要更严格的条件
    let ts_signals = vec![
        (content.contains("interface "), 0.25),
        (
            content.contains(": string")
                || content.contains(": number")
                || content.contains(": boolean")
                || content.contains(": any"),
            0.2,
        ),
        (content.contains("type ") && content.contains("="), 0.15),
        (content.contains("as ") && (content.contains(":") || content.contains("<")), 0.15), // 类型断言
        (content.contains("<T>") || content.contains("<T,>"), 0.2), // 泛型
        (content.contains("enum "), 0.2), // 枚举
        (content.contains("implements "), 0.15), // 实现接口
        (content.contains(": void") || content.contains(": Promise"), 0.12), // 返回类型注解
    ];
    let ts_conf = calculate_weighted_score(&ts_signals);
    if ts_conf >= 0.2 { // 进一步降低：单信号（如: string）也应触发
        scores.push((
            ContentType::TypeScript,
            (0.55 + ts_conf).min(0.9), // 提高基础分
            vec!["typescript_type_annotations"],
        ));
    }

    // ===== JavaScript检测 =====
    // 排除其他语言的function关键字冲突
    let js_signals = vec![
        (content.contains("function "), 0.15), // 提高权重：虽然通用但很重要
        (content.contains("const "), 0.15),
        (content.contains("let "), 0.12),
        (content.contains("var "), 0.08),
        (content.contains("=>"), 0.18), // 箭头函数（较特有）
        (content.contains("console.") && !content.contains("Write-"), 0.2), // 排除PS的Write-*
        (content.contains("document.") || content.contains("window."), 0.25), // DOM API（高度特有）
        (content.contains("require("), 0.15), // CommonJS
        (
            content.contains("export ") || (content.contains("import ") && content.contains("from ")),
            0.15,
        ), // ES6模块
        (content.contains("Promise.") || content.contains("async "), 0.12), // 异步
        (content.contains("===") || content.contains("!=="), 0.1), // 严格相等
        (content.contains("null") || content.contains("undefined"), 0.06), // JS字面量
    ];
    let js_conf = calculate_weighted_score(&js_signals);

    // 排除法：如果更可能是其他语言，不判定为JS
    let is_likely_not_js =
        // 已检测到TS且TS信号更强
        (ts_conf >= 0.4 && ts_conf >= js_conf - 0.1)
        // 已检测到PHP且包含PHP标签或超全局变量
        || (php_conf >= 0.35 && (content.contains("<?php") || content.contains("$_")))
        // 已检测到PowerShell且PS信号较强（稍后在script检测中处理）
        ;

    if js_conf >= 0.15 && !is_likely_not_js { // 进一步降低：单个function关键字也应触发
        scores.push((
            ContentType::JavaScript,
            (0.55 + js_conf).min(0.85), // 提高基础分
            vec!["javascript_with_exclusions"],
        ));
    }

    // ===== SQL检测 =====
    let sql_keywords = [
        "SELECT ",
        "INSERT ",
        "UPDATE ",
        "CREATE ",
        "ALTER ",
        "DROP ",
        "DELETE ",
        "FROM ",
        "WHERE ",
        "JOIN ",
        "INNER JOIN ",
        "LEFT JOIN ",
        "GROUP BY ",
        "ORDER BY ",
    ];
    let sql_count = sql_keywords
        .iter()
        .filter(|kw| {
            content
                .lines()
                .any(|l| l.trim().to_uppercase().starts_with(*kw))
        })
        .count();

    if sql_count >= 1 { // 降低阈值：单个SQL关键字也应触发检测
        scores.push((
            ContentType::Sql,
            (0.6 + sql_count as f64 * 0.05).min(0.88), // 提高基础分
            vec!["sql_keywords"],
        ));
    }

    // 返回得分最高的候选
    scores
        .into_iter()
        .map(|(ct, conf, sigs)| DetectionResult::new(ct, conf, sigs))
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
}

/// 计算加权得分（辅助函数）
fn calculate_weighted_score(signals: &[(bool, f64)]) -> f64 {
    signals.iter().filter(|(present, _)| *present).map(|(_, weight)| weight).sum()
}

/// 增强的Markdown检测
/// 改进：处理包含代码块的Markdown文档
fn detect_markdown_enhanced(content: &str, _trimmed: &str) -> Option<DetectionResult> {
    // 先尝试提取纯Markdown文本（排除代码块）
    let markdown_text = strip_code_blocks(content);

    let md_heading = markdown_text.trim().starts_with("# ")
        || markdown_text.trim().starts_with("## ");
    let md_link = markdown_text.contains("[") && markdown_text.contains("](");
    let md_fence = content.contains("```"); // 保留原始内容的代码围栏检测
    let md_bold = markdown_text.contains("**") || markdown_text.contains("__");
    let md_hr = markdown_text.contains("---") || markdown_text.contains("***");
    let md_list = markdown_text.lines().any(|l| {
        let t = l.trim();
        t.starts_with("- ") || t.starts_with("* ") || t.starts_with("1. ")
            || t.starts_with("2. ") || t.starts_with("3. ")
    });

    let total_lines = markdown_text.lines().count();
    let md_heading_count = markdown_text.lines().filter(|l| {
        let t = l.trim();
        t.starts_with("# ") || t.starts_with("## ") || t.starts_with("### ")
            || t.starts_with("#### ") || t.starts_with("##### ") || t.starts_with("###### ")
    }).count();

    // 要求至少2个Markdown信号以减少误判（从代码注释等）
    let signal_count = vec![md_fence, md_link, md_bold, md_hr, md_list].iter().filter(|&x| *x).count();

    if signal_count >= 1 // 降低要求：单个强信号（如标题）也可触发
        || (md_heading && (md_bold || md_hr || md_list || md_fence || md_link))
        || md_heading_count >= 1 // 单个标题也应被识别
        || (md_heading && total_lines <= 5)
    {
        // 计算置信度：信号越多越确定
        let confidence = 0.58 + (signal_count as f64 * 0.06) + (md_heading_count as f64 * 0.03); // 提高基础分
        return Some(DetectionResult::new(
            ContentType::Markdown,
            confidence.min(0.87),
            vec!["markdown_multiple_signals"],
        ));
    }

    None
}

/// 移除代码块内容，仅保留Markdown文本
/// 这解决了"Markdown中包含JS代码块被误判为JS"的问题
fn strip_code_blocks(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_code_block = false;

    for line in content.lines() {
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            continue; // 完全跳过代码围栏行
        }

        if !in_code_block {
            result.push_str(line);
            result.push('\n');
        }
        // 如果在代码块内，跳过该行
    }

    result
}

pub fn check_match(content_type: &ContentType, filename: &Path) -> Result<(), MfError> {
    let ext = filename.extension().and_then(|e| e.to_str());
    let ext = match ext {
        Some(e) => e,
        None => return Ok(()),
    };

    let suggested = content_type.suggested_extensions();
    if suggested.contains(&ext) {
        return Ok(());
    }

    // 对于纯文本、Markdown等相似格式（.log, .txt, .md, .markdown），互相之间不再警告
    // 因为它们本质上都是文本文档，格式界限模糊
    let text_like_extensions = ["log", "txt", "md", "markdown"];
    let ext_lower = ext.to_lowercase();
    let is_text_like = text_like_extensions.contains(&ext_lower.as_str());
    let has_text_like_suggestion = suggested.iter().any(|s| {
        text_like_extensions.contains(&s.to_lowercase().as_str())
    });
    if is_text_like && has_text_like_suggestion {
        return Ok(());
    }

    if *content_type == ContentType::Binary {
        return Err(MfError::InvalidArgument(
            "检测到二进制数据，无法处理".to_string(),
        ));
    }

    let alternatives = if suggested.is_empty() {
        String::new()
    } else {
        suggested.join(", ")
    };

    Err(MfError::InvalidArgument(format!(
        "文件扩展名 '.{}' 与检测到的内容类型不匹配。建议扩展名: {}",
        ext, alternatives
    )))
}

/// Return the best suggested extension for the detected content type.
pub fn suggest_extension(content_type: &ContentType) -> Option<&'static str> {
    content_type.suggested_extensions().first().copied()
}

/// Enhanced content type detection using file-identify for shebang and binary analysis.
/// Writes content to a temp file with a generic name (avoiding extension bias),
/// runs file-identify, and cross-checks tags with heuristic content detection.
/// Falls back to regular `detect()` if file-identify is unavailable or ambiguous.
pub fn detect_enhanced(content: &str, _filename: Option<&Path>) -> ContentType {
    // Write to temp file for file-identify analysis.
    // Use a GENERIC filename to avoid extension bias — file-identify's extension
    // mapping would override actual content (e.g. Python code saved to .rs would
    // be tagged as Rust). Only shebang and binary detection from content matter.
    let tmp_dir = std::env::temp_dir().join("mf-detect");
    let _ = std::fs::create_dir_all(&tmp_dir);

    let tmp_path = tmp_dir.join("clipboard.txt");
    let _ = std::fs::write(&tmp_path, content);

    let tags = file_identify::tags_from_path(&tmp_path).unwrap_or_default();

    let _ = std::fs::remove_file(&tmp_path);

    let tag_set: Vec<&str> = tags.iter().map(|t| t.as_ref()).collect();

    // Binary detection from content — file-identify is authoritative here
    if tag_set.contains(&"binary") {
        return ContentType::Binary;
    }

    // Shebang-based detection from content — file-identify knows many interpreters
    // that we don't manually handle (perl, lua, awk, sed, etc.)
    let heuristic = detect(content);

    // For all language/format tags, only accept if file-identify agrees with
    // heuristic content analysis. This prevents extension-based false positives.
    if tag_set.contains(&"python") && heuristic == ContentType::Python {
        return ContentType::Python;
    }
    if tag_set.contains(&"javascript") && heuristic == ContentType::JavaScript {
        return ContentType::JavaScript;
    }
    if tag_set.contains(&"typescript") && heuristic == ContentType::TypeScript {
        return ContentType::TypeScript;
    }
    if tag_set.contains(&"ruby") && heuristic == ContentType::Ruby {
        return ContentType::Ruby;
    }
    if tag_set.contains(&"php") && heuristic == ContentType::Php {
        return ContentType::Php;
    }
    if tag_set.contains(&"shell") && heuristic == ContentType::Shell {
        return ContentType::Shell;
    }
    if (tag_set.contains(&"bash") || tag_set.contains(&"zsh")) && heuristic == ContentType::Shell {
        return ContentType::Shell;
    }
    if tag_set.contains(&"json") && heuristic == ContentType::Json {
        return ContentType::Json;
    }
    if tag_set.contains(&"xml") && heuristic == ContentType::Xml {
        return ContentType::Xml;
    }
    if tag_set.contains(&"html") && heuristic == ContentType::Html {
        return ContentType::Html;
    }
    if tag_set.contains(&"yaml") && heuristic == ContentType::Yaml {
        return ContentType::Yaml;
    }
    if tag_set.contains(&"toml") && heuristic == ContentType::Toml {
        return ContentType::Toml;
    }
    if tag_set.contains(&"markdown") && heuristic == ContentType::Markdown {
        return ContentType::Markdown;
    }
    if tag_set.contains(&"csv") && heuristic == ContentType::Csv {
        return ContentType::Csv;
    }
    if tag_set.contains(&"sql") && heuristic == ContentType::Sql {
        return ContentType::Sql;
    }

    heuristic
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_detect_empty() {
        assert_eq!(detect(""), ContentType::PlainText);
        assert_eq!(detect("   "), ContentType::PlainText);
    }

    #[test]
    fn test_detect_json_object() {
        assert_eq!(detect(r#"{"key": "value"}"#), ContentType::Json);
    }

    #[test]
    fn test_detect_json_array() {
        assert_eq!(detect("[1,2,3]"), ContentType::Json);
    }

    #[test]
    fn test_detect_xml() {
        assert_eq!(detect("<root>text</root>"), ContentType::Xml);
        
        // 伪XML：多个顶级元素
        let pseudo_xml = "<user>John</user> said <message>Hello</message>";
        let result = detect(pseudo_xml);
        eprintln!("伪XML检测结果: {:?}", result);
        assert_eq!(result, ContentType::PlainText);
    }

    #[test]
    fn test_detect_html() {
        assert_eq!(detect("<html><body></body></html>"), ContentType::Html);
        assert_eq!(detect("<!DOCTYPE html><html></html>"), ContentType::Html);
    }

    #[test]
    fn test_detect_html_fragment() {
        // 新增测试：HTML片段应该被正确识别（之前的bug）
        assert_eq!(
            detect("<div class=\"container\">\n  <p>Hello</p>\n</div>"),
            ContentType::Html
        );
    }

    #[test]
    fn test_detect_python() {
        assert_eq!(detect("def main():\n    pass"), ContentType::Python);
        assert_eq!(detect("import os"), ContentType::Python);
    }

    #[test]
    fn test_detect_python_shebang() {
        assert_eq!(detect("#!/usr/bin/env python3\nprint('hi')"), ContentType::Python);
        assert_eq!(detect("#!/usr/bin/python\nprint('hi')"), ContentType::Python);
    }

    #[test]
    fn test_detect_javascript() {
        assert_eq!(detect("function foo() {}"), ContentType::JavaScript);
        assert_eq!(detect("const x = 1;"), ContentType::JavaScript);
    }

    #[test]
    fn test_detect_node_shebang() {
        assert_eq!(detect("#!/usr/bin/env node\nconsole.log('hi')"), ContentType::JavaScript);
    }

    #[test]
    fn test_detect_typescript() {
        assert_eq!(detect("const x: string = 'hello'"), ContentType::TypeScript);
        assert_eq!(detect("interface Foo { bar: string }"), ContentType::TypeScript);
    }

    #[test]
    fn test_detect_rust() {
        assert_eq!(
            detect("fn main() {\n    println!(\"Hello\");\n}"),
            ContentType::Rust
        );
    }

    #[test]
    fn test_detect_go() {
        assert_eq!(
            detect("package main\n\nimport \"fmt\"\n\nfunc main() {\n    fmt.Println(\"hello\")\n}"),
            ContentType::Go
        );
    }

    #[test]
    fn test_detect_java() {
        assert_eq!(
            detect("public class Main {\n    public static void main(String[] args) {\n    }\n}"),
            ContentType::Java
        );
    }

    #[test]
    fn test_detect_sql() {
        assert_eq!(detect("SELECT * FROM users"), ContentType::Sql);
    }

    #[test]
    fn test_detect_yaml() {
        // 单文档YAML
        assert_eq!(detect("---\nkey: value"), ContentType::Yaml);
        
        // YAML多文档
        let multi_doc = "---\ndebug: true\nlevel: info\n---\ntest: value";
        let result = detect(multi_doc);
        eprintln!("YAML多文档检测结果: {:?}", result);
        assert_eq!(result, ContentType::Yaml);
        
        // 伪YAML（不应误判为YAML）
        let pseudo_yaml = "---\n[10:55:57.046] activate_window: 0.000s\n";
        let pseudo_result = detect(pseudo_yaml);
        eprintln!("伪YAML检测结果: {:?}", pseudo_result);
        assert_eq!(pseudo_result, ContentType::PlainText);
    }

    #[test]
    fn test_detect_toml() {
        assert_eq!(detect("[section]\nkey = \"value\""), ContentType::Toml);
    }

    #[test]
    fn test_detect_csv() {
        assert_eq!(detect("name,age\nAlice,30"), ContentType::Csv);
    }

    #[test]
    fn test_detect_markdown() {
        assert_eq!(detect("# Title\n\nSome text"), ContentType::Markdown);
    }

    #[test]
    fn test_detect_binary() {
        assert_eq!(detect("hello\0world"), ContentType::Binary);
    }

    // ===== 回归测试：验证之前失败的边界情况已修复 =====

    #[test]
    fn test_powershell_not_mistaken_as_javascript() {
        // 修复：PowerShell脚本不应被误判为JavaScript
        let ps_script = r#"
# PowerShell script
Write-Host "Hello"
$var = Get-Process
"#;
        assert_eq!(detect(ps_script), ContentType::PowerShell);
    }

    #[test]
    fn test_markdown_with_code_block() {
        // 修复：包含JS代码块的Markdown应被识别为Markdown
        let md_content = "# Title\n\n```javascript\nconst x = 1;\n```\n\nSome text";
        assert_eq!(detect(md_content), ContentType::Markdown);
    }

    #[test]
    fn test_ruby_not_mistaken_as_python() {
        // 修复：Ruby方法定义不应被误判为Python
        let ruby_code = "def greet(name)\n  puts \"Hello #{name}\"\nend";
        assert_eq!(detect(ruby_code), ContentType::Ruby);
    }

    #[test]
    fn test_php_not_mistaken_as_javascript() {
        // 修复：PHP函数不应被误判为JavaScript
        let php_code = "<?php\nfunction hello($name) {\n    echo \"Hello $name\";\n}\n?>";
        assert_eq!(detect(php_code), ContentType::Php);
    }

    #[test]
    fn test_batch_file_detection() {
        assert_eq!(
            detect("@echo off\necho Hello World"),
            ContentType::Batch
        );
    }

    #[test]
    fn test_shell_script_detection() {
        assert_eq!(
            detect("#!/bin/bash\necho 'Hello World'"),
            ContentType::Shell
        );
    }

    #[test]
    fn test_csharp_detection() {
        assert_eq!(
            detect("using System;\nclass Program {\n    static void Main(string[] args) {}\n}"),
            ContentType::CSharp
        );
    }

    #[test]
    fn test_c_detection() {
        assert_eq!(
            detect("#include <stdio.h>\nint main() {\n    return 0;\n}"),
            ContentType::C
        );
    }

    #[test]
    fn test_cpp_detection() {
        assert_eq!(
            detect("#include <iostream>\nint main() {\n    std::cout << \"Hello\";\n    return 0;\n}"),
            ContentType::Cpp
        );
    }

    // check_match 测试
    #[test]
    fn test_check_match_similar_text_formats() {
        // .log, .txt, .md, .markdown 之间互相匹配，不再警告
        let ct_plain = ContentType::PlainText; // 建议 ["txt", "md"]
        let ct_md = ContentType::Markdown;      // 建议 ["md", "markdown"]
        
        // PlainText 检测结果配 .log 扩展名
        assert!(check_match(&ct_plain, Path::new("test.log")).is_ok());
        // PlainText 检测结果配 .txt 扩展名
        assert!(check_match(&ct_plain, Path::new("test.txt")).is_ok());
        // Markdown 检测结果配 .log 扩展名
        assert!(check_match(&ct_md, Path::new("test.log")).is_ok());
        // Markdown 检测结果配 .txt 扩展名
        assert!(check_match(&ct_md, Path::new("test.txt")).is_ok());
    }

    #[test]
    fn test_check_match_different_formats_still_warn() {
        // 不同类别的格式仍然应该警告
        let ct_json = ContentType::Json; // 建议 ["json"]
        let ct_xml = ContentType::Xml;   // 建议 ["xml"]
        
        // JSON 检测结果配 .txt 扩展名 - 应该警告
        assert!(check_match(&ct_json, Path::new("test.txt")).is_err());
        // XML 检测结果配 .json 扩展名 - 应该警告
        assert!(check_match(&ct_xml, Path::new("data.json")).is_err());
    }
}
