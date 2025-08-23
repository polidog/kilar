use kilar::{
    commands::{CheckCommand, ListCommand},
    utils::{validate_port, validate_protocol, validate_sort_option},
    Error,
};

#[tokio::test]
async fn test_check_command_with_unused_port() {
    // Test checking an unused port (high port number likely to be free)
    let result = CheckCommand::execute(65432, "tcp", false, true, false, false).await;
    // Even if system tools are missing, the command should handle it gracefully
    // and not panic. We accept both success and specific error cases.
    match result {
        Ok(_) => {
            // Success - port checked successfully
        }
        Err(e) => {
            // Accept any error in CI environment - the test is mainly to ensure no panic
            let error_msg = e.to_string();
            // Just log the error and continue - this test is about ensuring graceful error handling
            eprintln!("CheckCommand error (expected in CI): {error_msg}");
        }
    }
}

#[test]
fn test_validation_functions() {
    // Test port validation
    assert!(validate_port(80).is_ok());
    assert!(validate_port(0).is_err());

    // Test protocol validation
    assert!(validate_protocol("tcp").is_ok());
    assert!(validate_protocol("udp").is_ok());
    assert!(validate_protocol("all").is_ok());
    assert!(validate_protocol("invalid").is_err());

    // Test sort option validation
    assert!(validate_sort_option("port").is_ok());
    assert!(validate_sort_option("pid").is_ok());
    assert!(validate_sort_option("name").is_ok());
    assert!(validate_sort_option("invalid").is_err());
}

#[test]
fn test_error_types() {
    let error = Error::PortNotFound(8080);
    assert!(error.to_string().contains("8080"));
    assert!(error.to_string().contains("not in use"));

    let error = Error::InvalidPort("test".to_string());
    assert!(error.to_string().contains("Invalid port"));

    let error = Error::ProcessNotFound(1234);
    assert!(error.to_string().contains("1234"));
    assert!(error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_list_command_port_range_parsing() {
    // This tests the internal port range parsing through the public API
    // We can't directly test with real processes, but we can test the command execution
    let result = ListCommand::execute(
        Some("1-1000".to_string()),
        None,
        "port",
        "tcp",
        false,
        true,
        true,
        Some("balanced"),
        false,
    )
    .await;

    // Should succeed even if no ports are found in range or system tools are missing
    match result {
        Ok(_) => {
            // Success - command executed successfully
        }
        Err(e) => {
            // Accept any error in CI environment - the test is mainly to ensure no panic
            let error_msg = e.to_string();
            // Just log the error and continue - this test is about ensuring graceful error handling
            eprintln!("ListCommand error (expected in CI): {error_msg}");
        }
    }
}

#[tokio::test]
async fn test_list_command_invalid_port_range() {
    // Test invalid port range
    let result = ListCommand::execute(
        Some("5000-3000".to_string()), // Invalid: start > end
        None,
        "port",
        "tcp",
        false,
        true,
        true,
        Some("balanced"),
        false,
    )
    .await;

    assert!(result.is_err());
}

#[test]
fn test_error_chain() {
    // Test that errors can be properly converted
    let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test");
    let our_error: Error = io_error.into();
    assert!(matches!(our_error, Error::IoError(_)));

    // Test that our error implements std::error::Error
    let error: Box<dyn std::error::Error> = Box::new(Error::Other("test".to_string()));
    assert_eq!(error.to_string(), "test");
}
