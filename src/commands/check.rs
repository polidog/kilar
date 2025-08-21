use crate::{port::PortManager, Result};
use colored::Colorize;

pub struct CheckCommand;

impl CheckCommand {
    pub async fn execute(
        port: u16,
        protocol: &str,
        quiet: bool,
        json: bool,
    ) -> Result<()> {
        let port_manager = PortManager::new();
        
        match port_manager.check_port(port, protocol).await {
            Ok(Some(process_info)) => {
                if json {
                    let json_output = serde_json::json!({
                        "port": port,
                        "protocol": protocol,
                        "status": "occupied",
                        "process": {
                            "pid": process_info.pid,
                            "name": process_info.name,
                            "command": process_info.command
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else if !quiet {
                    println!("{} {}:{} は使用中です", "✓".green(), protocol.to_uppercase(), port);
                    println!("  PID: {}", process_info.pid);
                    println!("  プロセス名: {}", process_info.name);
                    println!("  パス: {}", process_info.path);
                    println!("  コマンド: {}", process_info.command);
                }
            },
            Ok(None) => {
                if json {
                    let json_output = serde_json::json!({
                        "port": port,
                        "protocol": protocol,
                        "status": "available"
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else if !quiet {
                    println!("{} {}:{} は使用されていません", "○".blue(), protocol.to_uppercase(), port);
                }
            },
            Err(e) => {
                if json {
                    let json_output = serde_json::json!({
                        "port": port,
                        "protocol": protocol,
                        "status": "error",
                        "error": e.to_string()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else {
                    eprintln!("{} {}", "エラー:".red(), e);
                }
                return Err(e);
            }
        }
        
        Ok(())
    }
}