use crate::Result;

pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(crate::Error::InvalidPort("ポート番号は0以上である必要があります".to_string()));
    }
    Ok(())
}

pub fn validate_protocol(protocol: &str) -> Result<()> {
    match protocol.to_lowercase().as_str() {
        "tcp" | "udp" | "all" => Ok(()),
        _ => Err(crate::Error::InvalidPort(
            "プロトコルはtcp、udp、またはallである必要があります".to_string()
        ))
    }
}

pub fn validate_sort_option(sort: &str) -> Result<()> {
    match sort.to_lowercase().as_str() {
        "port" | "pid" | "name" => Ok(()),
        _ => Err(crate::Error::Other(
            "ソートオプションはport、pid、またはnameである必要があります".to_string()
        ))
    }
}