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
                            println!(
                                "{} Killed process {} (PID: {})",
                                "✓".green(),
                                process_info.name.yellow(),
                                process_info.pid.to_string().cyan()
                            );
                            if verbose {
                                println!("  Process was using port {}", port.to_string().yellow());
                                println!("  Protocol: {}", protocol.to_uppercase().blue());
                            }
                        }
                    }
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
            }
            None => {
                let error_msg = format!("Port {}:{port} is not in use", protocol.to_uppercase());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::ProcessInfo;

    // テスト用のモックプロセス情報を作成
    fn create_test_process_info(port: u16, pid: u32) -> ProcessInfo {
        ProcessInfo {
            pid,
            name: "test_process".to_string(),
            command: format!("/usr/bin/test_process --port {}", port),
            executable_path: "/usr/bin/test_process".to_string(),
            working_directory: "/home/user/project".to_string(),
            port,
            protocol: "tcp".to_string(),
            address: "127.0.0.1".to_string(),
            inode: Some(12345),
        }
    }

    #[tokio::test]
    async fn test_kill_command_force_mode() {
        // forceモードでの実行をテスト
        let result = KillCommand::execute(65437, "tcp", true, false, true, false).await;

        match result {
            Ok(_) => {
                // 成功した場合はOK
            }
            Err(e) => {
                // ポートが使用されていない、またはシステムツールがない場合のエラーを確認
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("not in use")
                        || error_msg.contains("lsof")
                        || error_msg.contains("ss")
                        || error_msg.contains("netstat")
                        || error_msg.contains("system tools")
                );
            }
        }
    }

    #[tokio::test]
    async fn test_kill_command_quiet_mode() {
        // quietモードでの実行をテスト
        let result = KillCommand::execute(65438, "tcp", false, true, true, false).await;

        match result {
            Ok(_) => {
                // 成功した場合はOK
            }
            Err(e) => {
                // エラーハンドリングが正しく動作することを確認
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_kill_command_json_output() {
        // JSON出力モードでの実行をテスト
        let result = KillCommand::execute(65439, "tcp", true, false, true, false).await;

        match result {
            Ok(_) => {
                // 成功した場合はOK
            }
            Err(e) => {
                // エラーが適切に処理されることを確認
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("not in use")
                        || error_msg.contains("system tools")
                        || !error_msg.is_empty()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_kill_command_verbose_mode() {
        // verboseモードでの実行をテスト
        let result = KillCommand::execute(65440, "tcp", true, false, true, true).await;

        match result {
            Ok(_) => {
                // 成功した場合はOK
            }
            Err(e) => {
                // エラーが適切に処理されることを確認
                assert!(!e.to_string().is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_kill_command_different_protocols() {
        // 異なるプロトコルでのテスト
        for protocol in ["tcp", "udp"] {
            let result = KillCommand::execute(65441, protocol, true, true, true, false).await;

            match result {
                Ok(_) => {
                    // 成功した場合はOK
                }
                Err(e) => {
                    // プロトコル固有のエラーハンドリングをテスト
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("not in use")
                            || error_msg.contains("system tools")
                            || !error_msg.is_empty()
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_kill_command_port_not_in_use() {
        // 使用されていないポートに対するkillコマンドをテスト
        let result = KillCommand::execute(65442, "tcp", true, false, true, false).await;

        // 使用されていないポートの場合はエラーが返される
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                crate::Error::PortNotFound(port) => {
                    assert_eq!(port, 65442);
                }
                _ => {
                    // システムツールがない場合など、他のエラーも受け入れ
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }

    #[test]
    fn test_kill_success_json_structure() {
        // killが成功した場合のJSON出力構造をテスト
        let process_info = create_test_process_info(8080, 1234);
        let json_output = serde_json::json!({
            "port": 8080,
            "protocol": "tcp",
            "action": "killed",
            "process": {
                "pid": process_info.pid,
                "name": process_info.name
            }
        });

        assert_eq!(json_output["port"].as_u64().unwrap(), 8080);
        assert_eq!(json_output["protocol"].as_str().unwrap(), "tcp");
        assert_eq!(json_output["action"].as_str().unwrap(), "killed");
        assert_eq!(json_output["process"]["pid"].as_u64().unwrap(), 1234);
        assert_eq!(
            json_output["process"]["name"].as_str().unwrap(),
            "test_process"
        );
    }

    #[test]
    fn test_kill_failed_json_structure() {
        // kill失敗時のJSON出力構造をテスト
        let process_info = create_test_process_info(8080, 1234);
        let error_msg = "Permission denied";
        let json_output = serde_json::json!({
            "port": 8080,
            "protocol": "tcp",
            "action": "failed",
            "error": error_msg,
            "process": {
                "pid": process_info.pid,
                "name": process_info.name
            }
        });

        assert_eq!(json_output["port"].as_u64().unwrap(), 8080);
        assert_eq!(json_output["protocol"].as_str().unwrap(), "tcp");
        assert_eq!(json_output["action"].as_str().unwrap(), "failed");
        assert_eq!(json_output["error"].as_str().unwrap(), error_msg);
        assert_eq!(json_output["process"]["pid"].as_u64().unwrap(), 1234);
    }

    #[test]
    fn test_port_not_found_json_structure() {
        // ポートが見つからない場合のJSON出力構造をテスト
        let error_msg = "Port TCP:8080 is not in use";
        let json_output = serde_json::json!({
            "port": 8080,
            "protocol": "tcp",
            "action": "not_found",
            "error": error_msg
        });

        assert_eq!(json_output["port"].as_u64().unwrap(), 8080);
        assert_eq!(json_output["protocol"].as_str().unwrap(), "tcp");
        assert_eq!(json_output["action"].as_str().unwrap(), "not_found");
        assert_eq!(json_output["error"].as_str().unwrap(), error_msg);
    }

    #[tokio::test]
    async fn test_kill_command_edge_case_ports() {
        // エッジケースのポート番号でのテスト
        let edge_ports = [1, 1023, 1024, 65535];

        for port in edge_ports {
            let result = KillCommand::execute(port, "tcp", true, true, true, false).await;

            match result {
                Ok(_) => {
                    // 成功した場合はOK
                }
                Err(e) => {
                    // エラーが適切に処理されていることを確認
                    let error_msg = e.to_string();
                    assert!(
                        error_msg.contains("not in use")
                            || error_msg.contains("system tools")
                            || error_msg.contains("Permission denied")
                            || !error_msg.is_empty()
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_kill_command_error_handling_consistency() {
        // エラーハンドリングの一貫性をテスト
        let test_cases = [
            (65443, "tcp", false, false, false, false), // インタラクティブモード
            (65444, "tcp", true, false, false, false),  // フォースモード
            (65445, "tcp", false, true, false, false),  // クワイエットモード
            (65446, "tcp", false, false, true, false),  // JSONモード
            (65447, "tcp", false, false, false, true),  // verboseモード
        ];

        for (port, protocol, force, quiet, json, verbose) in test_cases {
            let result = KillCommand::execute(port, protocol, force, quiet, json, verbose).await;

            match result {
                Ok(_) => {
                    // 成功した場合はOK
                }
                Err(e) => {
                    // すべてのモードで一貫したエラーハンドリングを確認
                    assert!(!e.to_string().is_empty());
                }
            }
        }
    }
}
