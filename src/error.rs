use std::fmt;

/// Error types for the kilar application.
///
/// This enum represents all possible errors that can occur during
/// the execution of kilar commands.
#[derive(Debug, Clone)]
pub enum Error {
    /// I/O operation failed
    IoError(String),
    /// Failed to parse data
    ParseError(String),
    /// The specified port is not in use
    PortNotFound(u16),
    /// Operation requires elevated privileges
    PermissionDenied(String),
    /// Process with the specified PID was not found
    ProcessNotFound(u32),
    /// Invalid port number or range
    InvalidPort(String),
    /// System command execution failed
    CommandFailed(String),
    /// Other generic error
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(msg) => write!(f, "I/O error: {msg}"),
            Error::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Error::PortNotFound(port) => write!(f, "Port {port} is not in use"),
            Error::PermissionDenied(msg) => {
                write!(
                    f,
                    "Permission denied: {msg}. Try running with 'sudo' for system processes"
                )
            }
            Error::ProcessNotFound(pid) => write!(f, "Process with PID {pid} not found"),
            Error::InvalidPort(msg) => {
                write!(f, "Invalid port: {msg}. Port must be between 1 and 65535")
            }
            Error::CommandFailed(msg) => {
                if msg.contains("lsof") || msg.contains("netstat") {
                    write!(
                        f,
                        "Command failed: {msg}. Make sure required system tools are installed"
                    )
                } else {
                    write!(f, "Command execution failed: {msg}")
                }
            }
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e.to_string())
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

/// A specialized `Result` type for kilar operations.
///
/// This is a convenience type alias for `std::result::Result<T, Error>`
/// where the error type is always `kilar::Error`.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn test_error_display() {
        let err = Error::PortNotFound(8080);
        assert_eq!(err.to_string(), "Port 8080 is not in use");

        let err = Error::InvalidPort("65536".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid port: 65536. Port must be between 1 and 65535"
        );

        let err = Error::ProcessNotFound(1234);
        assert_eq!(err.to_string(), "Process with PID 1234 not found");
    }

    #[test]
    fn test_permission_denied_message() {
        let err = Error::PermissionDenied("Operation not permitted".to_string());
        assert!(err.to_string().contains("sudo"));
        assert!(err.to_string().contains("Permission denied"));
    }

    #[test]
    fn test_command_failed_special_cases() {
        let err = Error::CommandFailed("lsof not found".to_string());
        assert!(err
            .to_string()
            .contains("Make sure required system tools are installed"));

        let err = Error::CommandFailed("netstat error".to_string());
        assert!(err
            .to_string()
            .contains("Make sure required system tools are installed"));

        let err = Error::CommandFailed("generic error".to_string());
        assert_eq!(err.to_string(), "Command execution failed: generic error");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::IoError(_)));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_str = "invalid json";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::ParseError(_)));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("test error");
        let err: Error = anyhow_err.into();
        assert!(matches!(err, Error::Other(_)));
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_error_debug_format() {
        let err = Error::PortNotFound(3000);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("PortNotFound"));
        assert!(debug_str.contains("3000"));
    }

    #[test]
    fn test_error_clone() {
        let err = Error::ProcessNotFound(1234);
        let cloned = err.clone();

        assert_eq!(err.to_string(), cloned.to_string());
        match (err, cloned) {
            (Error::ProcessNotFound(pid1), Error::ProcessNotFound(pid2)) => {
                assert_eq!(pid1, pid2);
            }
            _ => panic!("Clone failed or types don't match"),
        }
    }

    #[test]
    fn test_all_error_variants_display() {
        let test_cases = vec![
            (
                Error::IoError("file not found".to_string()),
                "I/O error: file not found",
            ),
            (
                Error::ParseError("invalid format".to_string()),
                "Parse error: invalid format",
            ),
            (Error::PortNotFound(8080), "Port 8080 is not in use"),
            (
                Error::PermissionDenied("access denied".to_string()),
                "Permission denied: access denied. Try running with 'sudo' for system processes",
            ),
            (
                Error::ProcessNotFound(9999),
                "Process with PID 9999 not found",
            ),
            (
                Error::InvalidPort("99999".to_string()),
                "Invalid port: 99999. Port must be between 1 and 65535",
            ),
            (
                Error::CommandFailed("general failure".to_string()),
                "Command execution failed: general failure",
            ),
            (Error::Other("custom error".to_string()), "custom error"),
        ];

        for (error, expected) in test_cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn test_command_failed_tool_detection() {
        let test_cases = vec![
            ("lsof: command not found", true),
            ("netstat: permission denied", true),
            ("ss: not found", false),
            ("generic command error", false),
            ("failed to execute lsof", true),
            ("netstat output parsing failed", true),
            ("unrelated error message", false),
        ];

        for (message, should_contain_install_hint) in test_cases {
            let err = Error::CommandFailed(message.to_string());
            let error_string = err.to_string();

            if should_contain_install_hint {
                assert!(
                    error_string.contains("Make sure required system tools are installed"),
                    "Error '{}' should contain install hint",
                    message
                );
            } else {
                assert!(
                    error_string.contains("Command execution failed"),
                    "Error '{}' should contain generic failure message",
                    message
                );
            }
        }
    }

    #[test]
    fn test_error_as_std_error_trait() {
        let err = Error::PortNotFound(3000);

        // std::error::Errorトレイトを実装していることを確認
        let _error_trait: &dyn std::error::Error = &err;

        // sourceメソッドが呼び出せることを確認（Noneを返すが）
        assert!(StdError::source(&err).is_none());
    }

    #[test]
    fn test_result_type_alias() {
        // Result型エイリアスが正しく動作することを確認
        fn test_function() -> Result<i32> {
            Ok(42)
        }

        fn test_function_error() -> Result<i32> {
            Err(Error::Other("test error".to_string()))
        }

        assert_eq!(test_function().unwrap(), 42);
        assert!(test_function_error().is_err());
    }

    #[test]
    fn test_error_chain_conversions() {
        // 連続的な変換をテスト
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let kilar_error: Error = io_error.into();

        match kilar_error {
            Error::IoError(msg) => {
                assert!(msg.contains("access denied"));
                // "permission denied"は大文字小文字が異なる可能性がある
                let msg_lower = msg.to_lowercase();
                assert!(
                    msg_lower.contains("permission denied") || msg_lower.contains("access denied")
                );
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_error_message_consistency() {
        // エラーメッセージが一貫していることを確認
        let port_errors = vec![
            Error::PortNotFound(80),
            Error::PortNotFound(443),
            Error::PortNotFound(65535),
        ];

        for err in port_errors {
            let msg = err.to_string();
            assert!(msg.starts_with("Port"));
            assert!(msg.ends_with("is not in use"));
        }

        let process_errors = vec![
            Error::ProcessNotFound(1),
            Error::ProcessNotFound(1000),
            Error::ProcessNotFound(99999),
        ];

        for err in process_errors {
            let msg = err.to_string();
            assert!(msg.starts_with("Process with PID"));
            assert!(msg.ends_with("not found"));
        }
    }

    #[test]
    fn test_error_edge_cases() {
        // エッジケースをテスト
        let edge_cases = vec![
            Error::PortNotFound(1),                    // 最小ポート
            Error::PortNotFound(65535),                // 最大ポート
            Error::ProcessNotFound(0),                 // PID 0
            Error::ProcessNotFound(u32::MAX),          // 最大PID
            Error::InvalidPort("".to_string()),        // 空文字列
            Error::InvalidPort("0".to_string()),       // 無効な最小値
            Error::InvalidPort("65536".to_string()),   // 無効な最大値
            Error::Other("Unknown error".to_string()), // 空でないその他エラー
        ];

        // すべてのエッジケースでto_stringが動作することを確認
        for err in edge_cases {
            let msg = err.to_string();
            assert!(
                !msg.is_empty(),
                "Error message should not be empty: {:?}",
                err
            );
        }
    }

    #[test]
    fn test_permission_denied_variants() {
        // 異なるPermissionDeniedメッセージのテスト
        let permission_messages = vec![
            "Operation not permitted",
            "Access denied",
            "Insufficient privileges",
            "", // 空メッセージ
        ];

        for msg in permission_messages {
            let err = Error::PermissionDenied(msg.to_string());
            let error_str = err.to_string();

            assert!(error_str.contains("Permission denied"));
            assert!(error_str.contains("sudo"));
            assert!(error_str.contains("system processes"));
        }
    }

    #[test]
    fn test_parse_error_variants() {
        // 異なるParseErrorのケースをテスト
        let parse_cases = vec![
            "JSON parsing failed",
            "Invalid number format",
            "Unexpected character",
            "EOF while parsing",
        ];

        for case in parse_cases {
            let err = Error::ParseError(case.to_string());
            let error_str = err.to_string();

            assert!(error_str.starts_with("Parse error: "));
            assert!(error_str.contains(case));
        }
    }

    #[test]
    fn test_io_error_variants() {
        // 異なるI/O Errorのケースをテスト
        let io_cases = vec![
            std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"),
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused"),
            std::io::Error::new(std::io::ErrorKind::TimedOut, "operation timed out"),
        ];

        for io_err in io_cases {
            let error_kind = io_err.kind();
            let kilar_err: Error = io_err.into();

            match kilar_err {
                Error::IoError(msg) => {
                    // メッセージに元のエラー情報が含まれていることを確認
                    assert!(!msg.is_empty());
                    // 特定のエラータイプでの特別な処理を確認
                    match error_kind {
                        std::io::ErrorKind::NotFound => {
                            assert!(msg.to_lowercase().contains("not found"))
                        }
                        std::io::ErrorKind::PermissionDenied => {
                            assert!(msg.to_lowercase().contains("permission"))
                        }
                        _ => {} // その他のケースは特別な処理なし
                    }
                }
                _ => panic!("Expected IoError variant"),
            }
        }
    }
}
