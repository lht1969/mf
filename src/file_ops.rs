use crate::encoding;
use crate::error::MfError;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

static TEMP_FILES: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Register a temp file path for cleanup on signal
pub fn register_temp_file(path: PathBuf) {
    if let Ok(mut files) = TEMP_FILES.lock() {
        files.push(path);
    }
}

/// Unregister a temp file (called after successful rename)
pub fn unregister_temp_file(path: &Path) {
    if let Ok(mut files) = TEMP_FILES.lock() {
        files.retain(|p| p != path);
    }
}

/// Clean up all registered temp files (called on signal)
pub fn cleanup_temp_files() {
    if let Ok(mut files) = TEMP_FILES.lock() {
        for path in files.drain(..) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

fn file_system_error(reason: impl Into<String>, path: &Path, source: std::io::Error) -> MfError {
    MfError::FileSystem {
        reason: reason.into(),
        path: path.to_path_buf(),
        source: Some(source),
    }
}

pub fn create_empty(path: &Path) -> Result<(), MfError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| file_system_error("Failed to create parent directories", parent, e))?;
    }
    std::fs::File::create(path)
        .map_err(|e| file_system_error("Failed to create file", path, e))?;
    Ok(())
}

pub fn write_file(path: &Path, content: &str, encoding: &str) -> Result<(), MfError> {
    let data = encoding::encode_string(content, encoding)?;
    atomic_write(path, &data)
}

pub fn append_file(path: &Path, content: &str, encoding: &str) -> Result<(), MfError> {
    let data = encoding::encode_string(content, encoding)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| file_system_error("Failed to create parent directories", parent, e))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| file_system_error("Failed to open file for appending", path, e))?;
    file.write_all(&data)
        .map_err(|e| file_system_error("Failed to write to file", path, e))?;
    Ok(())
}

pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), MfError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_name = format!(".mf_tmp_{}", timestamp);
    let tmp_path = dir.join(&tmp_name);

    // Register for cleanup before writing
    register_temp_file(tmp_path.clone());

    let result = (|| -> Result<(), MfError> {
        std::fs::write(&tmp_path, data)
            .map_err(|e| file_system_error("Failed to write temp file", &tmp_path, e))?;
        let _ = std::fs::remove_file(path);
        std::fs::rename(&tmp_path, path)
            .map_err(|e| file_system_error("Failed to rename temp file", path, e))?;
        Ok(())
    })();

    if result.is_ok() {
        unregister_temp_file(&tmp_path);
    } else {
        let _ = std::fs::remove_file(&tmp_path);
        unregister_temp_file(&tmp_path);
    }

    result
}

pub fn exists(path: &Path) -> bool {
    path.exists()
}

pub fn create_backup(path: &Path) -> Result<(), MfError> {
    let mut backup_name = path.as_os_str().to_os_string();
    backup_name.push(".bak");
    let backup_path = std::path::PathBuf::from(backup_name);
    std::fs::copy(path, &backup_path)
        .map_err(|e| file_system_error("Failed to create backup", path, e))?;
    Ok(())
}

pub fn validate_path(path: &Path) -> Result<(), MfError> {
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err(MfError::InvalidArgument(format!(
            "Path '{}' contains '..' which is not allowed",
            path_str
        )));
    }
    Ok(())
}

pub fn open_with_default(path: &Path) -> Result<(), MfError> {
    open::that(path)
        .map_err(|e| MfError::Internal(format!("Failed to open '{}': {}", path.display(), e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.txt");

        create_empty(&path).unwrap();
        assert!(path.exists());
        assert_eq!(path.metadata().unwrap().len(), 0);
    }

    #[test]
    fn test_write_and_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hello.txt");

        write_file(&path, "hello", "utf8").unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_append() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("append.txt");

        write_file(&path, "line1\n", "utf8").unwrap();
        append_file(&path, "line2\n", "utf8").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    #[test]
    fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("atomic.txt");

        atomic_write(&path, b"atomic data").unwrap();
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "atomic data");
    }

    #[test]
    fn test_atomic_write_no_tmp_left() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("clean.txt");

        atomic_write(&path, b"clean").unwrap();

        let entries = fs::read_dir(dir.path()).unwrap();
        for entry in entries {
            let name = entry.unwrap().file_name();
            let name_str = name.to_string_lossy();
            assert!(
                !name_str.starts_with(".mf_tmp_"),
                "Temp file left behind: {}",
                name_str
            );
        }
    }

    #[test]
    fn test_validate_path_rejection() {
        assert!(validate_path(Path::new("normal.txt")).is_ok());
        assert!(validate_path(Path::new("dir/file.txt")).is_ok());
        assert!(validate_path(Path::new("/absolute/path")).is_ok());

        assert!(validate_path(Path::new("../escape.txt")).is_err());
        assert!(validate_path(Path::new("dir/../../bad.txt")).is_err());
        assert!(validate_path(Path::new("..")).is_err());
    }

    #[test]
    fn test_create_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("original.txt");

        write_file(&path, "backup me", "utf8").unwrap();
        create_backup(&path).unwrap();

        let backup_path = {
            let mut name = path.as_os_str().to_os_string();
            name.push(".bak");
            std::path::PathBuf::from(name)
        };
        assert!(backup_path.exists());
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, "backup me");
    }

    #[test]
    fn test_temp_file_registration() {
        use std::path::PathBuf;
        let p = PathBuf::from("test_tmp_123.tmp");
        register_temp_file(p.clone());

        {
            let files = TEMP_FILES.lock().unwrap();
            assert!(files.contains(&p));
        }

        unregister_temp_file(&p);

        {
            let files = TEMP_FILES.lock().unwrap();
            assert!(!files.contains(&p));
        }
    }

    #[test]
    fn test_cleanup_temp_files() {
        use std::path::PathBuf;
        let p1 = PathBuf::from("cleanup_test_1.tmp");
        let p2 = PathBuf::from("cleanup_test_2.tmp");
        register_temp_file(p1);
        register_temp_file(p2);

        cleanup_temp_files();

        let files = TEMP_FILES.lock().unwrap();
        assert!(files.is_empty());
    }
}
