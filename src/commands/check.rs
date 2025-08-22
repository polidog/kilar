use crate::{port::PortManager, process::ProcessManager, Result};
use colored::Colorize;
use dialoguer::Confirm;

/// Command for checking port usage status.
///
/// This command allows you to check if a specific port is in use
/// and provides information about the process using it.
///
/// # Example
///
/// ```no_run
/// use kilar::commands::CheckCommand;
///
/// #[tokio::main]
/// async fn main() {
///     // Check if port 3000 is in use (TCP)
///     CheckCommand::execute(3000, "tcp", false, false, false, false).await.unwrap();
/// }
/// ```
pub struct CheckCommand;

impl CheckCommand {
    /// Execute the check command for a specific port.
    ///
    /// # Arguments
    ///
    /// * `port` - The port number to check
    /// * `protocol` - The protocol to check ("tcp" or "udp")
    /// * `quiet` - Suppress output if true
    /// * `json` - Output in JSON format if true
    /// * `verbose` - Show verbose information if true
    /// * `interactive` - Enable interactive mode with kill option if true
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the command executes successfully, or an error if something goes wrong.
    pub async fn execute(
        port: u16,
        protocol: &str,
        quiet: bool,
        json: bool,
        verbose: bool,
        interactive: bool,
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
                            "executable_path": process_info.executable_path,
                            "working_directory": process_info.working_directory,
                            "command": process_info.command
                        }
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else if !quiet {
                    println!(
                        "{} {}:{} is in use",
                        "✓".green(),
                        protocol.to_uppercase().blue(),
                        port.to_string().yellow()
                    );
                    println!("  {} {}", "PID:".cyan(), process_info.pid);
                    println!("  {} {}", "Process:".cyan(), process_info.name);

                    // Use smart path display logic
                    let port_manager = PortManager::new();
                    let display_path = port_manager.get_display_path(&process_info);
                    println!("  {} {}", "Path:".cyan(), display_path);
                    if verbose {
                        println!("  {} {}", "Command:".cyan(), process_info.command);
                    }

                    // Interactive kill option
                    if interactive && !json {
                        println!();
                        let prompt = format!(
                            "Kill process {} (PID: {})?",
                            process_info.name.yellow(),
                            process_info.pid.to_string().cyan()
                        );

                        let confirmed = Confirm::new()
                            .with_prompt(prompt)
                            .default(false)
                            .interact()?;

                        if confirmed {
                            let process_manager = ProcessManager::new();
                            match process_manager.kill_process(process_info.pid).await {
                                Ok(()) => {
                                    println!(
                                        "{} Killed process {} (PID: {})",
                                        "✓".green(),
                                        process_info.name.yellow(),
                                        process_info.pid.to_string().cyan()
                                    );
                                }
                                Err(e) => {
                                    eprintln!("{} Failed to kill process: {}", "×".red(), e);
                                    return Err(e);
                                }
                            }
                        } else {
                            println!("{} Operation cancelled", "×".yellow());
                        }
                    }
                }
            }
            Ok(None) => {
                if json {
                    let json_output = serde_json::json!({
                        "port": port,
                        "protocol": protocol,
                        "status": "available"
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output)?);
                } else if !quiet {
                    println!(
                        "{} {}:{} is available",
                        "○".blue(),
                        protocol.to_uppercase().blue(),
                        port.to_string().yellow()
                    );
                }
            }
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
                    eprintln!("{} {}", "Error:".red(), e);
                }
                return Err(e);
            }
        }

        Ok(())
    }
}
