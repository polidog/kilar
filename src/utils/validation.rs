use crate::Result;

pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(crate::Error::InvalidPort(
            "Port number must be greater than 0".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_protocol(protocol: &str) -> Result<()> {
    match protocol.to_lowercase().as_str() {
        "tcp" | "udp" | "all" => Ok(()),
        _ => Err(crate::Error::InvalidPort(format!(
            "Invalid protocol '{}'. Must be tcp, udp, or all",
            protocol
        ))),
    }
}

pub fn validate_sort_option(sort: &str) -> Result<()> {
    match sort.to_lowercase().as_str() {
        "port" | "pid" | "name" => Ok(()),
        _ => Err(crate::Error::Other(format!(
            "Invalid sort option '{}'. Must be port, pid, or name",
            sort
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_port() {
        assert!(validate_port(80).is_ok());
        assert!(validate_port(8080).is_ok());
        assert!(validate_port(65535).is_ok());

        let result = validate_port(0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("greater than 0"));
    }

    #[test]
    fn test_validate_protocol() {
        assert!(validate_protocol("tcp").is_ok());
        assert!(validate_protocol("TCP").is_ok());
        assert!(validate_protocol("udp").is_ok());
        assert!(validate_protocol("UDP").is_ok());
        assert!(validate_protocol("all").is_ok());
        assert!(validate_protocol("ALL").is_ok());

        let result = validate_protocol("http");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid protocol"));
    }

    #[test]
    fn test_validate_sort_option() {
        assert!(validate_sort_option("port").is_ok());
        assert!(validate_sort_option("PORT").is_ok());
        assert!(validate_sort_option("pid").is_ok());
        assert!(validate_sort_option("PID").is_ok());
        assert!(validate_sort_option("name").is_ok());
        assert!(validate_sort_option("NAME").is_ok());

        let result = validate_sort_option("date");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid sort option"));
    }
}
