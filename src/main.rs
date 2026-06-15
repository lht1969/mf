mod args;
mod clipboard;
mod conflict;
mod config;
mod content_type;
mod encoding;
mod error;
mod file_ops;
mod ui;
mod platform;

use args::Args;
use clap::Parser;
use config::Config;
use conflict::ConflictAction;
use conflict::ConflictMode;
use error::MfError;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

fn get_content(args: &Args) -> Result<Option<String>, MfError> {
    if args.from_clipboard {
        let cb = clipboard::Clipboard::new()?;
        match cb.read_text() {
            Ok(text) => {
                if text.trim().is_empty() {
                    ui::warn("剪切板文本内容为空");
                }
                return Ok(Some(text));
            }
            Err(_) => {
                // Clipboard may contain an image; return None and let
                // process_file try read_image() later.
                return Ok(None);
            }
        }
    }

    {
        use std::io::IsTerminal;
        let mut buf = Vec::new();
        let mut stdin = std::io::stdin().lock();
        if !stdin.is_terminal() && stdin.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
            // Try UTF-8 first (most common for modern pipes)
            if let Ok(text) = String::from_utf8(buf.clone()) {
                if !text.trim().is_empty() {
                    return Ok(Some(text));
                }
            }

            // Try guessed encoding via chardetng, then fallback encodings.
            // PowerShell/cmd.exe pipes ANSI bytes (GBK on zh-CN, Shift_JIS on ja-JP…)
            // chardetng may not detect encoding reliably from very short input,
            // so try common Windows ANSI encodings in addition.
            let candidates = [
                encoding::detect_from_bytes(&buf),
            ];
            let fallbacks = ["gbk", "gb18030", "shift_jis", "euc-kr", "big5", "iso-2022-jp"];

            for label in candidates.iter().chain(fallbacks.iter()) {
                if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
                    let (cow, _, had_errors) = enc.decode(&buf);
                    if !had_errors && !cow.trim().is_empty() {
                        return Ok(Some(cow.into_owned()));
                    }
                }
            }

            // Lenient fallback – accept replacement chars
            for label in candidates.iter().chain(fallbacks.iter()) {
                if let Some(enc) = encoding_rs::Encoding::for_label(label.as_bytes()) {
                    let (cow, _, _) = enc.decode(&buf);
                    if !cow.trim().is_empty() {
                        return Ok(Some(cow.into_owned()));
                    }
                }
            }

            // Last resort: lossy UTF-8
            let text = String::from_utf8_lossy(&buf);
            if !text.trim().is_empty() {
                return Ok(Some(text.into_owned()));
            }
        }
    }

    if let Some(from_file) = &args.from_file {
        if args.detect_encoding {
            let raw_bytes = std::fs::read(from_file)?;
            let detected = encoding::detect_from_bytes(&raw_bytes);
            let content = encoding::decode_to_string(&raw_bytes)?;
            if args.verbose && !args.quiet {
                eprintln!("检测到输入编码: {}", detected);
            }
            return Ok(Some(content));
        }
        let mut file = std::fs::File::open(from_file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        return Ok(Some(content));
    }

    Ok(None)
}

fn auto_correct_extension(path: &Path, content_type: &content_type::ContentType) -> Option<PathBuf> {
    let suggested = content_type.suggested_extensions();
    if suggested.is_empty() {
        return None;
    }
    let new_ext = suggested[0];
    let new_path = path.with_extension(new_ext);
    if new_path == path {
        return None;
    }
    Some(new_path)
}

fn resolve_encoding(filename: &Path, config: &Config) -> String {
    let ext = filename
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("bat") | Some("cmd") => config.encodings.bat.clone(),
        Some("ps1") => config.encodings.ps1.clone(),
        Some(ext) => {
            config
                .encodings
                .custom
                .get(ext)
                .cloned()
                .unwrap_or_else(|| config.core.default_encoding.clone())
        }
        None => config.core.default_encoding.clone(),
    }
}

fn resolve_conflict_mode(args: &Args) -> ConflictMode {
    if let Some(ref mode) = args.conflict {
        match mode.as_str() {
            "overwrite" => ConflictMode::Force,
            "skip" => ConflictMode::NoClobber,
            "append" => ConflictMode::Append,
            "rename" => ConflictMode::Rename,
            _ => ConflictMode::Interactive,
        }
    } else if args.force {
        ConflictMode::Force
    } else if args.no_clobber {
        ConflictMode::NoClobber
    } else {
        ConflictMode::Interactive
    }
}

fn process_file(args: &Args, config: &Config, filename: &Path) -> Result<(), MfError> {
    file_ops::validate_path(filename)?;

    let content = get_content(args)?;

    let encoding = if let Some(ref enc) = args.encoding {
        enc.clone()
    } else {
        resolve_encoding(filename, config)
    };

    // Check file size limit
    let max_size = args.max_size.unwrap_or(config.core.max_file_size_mb);
    if let Some(ref text) = content {
        if text.len() > (max_size as usize) * 1024 * 1024 {
            return Err(MfError::InvalidArgument(format!(
                "内容超过大小限制 ({} MB)，请使用 --max-size 增加限制或 --force 跳过",
                max_size
            )));
        }
    }

    if let Some(ref text) = content {
        if !text.trim().is_empty() {
            let ct = content_type::detect_enhanced(text, Some(filename));
            if args.verbose && !args.quiet {
                ui::verbose(&format!("检测到内容类型: {}", ct.name()), false);
            }
            if let Err(e) = content_type::check_match(&ct, filename) {
                if !args.force && args.conflict.is_none() {
                    ui::warn(&format!("{}", e));
                    let suggested_ext = ct.suggested_extensions().first().copied().unwrap_or("");

                    let choices: &[(&str, &str)] = if !suggested_ext.is_empty() {
                        &[
                            ("继续保存", "使用原扩展名"),
                            ("自动修正", &format!("改为 .{}", suggested_ext)),
                            ("查看内容", "预览前 100 字符"),
                            ("取消", "放弃操作"),
                        ]
                    } else {
                        &[
                            ("继续保存", "使用原扩展名"),
                            ("查看内容", "预览前 100 字符"),
                            ("取消", "放弃操作"),
                        ]
                    };
                    // 默认选择"取消"（最后一个选项）
                    let default_cancel = choices.len() - 1;
                    let choice = ui::choose("内容与扩展名不匹配，请选择:", choices, Some(default_cancel));

                    let preview_idx = if suggested_ext.is_empty() { 1 } else { 2 };
                    match choice {
                        Some(0) => {}
                        Some(1) if !suggested_ext.is_empty() => {
                            if let Some(new_path) = auto_correct_extension(filename, &ct) {
                                ui::info(&format!("自动修正扩展名: .{}", suggested_ext));
                                return process_file(args, config, &new_path);
                            }
                        }
                        Some(n) if n == preview_idx => {
                            ui::info(&ui::preview(text, 100));
                            let retry = ui::choose(
                                "请选择:",
                                &[("继续保存", "使用原扩展名"), ("取消", "放弃操作")],
                                Some(1), // 默认选择"取消"
                            );
                            if retry != Some(0) {
                                return Err(MfError::UserCancelled);
                            }
                        }
                        _ => return Err(MfError::UserCancelled),
                    }
                }
            }
        }
    }

    if file_ops::exists(filename) {
        let mode = resolve_conflict_mode(args);
        let action = conflict::resolve_conflict(filename, mode, args.backup)?;
        match action {
            ConflictAction::Skip => {
                if args.verbose || !args.quiet {
                    ui::verbose(&format!("跳过已存在文件: {}", filename.display()), true);
                }
                return Ok(());
            }
            ConflictAction::Append => {
                if let Some(ref text) = content {
                    file_ops::append_file(filename, text, &encoding)?;
                }
                if !args.quiet {
                    if args.verbose {
                        ui::success(&format!("追加 {}  (编码: {})", filename.display(), encoding));
                    } else {
                        ui::success(&format!("✓ {} (追加)", filename.display()));
                    }
                }
                return Ok(());
            }
            ConflictAction::Rename(new_path) => {
                return process_file(args, config, &new_path);
            }
            ConflictAction::Overwrite => {}
        }
    }

    match content {
        Some(ref text) => {
            if args.append {
                file_ops::append_file(filename, text, &encoding)?;
            } else {
                file_ops::write_file(filename, text, &encoding)?;
            }
        }
        None if args.from_clipboard => {
            let cb = clipboard::Clipboard::new()?;
            let img = cb.read_image().map_err(|_| {
                MfError::Clipboard("剪贴板内容为空或不是受支持的格式（文本 / 图片）".into())
            })?;

            let ext = filename
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_lowercase();

            let format = match ext.as_str() {
                "png" => image::ImageFormat::Png,
                "jpg" | "jpeg" => image::ImageFormat::Jpeg,
                "bmp" => image::ImageFormat::Bmp,
                "gif" => image::ImageFormat::Gif,
                "ico" => image::ImageFormat::Ico,
                "webp" => image::ImageFormat::WebP,
                _ => {
                    return Err(MfError::InvalidArgument(format!(
                        "不支持的图片格式: .{}，支持: png, jpg, bmp, gif, ico, webp",
                        ext
                    )));
                }
            };

            let buf = image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.data)
                .ok_or_else(|| MfError::InvalidArgument("无效的图片数据".into()))?;

            let mut output = std::io::BufWriter::new(std::fs::File::create(filename)?);
            image::DynamicImage::ImageRgba8(buf)
                .write_to(&mut output, format)
                .map_err(|e| MfError::InvalidArgument(format!("图片编码失败: {}", e)))?;

            if !args.quiet {
                if args.verbose {
                    ui::success(&format!(
                        "保存剪贴板图片 {}  ({}×{}, {})",
                        filename.display(),
                        img.width,
                        img.height,
                        ext
                    ));
                } else {
                    ui::success(&format!("✓ {} (图片)", filename.display()));
                }
            }
            // 图片已处理完毕，直接返回，避免重复输出
            return Ok(());
        }
        None => {
            file_ops::create_empty(filename)?;
        }
    }

    if args.open {
        file_ops::open_with_default(filename)?;
    }

    if !args.quiet {
        if args.verbose {
            let action = if content.is_some() { "写入" } else { "创建" };
            ui::success(&format!("{} {}  (编码: {})", action, filename.display(), encoding));
        } else {
            ui::success(&format!("✓ {}", filename.display()));
        }
    }

    Ok(())
}

fn print_encoding_help() {
    println!("mf 编码支持");
    println!("============");
    println!("自动编码（基于文件扩展名）:");
    println!("  .bat, .cmd → GBK (ANSI, Windows 代码页)");
    println!("  .ps1       → UTF-8 with BOM (PowerShell 兼容)");
    println!("  其他       → UTF-8 without BOM");
    println!();
    println!("支持的编码:");
    println!("  utf8    - UTF-8 (无 BOM)");
    println!("  utf8bom - UTF-8 (含 BOM)");
    println!("  gbk     - GBK (Windows 936 代码页)");
    println!("  utf16le - UTF-16 Little Endian");
    println!("  utf16be - UTF-16 Big Endian");
    println!();
    println!("使用: mf <file> --encoding <编码名>");
}

fn print_examples() {
    println!("=== 基本用法 ===");
    println!("  mf newfile.txt                                   创建空文件");
    println!("  mf file1.txt file2.txt file3.txt                 批量创建多个文件");
    println!("  mf --clipboard paste.txt                         从剪切板读取内容写入文件");
    println!();
    println!("=== 内容来源 ===");
    println!("  echo \"Hello\" | mf output.txt                     从管道读取内容");
    println!("  mf --from-file source.txt output.txt             从文件读取内容写入新文件");
    println!("  mf -c file.txt                                   从剪贴板读取文本");
    println!("  mf -c file.png                                   从剪贴板保存图片 (截图后使用)");
    println!();
    println!("=== 编码控制 ===");
    println!("  mf --encoding gbk chinese-file.txt               指定编码 (GBK)");
    println!("  mf --detect-encoding unknown.bin                 检测并显示文件编码");
    println!("  mf --encoding utf8 --from-file sjis.txt out.txt  指定源文件编码");
    println!();
    println!("=== 文件操作 ===");
    println!("  mf --force existing.txt                          强制覆盖已存在文件");
    println!("  mf --no-clobber existing.txt                     跳过已存在文件 (不覆盖)");
    println!("  mf --conflict overwrite file.txt                 参数化覆盖 (同 --force)");
    println!("  mf --conflict skip file.txt                      参数化跳过 (同 --no-clobber)");
    println!("  mf --conflict append file.txt                    参数化追加到已存在文件");
    println!("  mf --conflict rename file.txt                    参数化自动重命名");
    println!("  mf -o created.txt                                创建后用系统默认程序打开");
    println!();
    println!("=== 配置 ===");
    println!("  mf --help-config                                 显示当前配置");
    println!("  mf --max-size 1024 output.txt                    限制最大文件大小 (字节)");
    println!();
    println!("=== 其他 ===");
    println!("  mf --verbose newfile.txt                         显示详细日志");
    println!("  mf --quiet newfile.txt                           静默模式 (无输出)");
    println!("  mf --help-encoding                               显示编码参考");
    println!("  mf --version                                     显示版本信息");
}

fn print_config_help() {
    println!("=== 配置文件优先级 ===");
    println!("  1. 项目配置: <当前目录>/.mf-config.toml");
    println!("  2. 用户配置: %APPDATA%/mf/config.toml (Windows) / ~/.config/mf/config.toml (Linux/macOS)");
    println!("  3. 默认配置: 内置默认值");
    println!();
    println!("=== 配置项示例 (.mf-config.toml) ===");
    println!("  [behavior]");
    println!("  force = false");
    println!("  no_clobber = false");
    println!("  quiet = false");
    println!();
    println!("  [encoding]");
    println!("  default = \"utf8\"");
    println!("  fallback = \"gbk\"");
    println!("  always_add_bom = false");
    println!();
    println!("  [limits]");
    println!("  max_size = 52428800");
    println!();
    println!("  [custom]");
    println!("  ...");
}

fn setup_signal_handler() {
    let result = ctrlc::set_handler(move || {
        file_ops::cleanup_temp_files();
        std::process::exit(130);
    });
    if let Err(e) = result {
        eprintln!("警告: 无法设置信号处理器: {}", e);
    }
}

fn main() {
    setup_signal_handler();
    let config = config::Config::load();
    let args = Args::parse();

    match (args.help_encoding, args.examples, args.help_config) {
        (true, _, _) => {
            print_encoding_help();
            return;
        }
        (_, true, _) => {
            print_examples();
            return;
        }
        (_, _, true) => {
            print_config_help();
            return;
        }
        _ => {}
    }

    for file in &args.files {
        if let Err(e) = process_file(&args, &config, file) {
            if !args.quiet || matches!(&e, MfError::UserCancelled) {
                eprintln!("{}", e);
            }
            std::process::exit(e.exit_code() as i32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_get_content_from_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        fs::write(&src, "hello from file").unwrap();

        let args = Args::parse_from(&["mf", "out.txt", "--from-file", src.to_str().unwrap()]);
        let content = get_content(&args).unwrap();
        assert_eq!(content, Some("hello from file".to_string()));
    }

    #[test]
    fn test_get_content_from_file_with_detect() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.txt");
        fs::write(&src, "hello").unwrap();

        let args = Args::parse_from(&["mf", "out.txt", "--from-file", src.to_str().unwrap(), "--detect-encoding"]);
        let content = get_content(&args).unwrap();
        assert_eq!(content, Some("hello".to_string()));
    }

    #[test]
    fn test_get_content_empty_when_no_source() {
        let args = Args::parse_from(&["mf", "test.txt"]);
        let content = get_content(&args).unwrap();
        assert!(content.is_none());
    }

    #[test]
    fn test_get_content_from_clipboard_flag_no_clipboard() {
        let result = Args::try_parse_from(&["mf", "f.txt", "-c"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_auto_correct_extension_python_to_py() {
        let ct = content_type::ContentType::Python;
        let result = auto_correct_extension(Path::new("script.js"), &ct);
        assert_eq!(result, Some(PathBuf::from("script.py")));
    }

    #[test]
    fn test_auto_correct_extension_already_correct() {
        let ct = content_type::ContentType::Json;
        let result = auto_correct_extension(Path::new("data.json"), &ct);
        assert_eq!(result, None);
    }

    #[test]
    fn test_auto_correct_extension_no_suggested() {
        let ct = content_type::ContentType::Binary;
        let result = auto_correct_extension(Path::new("file.bin"), &ct);
        assert_eq!(result, None);
    }
}
