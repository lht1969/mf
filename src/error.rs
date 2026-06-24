use std::io;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum MfError {
    #[error("提示: {0}")]
    InvalidArgument(String),

    #[error("文件访问遇到问题: {reason} (路径: {path})")]
    FileSystem {
        reason: String,
        path: PathBuf,
        source: Option<io::Error>,
    },

    #[error("剪贴板操作未完成: {0}")]
    Clipboard(String),

    #[error("编码处理遇到问题: {0}")]
    Encoding(String),

    #[error("操作已取消")]
    UserCancelled,

    #[error("程序内部异常: {0}")]
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
