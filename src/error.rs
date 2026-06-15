use std::io;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum MfError {
    #[error("参数错误: {0}")]
    InvalidArgument(String),

    #[error("文件系统错误: {reason} (路径: {path})")]
    FileSystem {
        reason: String,
        path: PathBuf,
        source: Option<io::Error>,
    },

    #[error("剪切板错误: {0}")]
    Clipboard(String),

    #[error("编码错误: {0}")]
    Encoding(String),

    #[error("用户取消操作")]
    UserCancelled,

    #[error("内部错误: {0}")]
    Internal(String),
}

impl MfError {
    pub fn exit_code(&self) -> u8 {
        match self {
            MfError::InvalidArgument(_) => 1,
            MfError::FileSystem { .. } => 2,
            MfError::Clipboard(_) => 3,
            MfError::Encoding(_) => 4,
            MfError::UserCancelled => 5,
            MfError::Internal(_) => 127,
        }
    }
}

impl From<io::Error> for MfError {
    fn from(err: io::Error) -> Self {
        MfError::FileSystem {
            reason: err.to_string(),
            path: PathBuf::new(),
            source: Some(err),
        }
    }
}

#[allow(dead_code)]
pub trait ErrorDisplay {
    fn format_error(&self) -> String;
    fn format_warning(&self) -> String;
}

#[allow(dead_code)]
impl ErrorDisplay for MfError {
    fn format_error(&self) -> String {
        format!("❌ {}", self)
    }

    fn format_warning(&self) -> String {
        format!("⚠️ {}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_exit_codes() {
        assert_eq!(MfError::InvalidArgument("test".into()).exit_code(), 1);
        assert_eq!(
            MfError::FileSystem {
                reason: "not found".into(),
                path: PathBuf::from("/test"),
                source: None,
            }
            .exit_code(),
            2
        );
        assert_eq!(MfError::Clipboard("test".into()).exit_code(), 3);
        assert_eq!(MfError::Encoding("test".into()).exit_code(), 4);
        assert_eq!(MfError::UserCancelled.exit_code(), 5);
        assert_eq!(MfError::Internal("test".into()).exit_code(), 127);
    }

    #[test]
    fn test_error_format() {
        let err = MfError::InvalidArgument("missing flag".into());
        let msg = err.format_error();
        assert!(msg.contains("❌"));
        assert!(msg.contains("参数错误"));

        let err = MfError::UserCancelled;
        let msg = err.format_error();
        assert!(msg.contains("❌"));
        assert!(msg.contains("用户取消"));
    }

    #[test]
    fn test_warning_format() {
        let err = MfError::Internal("deprecated".into());
        let msg = err.format_warning();
        assert!(msg.contains("⚠️"));
        assert!(msg.contains("内部错误"));
    }

    #[test]
    fn test_io_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let mf_err: MfError = io_err.into();
        match mf_err {
            MfError::FileSystem {
                ref reason,
                ref source,
                ..
            } => {
                assert!(reason.contains("file not found"));
                assert!(source.is_some());
            }
            _ => panic!("expected FileSystem variant"),
        }
    }

}
