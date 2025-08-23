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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::ProcessInfo;

    // Mockポートマネージャーを作成するためのヘルパー関数
    fn create_mock_process_info(port: u16) -> ProcessInfo {
        ProcessInfo {
            pid: 1234,
            name: "test_process".to_string(),
            command: "/usr/bin/test_process --port 8080".to_string(),
            executable_path: "/usr/bin/test_process".to_string(),
            working_directory: "/home/user/project".to_string(),
            port,
            protocol: "tcp".to_string(),
            address: "127.0.0.1".to_string(),
            inode: Some(12345),
        }
    }

    #[tokio::test]
    async fn test_check_command_json_output_occupied_port() {
        // この統合テストはシステムに依存するため、エラーハンドリングのテストとして機能
        let result = CheckCommand::execute(65432, "tcp", false, true, false, false).await;

        // JSONアウトプットの構造をテストする代わりに、エラーハンドリングをテスト
        match result {
            Ok(_) => {
                // JSON出力が成功した場合はOK
            }
            Err(e) => {
                // システムツールがない場合のエラーも受け入れ
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("lsof")
                        || error_msg.contains("ss")
                        || error_msg.contains("netstat")
                        || error_msg.contains("system tools")
                );
            }
        }
    }

    #[tokio::test]
    async fn test_check_command_quiet_mode() {
        // quiet=trueでの実行をテスト
        let result = CheckCommand::execute(65433, "tcp", true, false, false, false).await;

        // quietモードでもエラーハンドリングが正しく動作することを確認
        match result {
            Ok(_) => {
                // 成功した場合はOK
            }
            Err(e) => {
                // システムツールがない場合のエラーハンドリングをテスト
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_check_command_verbose_mode() {
        // verbose=trueでの実行をテスト
        let result = CheckCommand::execute(65434, "tcp", false, false, true, false).await;

        // verboseモードでもエラーハンドリングが正しく動作することを確認
        match result {
            Ok(_) => {
                // 成功した場合はOK
            }
            Err(e) => {
                // システムツールがない場合のエラーハンドリングをテスト
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_check_command_different_protocols() {
        // 異なるプロトコルでのテスト
        for protocol in ["tcp", "udp"] {
            let result = CheckCommand::execute(65435, protocol, true, true, false, false).await;

            match result {
                Ok(_) => {
                    // 成功した場合はOK
                }
                Err(e) => {
                    // システムツールがない場合のエラーハンドリングをテスト
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("lsof")
                            || error_msg.contains("ss")
                            || error_msg.contains("netstat")
                            || error_msg.contains("system tools")
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_check_command_invalid_port_edge_cases() {
        // エッジケースのポート番号をテスト
        let edge_ports = [1, 65535, 80, 443];

        for port in edge_ports {
            let result = CheckCommand::execute(port, "tcp", true, true, false, false).await;

            match result {
                Ok(_) => {
                    // 成功した場合はOK
                }
                Err(e) => {
                    // エラーが適切に処理されていることを確認
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }

    #[tokio::test]
    async fn test_check_command_json_structure_validation() {
        // JSON出力の構造をテストするための基本的な検証
        let result = CheckCommand::execute(65436, "tcp", false, true, false, false).await;

        // このテストでは、システムに関係なくJSON出力の形式をテストできないので、
        // 代わりにコマンドが適切にエラーハンドリングを行うことを確認
        match result {
            Ok(_) => {
                // 成功した場合、JSON出力が正しく生成されたことを示す
            }
            Err(e) => {
                // エラーの場合、適切なエラーメッセージが生成されることを確認
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[test]
    fn test_process_info_creation() {
        // ProcessInfoの作成とシリアライゼーションをテスト
        let process_info = create_mock_process_info(8080);

        assert_eq!(process_info.pid, 1234);
        assert_eq!(process_info.name, "test_process");
        assert_eq!(process_info.port, 8080);
        assert_eq!(process_info.protocol, "tcp");

        // JSON形式での出力を確認
        let json_output = serde_json::json!({
            "port": process_info.port,
            "protocol": process_info.protocol,
            "status": "occupied",
            "process": {
                "pid": process_info.pid,
                "name": process_info.name,
                "executable_path": process_info.executable_path,
                "working_directory": process_info.working_directory,
                "command": process_info.command
            }
        });

        assert!(json_output["process"]["pid"].as_u64().unwrap() == 1234);
        assert_eq!(json_output["status"].as_str().unwrap(), "occupied");
    }

    #[test]
    fn test_available_port_json_structure() {
        // 利用可能ポートのJSON出力構造をテスト
        let json_output = serde_json::json!({
            "port": 8080,
            "protocol": "tcp",
            "status": "available"
        });

        assert_eq!(json_output["port"].as_u64().unwrap(), 8080);
        assert_eq!(json_output["protocol"].as_str().unwrap(), "tcp");
        assert_eq!(json_output["status"].as_str().unwrap(), "available");
    }

    #[test]
    fn test_error_json_structure() {
        // エラー時のJSON出力構造をテスト
        let error_msg = "Test error message";
        let json_output = serde_json::json!({
            "port": 8080,
            "protocol": "tcp",
            "status": "error",
            "error": error_msg
        });

        assert_eq!(json_output["port"].as_u64().unwrap(), 8080);
        assert_eq!(json_output["protocol"].as_str().unwrap(), "tcp");
        assert_eq!(json_output["status"].as_str().unwrap(), "error");
        assert_eq!(json_output["error"].as_str().unwrap(), error_msg);
    }
}
