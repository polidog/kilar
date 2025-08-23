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
}
