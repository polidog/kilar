use crate::Result;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub executable_path: String,
    pub working_directory: String,
    pub port: u16,
    pub protocol: String,
    pub address: String,
}

pub struct PortManager;

impl PortManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn check_port(&self, port: u16, protocol: &str) -> Result<Option<ProcessInfo>> {
        let processes = self.list_processes(protocol).await?;
        Ok(processes.into_iter().find(|p| p.port == port))
    }

    pub async fn list_processes(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        self.list_processes_unix(protocol).await
    }

    async fn list_processes_unix(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        // Try lsof first with optimizations
        if let Ok(result) = self.try_lsof_optimized(protocol).await {
            return Ok(result);
        }

        // Fallback to ss
        if let Ok(result) = self.try_ss(protocol).await {
            return Ok(result);
        }

        // Final fallback to netstat
        self.try_netstat_unix(protocol).await
    }

    async fn try_lsof_optimized(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut cmd = TokioCommand::new("lsof");

        // 最適化されたフラグ
        cmd.arg("-n") // ホスト名解決をスキップ
            .arg("-P") // ポート名解決をスキップ
            .arg("-w"); // 警告を抑制（高速化）

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-iTCP").arg("-sTCP:LISTEN");
            }
            "udp" => {
                cmd.arg("-iUDP");
            }
            "all" => {
                cmd.arg("-i");
            }
            _ => {
                cmd.arg("-iTCP").arg("-sTCP:LISTEN");
            }
        }

        // タイムアウト設定で長時間待機を防ぐ
        let output = tokio::time::timeout(std::time::Duration::from_secs(5), cmd.output())
            .await
            .map_err(|_| crate::Error::CommandFailed("lsof command timed out".to_string()))?
            .map_err(|e| {
                crate::Error::CommandFailed(format!(
                    "lsof command failed: {}. Make sure required system tools are installed",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!(
                "lsof failed: {}. Make sure required system tools are installed",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // バッチでプロセス情報を取得
        self.parse_lsof_output_batch(&stdout, protocol).await
    }

    async fn try_ss(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut cmd = TokioCommand::new("ss");
        cmd.arg("-n").arg("-p");

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-lt");
            }
            "udp" => {
                cmd.arg("-lu");
            }
            "all" => {
                cmd.arg("-ltu");
            }
            _ => {
                cmd.arg("-lt");
            }
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("ss command failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!(
                "ss failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_ss_output(&stdout, protocol).await
    }

    async fn try_netstat_unix(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut cmd = TokioCommand::new("netstat");
        cmd.arg("-n").arg("-p");

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-lt");
            }
            "udp" => {
                cmd.arg("-lu");
            }
            "all" => {
                cmd.arg("-ltu");
            }
            _ => {
                cmd.arg("-lt");
            }
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("netstat command failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!(
                "netstat failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_netstat_unix_output(&stdout, protocol).await
    }

    async fn parse_lsof_output_batch(
        &self,
        output: &str,
        _protocol: &str,
    ) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        let mut pids = Vec::new();

        // First pass: collect basic info and PIDs
        for line in output.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 9 {
                continue;
            }

            let command = fields[0];
            let pid_str = fields[1];
            let type_field = fields[4];
            let protocol_field = if fields.len() > 7 { fields[7] } else { "" };
            let node = fields[8];

            if !type_field.contains("IPv4") && !type_field.contains("IPv6") {
                continue;
            }

            let pid = match pid_str.parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            let port = if let Some(colon_pos) = node.rfind(':') {
                match node[colon_pos + 1..].parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            let address = if let Some(colon_pos) = node.rfind(':') {
                node[..colon_pos].to_string()
            } else {
                "*".to_string()
            };

            let protocol = if protocol_field.contains("TCP") || protocol_field.contains("tcp") {
                "tcp"
            } else if protocol_field.contains("UDP") || protocol_field.contains("udp") {
                "udp"
            } else if node.contains("TCP") || node.contains("tcp") {
                "tcp"
            } else if node.contains("UDP") || node.contains("udp") {
                "udp"
            } else if type_field.contains("TCP") || type_field.contains("tcp") {
                "tcp"
            } else if type_field.contains("UDP") || type_field.contains("udp") {
                "udp"
            } else {
                "tcp"
            }
            .to_string();

            pids.push(pid);
            processes.push((pid, port, protocol, address, command.to_string()));
        }

        // Batch get all process commands at once
        let commands = self.get_process_commands_batch(&pids).await?;

        // Build final process list
        let mut final_processes = Vec::new();
        for (pid, port, protocol, address, fallback_command) in processes {
            let full_command = commands
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| fallback_command.clone());

            let name = self.extract_process_name(&full_command);

            // Get the actual executable path
            let executable_path = match self.get_process_executable(pid).await {
                Ok(path) => {
                    // If we got the same as command (fallback case), extract it
                    if path == full_command {
                        self.extract_executable_path(&full_command)
                    } else {
                        path
                    }
                }
                Err(_) => self.extract_executable_path(&full_command),
            };

            // Get the working directory
            let working_directory = self
                .get_process_working_directory(pid)
                .await
                .unwrap_or_else(|_| "Unknown".to_string());

            final_processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                executable_path,
                working_directory,
                port,
                protocol,
                address,
            });
        }

        Ok(final_processes)
    }

    async fn parse_ss_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        let mut pids = Vec::new();
        let mut temp_processes = Vec::new();

        for line in output.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }

            let protocol = parts[0].to_lowercase();
            let local_address = parts[4];
            let process_info = if parts.len() > 6 {
                parts[6]
            } else {
                continue;
            };

            let port = if let Some(colon_pos) = local_address.rfind(':') {
                match local_address[colon_pos + 1..].parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            let pid = if let Some(pid_start) = process_info.find("pid=") {
                let pid_start = pid_start + 4;
                if let Some(pid_end) = process_info[pid_start..].find(',') {
                    match process_info[pid_start..pid_start + pid_end].parse::<u32>() {
                        Ok(pid) => pid,
                        Err(_) => continue,
                    }
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let address = if let Some(colon_pos) = local_address.rfind(':') {
                local_address[..colon_pos].to_string()
            } else {
                "*".to_string()
            };

            pids.push(pid);
            temp_processes.push((pid, port, protocol, address));
        }

        // Batch get all process commands
        let commands = self.get_process_commands_batch(&pids).await?;

        for (pid, port, protocol, address) in temp_processes {
            let full_command = commands
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            let name = self.extract_process_name(&full_command);

            // Get the actual executable path
            let executable_path = match self.get_process_executable(pid).await {
                Ok(path) => {
                    // If we got the same as command (fallback case), extract it
                    if path == full_command {
                        self.extract_executable_path(&full_command)
                    } else {
                        path
                    }
                }
                Err(_) => self.extract_executable_path(&full_command),
            };

            // Get the working directory
            let working_directory = self
                .get_process_working_directory(pid)
                .await
                .unwrap_or_else(|_| "Unknown".to_string());

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                executable_path,
                working_directory,
                port,
                protocol,
                address,
            });
        }

        Ok(processes)
    }

    async fn parse_netstat_unix_output(
        &self,
        output: &str,
        _protocol: &str,
    ) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        let mut pids = Vec::new();
        let mut temp_processes = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }

            let protocol = parts[0].to_lowercase();
            let local_address = parts[3];
            let state = parts[5];
            let process_info = parts[6];

            if !state.contains("LISTEN") {
                continue;
            }

            let port = if let Some(colon_pos) = local_address.rfind(':') {
                match local_address[colon_pos + 1..].parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            let pid = if let Some(slash_pos) = process_info.find('/') {
                match process_info[..slash_pos].parse::<u32>() {
                    Ok(pid) => pid,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            let address = if let Some(colon_pos) = local_address.rfind(':') {
                local_address[..colon_pos].to_string()
            } else {
                "*".to_string()
            };

            pids.push(pid);
            temp_processes.push((pid, port, protocol, address, process_info.to_string()));
        }

        // Batch get all process commands
        let commands = self.get_process_commands_batch(&pids).await?;

        for (pid, port, protocol, address, fallback_info) in temp_processes {
            let full_command = commands
                .get(&pid)
                .cloned()
                .unwrap_or_else(|| fallback_info.clone());

            let name = self.extract_process_name(&full_command);

            // Get the actual executable path
            let executable_path = match self.get_process_executable(pid).await {
                Ok(path) => {
                    // If we got the same as command (fallback case), extract it
                    if path == full_command {
                        self.extract_executable_path(&full_command)
                    } else {
                        path
                    }
                }
                Err(_) => self.extract_executable_path(&full_command),
            };

            // Get the working directory
            let working_directory = self
                .get_process_working_directory(pid)
                .await
                .unwrap_or_else(|_| "Unknown".to_string());

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                executable_path,
                working_directory,
                port,
                protocol,
                address,
            });
        }

        Ok(processes)
    }

    fn extract_process_name(&self, command_line: &str) -> String {
        if command_line.is_empty() {
            return "Unknown".to_string();
        }

        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if let Some(first_part) = parts.first() {
            let name = first_part.split('/').next_back().unwrap_or(first_part);
            name.to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn extract_executable_path(&self, command_line: &str) -> String {
        if command_line.is_empty() {
            return "Unknown".to_string();
        }

        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if let Some(first_part) = parts.first() {
            first_part.to_string()
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_display_path(&self, process_info: &ProcessInfo) -> String {
        // Prefer working directory for development processes (when it's not root)
        if process_info.working_directory != "/" && process_info.working_directory != "Unknown" {
            // Check if this is likely a development process based on the executable or command
            let is_dev_process = process_info.executable_path.contains("/node")
                || process_info.executable_path.contains("/python")
                || process_info.executable_path.contains("/ruby")
                || process_info.executable_path.contains("/java")
                || process_info.command.contains("npm")
                || process_info.command.contains("yarn")
                || process_info.command.contains("pnpm")
                || process_info.command.contains("next")
                || process_info.command.contains("serve")
                || process_info.command.contains("dev");

            if is_dev_process {
                return process_info.working_directory.clone();
            }
        }

        // Fallback to executable path for system processes
        process_info.executable_path.clone()
    }

    async fn get_process_command(&self, pid: u32) -> Result<String> {
        let output = TokioCommand::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .arg("-o")
            .arg("command=")
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(crate::Error::ProcessNotFound(pid))
        }
    }

    async fn get_process_executable(&self, pid: u32) -> Result<String> {
        // Try to get the actual executable path using lsof
        let output = TokioCommand::new("lsof")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Look for the main executable (txt REG entry)
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 9 {
                        // Check if this is a regular file (REG) and text segment (txt)
                        if parts[3] == "txt" && parts[4] == "REG" {
                            // Extract the full path from field 8 onwards (0-indexed)
                            // The path starts from the 9th field (index 8) and may contain spaces
                            let path = parts[8..].join(" ");

                            // Filter out system libraries and dyld, prefer application executables
                            if !path.contains("/usr/lib")
                                && !path.contains("/System/Library")
                                && !path.contains("/usr/share")
                                && !path.contains("/Library/Preferences/Logging")
                                && !path.contains("/private/var/db")
                                && !path.ends_with("/dyld")
                            {
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }

        // Fallback to ps command if lsof fails
        self.get_process_command(pid).await
    }

    async fn get_process_working_directory(&self, pid: u32) -> Result<String> {
        // Try to get the working directory using lsof
        let output = TokioCommand::new("lsof")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Look for the current working directory (cwd DIR entry)
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 9 {
                        // Check if this is a directory (DIR) and current working directory (cwd)
                        if parts[3] == "cwd" && parts[4] == "DIR" {
                            // Extract the full path from field 8 onwards (0-indexed)
                            // The path starts from the 9th field (index 8) and may contain spaces
                            let path = parts[8..].join(" ");
                            return Ok(path);
                        }
                    }
                }
            }
        }

        // Fallback to "Unknown" if we can't get the working directory
        Ok("Unknown".to_string())
    }

    // バッチでプロセス情報を取得（大幅な高速化）
    async fn get_process_commands_batch(&self, pids: &[u32]) -> Result<HashMap<u32, String>> {
        if pids.is_empty() {
            return Ok(HashMap::new());
        }

        // 一度にすべてのプロセス情報を取得
        let pid_list = pids
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let output = TokioCommand::new("ps")
            .arg("-p")
            .arg(&pid_list)
            .arg("-o")
            .arg("pid=,command=")
            .output()
            .await?;

        let mut commands = HashMap::new();

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Split by first whitespace to separate PID and command
                if let Some(space_pos) = line.find(char::is_whitespace) {
                    if let Ok(pid) = line[..space_pos].trim().parse::<u32>() {
                        let command = line[space_pos..].trim().to_string();
                        commands.insert(pid, command);
                    }
                }
            }
        }

        // Fallback for missing PIDs (parallel execution)
        let missing_pids: Vec<u32> = pids
            .iter()
            .filter(|&&pid| !commands.contains_key(&pid))
            .copied()
            .collect();

        if !missing_pids.is_empty() {
            let futures = missing_pids.iter().map(|&pid| async move {
                self.get_process_command(pid)
                    .await
                    .ok()
                    .map(|cmd| (pid, cmd))
            });

            let results = join_all(futures).await;
            for result in results.into_iter().flatten() {
                commands.insert(result.0, result.1);
            }
        }

        Ok(commands)
    }
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}