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
        verbose: bool,
    ) -> Result<()> {
        let port_manager = PortManager::new();
        let process_manager = ProcessManager::new();
        
        match port_manager.check_port(port, protocol).await? {
            Some(process_info) => {
                if !force && !json {
                    let prompt = format!(
                        "Kill process {} (PID: {}) using {}:{}?",
                        process_info.name.yellow(),
                        process_info.pid.to_string().cyan(),
                        protocol.to_uppercase().blue(),
                        port.to_string().yellow()
                    );
                    
                    let confirmed = Confirm::new()
                        .with_prompt(prompt)
                        .default(false)
                        .interact()?;
                    
                    if !confirmed {
                        if !quiet {
                            println!("{} Operation cancelled", "×".yellow());
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
                            println!("{} Killed process {} (PID: {})", 
                                "✓".green(), process_info.name.yellow(), process_info.pid.to_string().cyan());
                            if verbose {
                                println!("  Process was using port {}", port.to_string().yellow());
                                println!("  Protocol: {}", protocol.to_uppercase().blue());
                            }
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
                            eprintln!("{} Failed to kill process: {}", "×".red(), e);
                        }
                        return Err(e);
                    }
                }
            },
            None => {
                let error_msg = format!("Port {}:{} is not in use", protocol.to_uppercase(), port);
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