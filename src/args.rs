use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mf",
    version,
    about = "多功能文件处理工具",
    after_help = "示例: mf newfile.txt\n查看 --examples 获取更多使用示例"
)]
pub struct Args {
    /// 要创建的文件路径（支持同时指定多个文件）
    #[arg(required_unless_present_any = ["help_encoding", "examples", "help_config"])]
    pub files: Vec<PathBuf>,

    /// 从剪贴板读取内容写入文件（适用于截图后保存图片或复制文本后创建文件）
    #[arg(short = 'c', long = "from-clipboard")]
    pub from_clipboard: bool,

    /// 以追加模式写入，不覆盖已有内容（适用于日志记录等场景）
    #[arg(short = 'a', long = "append")]
    pub append: bool,

    /// 创建文件后自动用系统默认程序打开该文件
    #[arg(short = 'o', long = "open")]
    pub open: bool,

    /// 强制覆盖已存在的文件，不弹出确认提示
    #[arg(short = 'f', long = "force", conflicts_with = "no_clobber")]
    pub force: bool,

    /// 当目标文件已存在时自动跳过，不覆盖也不报错
    #[arg(short = 'n', long = "no-clobber")]
    pub no_clobber: bool,

    /// 覆盖文件前先创建 .bak 备份副本
    #[arg(long = "backup")]
    pub backup: bool,

    /// 设置文件冲突处理策略：overwrite(覆盖) / skip(跳过) / append(追加) / rename(重命名)
    #[arg(
        long = "conflict",
        value_name = "MODE",
        value_parser = ["overwrite", "skip", "append", "rename"]
    )]
    pub conflict: Option<String>,

    /// 指定文件编码格式，如 utf8、gbk、utf16le 等（默认自动检测）
    #[arg(long = "encoding")]
    pub encoding: Option<String>,

    /// 从指定文件读取内容而非剪贴板（与 -c 配合使用时优先从此文件读取）
    #[arg(long = "from-file")]
    pub from_file: Option<PathBuf>,

    /// 仅检测并显示内容的编码格式，不实际创建文件
    #[arg(long = "detect-encoding")]
    pub detect_encoding: bool,

    /// 显示详细信息，包括编码检测结果、图片尺寸、操作过程等
    #[arg(short = 'v', long = "verbose", conflicts_with = "quiet")]
    pub verbose: bool,

    /// 启用调试模式，输出更详细的内部处理信息用于问题排查
    #[arg(long = "debug", conflicts_with = "quiet")]
    pub debug: bool,

    /// 静默模式，不输出任何成功/警告信息（仅输出错误）
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// 显示支持的编码格式列表及详细说明
    #[arg(long = "help-encoding")]
    pub help_encoding: bool,

    /// 显示常用使用示例和典型应用场景
    #[arg(long = "examples")]
    pub examples: bool,

    /// 显示配置文件格式说明及可配置项列表
    #[arg(long = "help-config")]
    pub help_config: bool,

    /// 限制允许创建的文件最大大小（单位：MB），超出则拒绝创建
    #[arg(long = "max-size")]
    pub max_size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_file_no_flags() {
        let args = Args::try_parse_from(["mf", "file.txt"]).unwrap();
        assert_eq!(args.files, vec![PathBuf::from("file.txt")]);
        assert!(!args.from_clipboard);
        assert!(!args.append);
        assert!(!args.verbose);
        assert!(!args.quiet);
    }

    #[test]
    fn test_from_clipboard_flag() {
        let args = Args::try_parse_from(["mf", "-c", "file.txt"]).unwrap();
        assert!(args.from_clipboard);
    }

    #[test]
    fn test_append_flag() {
        let args = Args::try_parse_from(["mf", "-a", "file.txt"]).unwrap();
        assert!(args.append);
    }

    #[test]
    fn test_from_clipboard_and_append_combined() {
        let args = Args::try_parse_from(["mf", "-c", "-a", "file.txt"]).unwrap();
        assert!(args.from_clipboard);
        assert!(args.append);
    }

    #[test]
    fn test_force_and_no_clobber_conflict() {
        let result = Args::try_parse_from(["mf", "--force", "--no-clobber", "file.txt"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_multi_file_parsing() {
        let args = Args::try_parse_from(["mf", "a.txt", "b.txt", "c.txt"]).unwrap();
        assert_eq!(
            args.files,
            vec![
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt"),
                PathBuf::from("c.txt"),
            ]
        );
    }

    #[test]
    fn test_encoding_option() {
        let args = Args::try_parse_from(["mf", "--encoding", "gbk", "file.txt"]).unwrap();
        assert_eq!(args.encoding, Some("gbk".to_string()));
    }

    #[test]
    fn test_version_flag() {
        let result = Args::try_parse_from(["mf", "--version", "file.txt"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_conflict_overwrite() {
        let args = Args::try_parse_from(["mf", "--conflict", "overwrite", "file.txt"]).unwrap();
        assert_eq!(args.conflict.as_deref(), Some("overwrite"));
    }

    #[test]
    fn test_conflict_skip() {
        let args = Args::try_parse_from(["mf", "--conflict", "skip", "file.txt"]).unwrap();
        assert_eq!(args.conflict.as_deref(), Some("skip"));
    }

    #[test]
    fn test_conflict_append() {
        let args = Args::try_parse_from(["mf", "--conflict", "append", "file.txt"]).unwrap();
        assert_eq!(args.conflict.as_deref(), Some("append"));
    }

    #[test]
    fn test_conflict_rename() {
        let args = Args::try_parse_from(["mf", "--conflict", "rename", "file.txt"]).unwrap();
        assert_eq!(args.conflict.as_deref(), Some("rename"));
    }

    #[test]
    fn test_conflict_invalid_value() {
        let result = Args::try_parse_from(["mf", "--conflict", "invalid", "file.txt"]);
        assert!(result.is_err());
    }
}
