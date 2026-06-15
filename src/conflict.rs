use crate::error::MfError;
use crate::file_ops;
use crate::ui;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub enum ConflictAction {
    Overwrite,
    Skip,
    Append,
    Rename(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictMode {
    Interactive,
    Force,
    NoClobber,
    Append,
    Rename,
}

pub fn resolve_rename(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    for i in 1..1000 {
        let new_name = format!("{}_{}{}", stem, i, ext);
        let new_path = parent.join(&new_name);
        if !new_path.exists() {
            return new_path;
        }
    }
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    parent.join(format!("{}_{}{}", stem, ts, ext))
}

pub fn resolve_conflict(
    path: &Path,
    mode: ConflictMode,
    backup: bool,
) -> Result<ConflictAction, MfError> {
    if !file_ops::exists(path) {
        return Ok(ConflictAction::Overwrite);
    }

    match mode {
        ConflictMode::NoClobber => Ok(ConflictAction::Skip),
        ConflictMode::Force => {
            if backup {
                file_ops::create_backup(path)?;
            }
            Ok(ConflictAction::Overwrite)
        }
        ConflictMode::Append => Ok(ConflictAction::Append),
        ConflictMode::Rename => Ok(ConflictAction::Rename(resolve_rename(path))),
        ConflictMode::Interactive => {
            // 默认选择"跳过"（索引1），用户按 Enter 直接跳过，按 ESC 取消
            let choice = ui::choose(
                &format!("文件已存在: {}，请选择:", path.display()),
                &[
                    ("覆盖", "覆盖现有文件"),
                    ("跳过", "保留现有文件"),
                    ("追加", "追加到文件末尾"),
                    ("重命名", "自动生成新名称"),
                ],
                Some(1), // 默认选择"跳过"
            );

            match choice {
                Some(0) => {
                    if backup {
                        file_ops::create_backup(path)?;
                    }
                    Ok(ConflictAction::Overwrite)
                }
                Some(1) => Ok(ConflictAction::Skip),
                Some(2) => Ok(ConflictAction::Append),
                Some(3) => Ok(ConflictAction::Rename(resolve_rename(path))),
                _ => Err(MfError::UserCancelled),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_rename_basic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "original").unwrap();
        let renamed = resolve_rename(&path);
        assert_ne!(renamed, path);
        assert!(renamed.to_string_lossy().contains("test_"));
        assert!(renamed.to_string_lossy().ends_with(".txt"));
    }

    #[test]
    fn test_resolve_rename_increment() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "orig").unwrap();
        let r1 = resolve_rename(&path);
        fs::write(&r1, "copy1").unwrap();
        let r2 = resolve_rename(&path);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_resolve_rename_no_ext() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Makefile");
        fs::write(&path, "content").unwrap();
        let renamed = resolve_rename(&path);
        assert!(renamed.to_string_lossy().contains("Makefile_"));
    }

    #[test]
    fn test_resolve_conflict_not_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.txt");
        let action = resolve_conflict(&path, ConflictMode::Interactive, false).unwrap();
        assert!(matches!(action, ConflictAction::Overwrite));
    }

    #[test]
    fn test_resolve_conflict_no_clobber() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("exists.txt");
        fs::write(&path, "data").unwrap();
        let action = resolve_conflict(&path, ConflictMode::NoClobber, false).unwrap();
        assert!(matches!(action, ConflictAction::Skip));
    }

    #[test]
    fn test_resolve_conflict_force() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("force.txt");
        fs::write(&path, "original").unwrap();
        let action = resolve_conflict(&path, ConflictMode::Force, false).unwrap();
        assert!(matches!(action, ConflictAction::Overwrite));
    }

    #[test]
    fn test_resolve_conflict_force_with_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backup_test.txt");
        std::fs::write(&path, "original").unwrap();
        let action = resolve_conflict(&path, ConflictMode::Force, true).unwrap();
        assert!(matches!(action, ConflictAction::Overwrite));
        let backup = dir.path().join("backup_test.txt.bak");
        assert!(backup.exists());
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "original");
    }

    #[test]
    fn test_resolve_conflict_append_mode() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("append_mode.txt");
        fs::write(&path, "original").unwrap();
        let action = resolve_conflict(&path, ConflictMode::Append, false).unwrap();
        assert!(matches!(action, ConflictAction::Append));
    }

    #[test]
    fn test_resolve_conflict_rename_mode() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rename_mode.txt");
        fs::write(&path, "original").unwrap();
        let action = resolve_conflict(&path, ConflictMode::Rename, false).unwrap();
        assert!(matches!(action, ConflictAction::Rename(_)));
        if let ConflictAction::Rename(new_path) = &action {
            assert_ne!(new_path, &path);
        }
    }
}
