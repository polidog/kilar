use crate::{port::PortManager, process::ProcessManager, Result};
use colored::Colorize;
use dialoguer::Confirm;

pub struct KillCommand;

impl KillCommand {
    pub async fn execute(
        port: u16,
        protocol: &str,
        force: bool,
        quiet: bool,
        json: bool,
    ) -> Result<()> {
        let port_manager = PortManager::new();
        let process_manager = ProcessManager::new();
        
        match port_manager.check_port(port, protocol).await? {
            Some(process_info) => {
                if !force && !json {
                    let prompt = format!(
                        "{}:{} を使用しているプロセス {} (PID: {}) を終了しますか?",
                        protocol.to_uppercase(),
                        port,
                        process_info.name,
                        process_info.pid
                    );
                    
                    let confirmed = Confirm::new()
                        .with_prompt(prompt)
                        .default(false)
                        .interact()?;
                    
                    if !confirmed {
                        if !quiet {
                            println!("{} 操作がキャンセルされました", "×".yellow());
                        }
                        return Ok(());
                    }
                }
                
                match process_manager.kill_process(process_info.pid).await {
                    Ok(()) => {
                        if json {
                            let json_output = serde_json::json!({
                                "port": port,
                                "protocol": protocol,
                                "action": "killed",
                                "process": {
                                    "pid": process_info.pid,
                                    "name": process_info.name
                                }
                            });
                            println!("{}", serde_json::to_string_pretty(&json_output)?);
                        } else if !quiet {
                            println!("{} プロセス {} (PID: {}) を終了しました", 
                                "✓".green(), process_info.name, process_info.pid);
                        }
                    },
                    Err(e) => {
                        if json {
                            let json_output = serde_json::json!({
                                "port": port,
                                "protocol": protocol,
                                "action": "failed",
                                "error": e.to_string(),
                                "process": {
                                    "pid": process_info.pid,
                                    "name": process_info.name
                                }
                            });
                            println!("{}", serde_json::to_string_pretty(&json_output)?);
                        } else {
                            eprintln!("{} プロセスの終了に失敗しました: {}", "×".red(), e);
                        }
                        return Err(e);
                    }
                }
            },
            None => {
                let error_msg = format!("ポート {}:{} は使用されていません", protocol.to_uppercase(), port);
                if json {
                    let json_output = serde_json::json!({
                        "port": port,
                        "protocol": protocol,
                        "action": "not_found",
                        "error": error_msg
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else if !quiet {
                    eprintln!("{} {}", "×".red(), error_msg);
                }
                return Err(crate::Error::PortNotFound(port));
            }
        }
        
        Ok(())
    }
}