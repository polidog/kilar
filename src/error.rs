use std::fmt;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ParseError(String),
    PortNotFound(u16),
    PermissionDenied(String),
    ProcessNotFound(u32),
    InvalidPort(String),
    CommandFailed(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "I/O エラー: {}", e),
            Error::ParseError(msg) => write!(f, "パースエラー: {}", msg),
            Error::PortNotFound(port) => write!(f, "ポート {} は使用されていません", port),
            Error::PermissionDenied(msg) => write!(f, "権限エラー: {}", msg),
            Error::ProcessNotFound(pid) => write!(f, "プロセス {} が見つかりません", pid),
            Error::InvalidPort(msg) => write!(f, "無効なポート番号: {}", msg),
            Error::CommandFailed(msg) => write!(f, "コマンド実行失敗: {}", msg),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Other(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::ParseError(e.to_string())
    }
}

impl From<dialoguer::Error> for Error {
    fn from(e: dialoguer::Error) -> Self {
        Error::Other(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;