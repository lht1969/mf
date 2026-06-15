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
