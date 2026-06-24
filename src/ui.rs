use colored::*;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io::{self, Write};

pub fn success(msg: &str) {
    println!("{}", msg.green());
}

#[allow(dead_code)]
pub fn error(msg: &str, reason: &str, suggestion: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "{}", format!("❌ 错误: {}", msg).red());
    let _ = writeln!(handle, "{}", format!("原因: {}", reason).yellow());
    let _ = writeln!(handle, "{}", format!("建议: {}", suggestion).cyan());
}

pub fn warn(msg: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "{}", format!("⚠️  警告: {}", msg).yellow());
}

#[allow(dead_code)]
pub fn debug(msg: &str) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    let _ = writeln!(handle, "{}", format!("[DEBUG] {}", msg).dimmed());
}

pub fn info(msg: &str) {
    println!("{}", msg);
}

pub fn verbose(msg: &str, is_verbose: bool) {
    if is_verbose {
        println!("{}", msg.dimmed());
    }
}

pub fn preview(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_chars).collect();
        let total = content.chars().count();
        format!("{}\n[已截断，共 {} 字符]", truncated, total)
    }
}

/// 生成剪贴板预览报告（用于 --preview 功能）
///
/// # 参数
/// - `content`: 剪贴板文本内容
/// - `results`: 内容类型检测结果列表（按置信度降序）
/// - `max_preview_chars`: 内容预览最大字符数
/// - `verbose`: 是否显示详细模式（包含匹配信号等信息）
///
/// # 返回值
/// 格式化后的完整预览报告字符串
pub fn preview_report(content: &str, results: &[crate::content_type::PreviewResult], max_preview_chars: usize, verbose: bool) {
    use colored::*;

    // 报告标题
    println!("{}", "剪贴板内容预览".cyan().bold());
    println!("{}", "═".repeat(40).cyan());

    // 基本信息
    let char_count = content.chars().count();
    let byte_count = content.len();
    println!("内容长度: {} 字符 ({} 字节)", char_count, byte_count);

    // 编码检测提示
    if !content.is_ascii() {
        println!("{}", "编码检测: 可能包含非 ASCII 字符".dimmed());
    }

    println!();

    // 检测结果列表
    println!("{}", "检测结果 (按置信度排序):".bold());
    for (i, r) in results.iter().enumerate() {
        // 格式化扩展名为点分隔的字符串
        let ext_str: String = r.suggested_extensions
            .iter()
            .map(|e| format!(".{}", e))
            .collect::<Vec<_>>()
            .join(", ");

        // 置信度颜色：高(绿) / 中(黄) / 低(红/暗)
        let conf_color = if r.confidence >= 0.7 {
            "green"
        } else if r.confidence >= 0.3 {
            "yellow"
        } else {
            "dimmed"
        };

        println!(
            "  {}. {:<12} 置信度: {:<6} 建议扩展名: {}",
            i + 1,
            r.content_type.name(),
            match conf_color {
                "green" => format!("{:.2}", r.confidence).green().to_string(),
                "yellow" => format!("{:.2}", r.confidence).yellow().to_string(),
                _ => format!("{:.2}", r.confidence).dimmed().to_string(),
            },
            if ext_str.is_empty() { "(无)".dimmed().to_string() } else { ext_str },
        );
    }

    println!();

    // 内容预览区域
    println!("{}", "内容预览:".bold());
    let preview_text = preview(content, max_preview_chars);
    // 缩进显示预览内容，每行前加两个空格
    for line in preview_text.lines() {
        println!("  {}", line);
    }

    // 详细模式：显示额外信息
    if verbose && !results.is_empty() {
        println!();
        println!("{}", "最佳匹配:".bold().green());
        let best = &results[0];
        println!(
            "  类型: {} | 置信度: {:.2} | 建议: {}",
            best.content_type.name().green(),
            best.confidence,
            best.suggested_extensions.iter()
                .map(|e| format!(".{}", e))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

/// 用户选择函数（支持 ESC 取消、Enter 确认默认值）
///
/// # 参数
/// - `prompt`: 提示文本
/// - `options`: 选项列表，每个选项是 (键名, 描述) 元组
/// - `default_choice`: 默认选中的选项索引（None 表示无默认值）
/// # 返回值
/// - `Some(索引)`: 用户选择的选项索引
/// - `None`: 用户按下 ESC 或 Ctrl+C（取消操作）
pub fn choose(prompt: &str, options: &[(&str, &str)], default_choice: Option<usize>) -> Option<usize> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", prompt);

    for (i, (key, desc)) in options.iter().enumerate() {
        // 标记默认选项
        if Some(i) == default_choice {
            let _ = writeln!(handle, "  {}. {} - {} [默认]", i + 1, key, desc);
        } else {
            let _ = writeln!(handle, "  {}. {} - {}", i + 1, key, desc);
        }
    }

    // 显示提示信息，包含操作说明
    match default_choice {
        Some(idx) => {
            let _ = write!(
                handle,
                "请选择 (1-{}/Enter=默认[{}]/ESC=取消): ",
                options.len(),
                idx + 1
            );
        }
        None => {
            let _ = write!(
                handle,
                "请选择 (1-{}/ESC=取消): ",
                options.len()
            );
        }
    }
    let _ = handle.flush();

    // 使用 crossterm 逐事件读取键盘输入（支持实时检测 ESC/Enter）
    let mut input_buf = String::new();
    loop {
        // 轮询等待键盘事件
        match event::read() {
            Ok(Event::Key(key)) => {
                // 只处理按下事件，忽略重复和释放事件
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Esc => {
                        // ESC 键：取消操作
                        let _ = writeln!(handle, "\n[已取消]");
                        return None;
                    }
                    KeyCode::Enter => {
                        // Enter 键：确认输入或使用默认值
                        println!(); // 换行
                        if input_buf.is_empty() {
                            // 空输入时返回默认值
                            return default_choice;
                        }
                        // 解析数字输入
                        let num: usize = input_buf.parse().ok()?;
                        if num >= 1 && num <= options.len() {
                            return Some(num - 1);
                        }
                        return None; // 无效数字
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        // 数字键：追加到输入缓冲区并回显
                        input_buf.push(c);
                        let _ = write!(handle, "{}", c);
                        let _ = handle.flush();
                    }
                    KeyCode::Backspace => {
                        // 退格键：删除最后一个字符
                        if !input_buf.is_empty() {
                            input_buf.pop();
                            let _ = write!(handle, "\x08 \x08"); // 光标回退、擦除、再回退
                            let _ = handle.flush();
                        }
                    }
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        // Ctrl+C：取消操作
                        let _ = writeln!(handle, "\n[已中断]");
                        return None;
                    }
                    _ => {}
                }
            }
            Err(_) => {
                // 读取失败（如非 TTY 环境），降级为行缓冲模式
                return choose_fallback(options.len(), default_choice);
            }
            // 忽略其他事件类型（鼠标、窗口大小变化等）
            _ => {}
        }
    }
}

/// 降级方案：当终端不支持事件模式时使用标准行缓冲输入
fn choose_fallback(option_count: usize, default_choice: Option<usize>) -> Option<usize> {
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return None;
    }

    let trimmed = input.trim();

    // 空输入：使用默认值
    if trimmed.is_empty() {
        return default_choice;
    }

    // 解析数字输入
    let num: usize = trimmed.parse().ok()?;

    if num >= 1 && num <= option_count {
        Some(num - 1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_short() {
        let result = preview("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_preview_truncated() {
        let result = preview("hello world this is a long string", 10);
        assert!(result.starts_with("hello worl"));
        assert!(result.contains("[已截断，共 "));
        assert!(result.contains("字符]"));
    }

    #[test]
    fn test_preview_empty() {
        let result = preview("", 10);
        assert_eq!(result, "");
    }

    #[test]
    fn test_preview_exact() {
        let result = preview("exactly", 7);
        assert_eq!(result, "exactly");
    }
}
