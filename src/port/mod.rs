use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command as TokioCommand;

pub mod adaptive;
pub mod incremental;
pub mod procfs;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inode: Option<u64>, // For procfs-based implementation
}

#[derive(Debug, Clone)]
struct ProcessDetails {
    executable_path: String,
    working_directory: String,
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

    pub async fn list_processes_with_progress<F>(
        &self,
        protocol: &str,
        progress_callback: Option<F>,
    ) -> Result<Vec<ProcessInfo>>
    where
        F: Fn(&str) + Send + Sync,
    {
        if let Some(callback) = &progress_callback {
            callback("Initializing port scan...");
        }

        self.list_processes_unix_with_progress(protocol, progress_callback)
            .await
    }

    async fn list_processes_unix(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        // Try lsof first, fallback to ss, then netstat
        if let Ok(result) = self.try_lsof(protocol).await {
            return Ok(result);
        }

        if let Ok(result) = self.try_ss(protocol).await {
            return Ok(result);
        }

        self.try_netstat_unix(protocol).await
    }

    async fn list_processes_unix_with_progress<F>(
        &self,
        protocol: &str,
        callback: Option<F>,
    ) -> Result<Vec<ProcessInfo>>
    where
        F: Fn(&str) + Send + Sync,
    {
        if let Some(ref cb) = callback {
            cb("Executing port scan with lsof...");
        }

        // Try lsof first, fallback to ss, then netstat
        if let Ok(result) = self.try_lsof_with_callback(protocol, &callback).await {
            return Ok(result);
        }

        if let Some(ref cb) = callback {
            cb("Trying alternative method (ss)...");
        }

        if let Ok(result) = self.try_ss(protocol).await {
            return Ok(result);
        }

        if let Some(ref cb) = callback {
            cb("Trying fallback method (netstat)...");
        }

        self.try_netstat_unix(protocol).await
    }

    /// Get process details for multiple PIDs in a single lsof command
    async fn get_all_process_details(&self, pids: &[u32]) -> Result<HashMap<u32, ProcessDetails>> {
        if pids.is_empty() {
            return Ok(HashMap::new());
        }

        let mut details_map = HashMap::new();

        // Build PID list for lsof -p option
        let pid_list = pids
            .iter()
            .map(|pid| pid.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let output = TokioCommand::new("lsof")
            .arg("-p")
            .arg(&pid_list)
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse lsof output to extract executable paths and working directories
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 9 {
                        if let Ok(pid) = parts[1].parse::<u32>() {
                            if pids.contains(&pid) {
                                let entry =
                                    details_map.entry(pid).or_insert_with(|| ProcessDetails {
                                        executable_path: "Unknown".to_string(),
                                        working_directory: "Unknown".to_string(),
                                    });

                                // Check for executable (txt REG)
                                if parts[3] == "txt" && parts[4] == "REG" {
                                    let path = parts[8..].join(" ");

                                    // Filter out system libraries, prefer application executables
                                    if !path.contains("/usr/lib")
                                        && !path.contains("/System/Library")
                                        && !path.contains("/usr/share")
                                        && !path.contains("/Library/Preferences/Logging")
                                        && !path.contains("/private/var/db")
                                        && !path.ends_with("/dyld")
                                    {
                                        entry.executable_path = path;
                                    }
                                }

                                // Check for working directory (cwd DIR)
                                if parts[3] == "cwd" && parts[4] == "DIR" {
                                    let path = parts[8..].join(" ");
                                    entry.working_directory = path;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fill in defaults for PIDs that weren't found
        for &pid in pids {
            details_map.entry(pid).or_insert_with(|| ProcessDetails {
                executable_path: "Unknown".to_string(),
                working_directory: "Unknown".to_string(),
            });
        }

        Ok(details_map)
    }

    async fn try_lsof(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        self.try_lsof_with_callback(protocol, &None::<fn(&str)>)
            .await
    }

    async fn try_lsof_with_callback<F>(
        &self,
        protocol: &str,
        callback: &Option<F>,
    ) -> Result<Vec<ProcessInfo>>
    where
        F: Fn(&str) + Send + Sync,
    {
        let mut cmd = TokioCommand::new("lsof");
        cmd.arg("-n") // 数値表示（ホスト名解決なし）
            .arg("-P"); // ポート番号を数値表示

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-iTCP").arg("-sTCP:LISTEN"); // リスニング状態のみ（TCP）
            }
            "udp" => {
                cmd.arg("-iUDP");
            }
            "all" => {
                cmd.arg("-i");
            }
            _ => {
                cmd.arg("-iTCP").arg("-sTCP:LISTEN"); // デフォルトはTCP
            }
        }

        let output = cmd.output().await.map_err(|e| {
            crate::Error::CommandFailed(format!(
                "lsof command failed: {e}. Make sure required system tools are installed"
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!(
                "lsof failed: {stderr}. Make sure required system tools are installed"
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Report parsing stage
        if let Some(ref cb) = callback {
            cb("Parsing port information...");
        }

        if callback.is_some() {
            self.parse_lsof_output_with_callback(&stdout, protocol, callback)
                .await
        } else {
            self.parse_lsof_output(&stdout, protocol).await
        }
    }

    async fn try_ss(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut cmd = TokioCommand::new("ss");
        cmd.arg("-n") // 数値表示
            .arg("-p"); // プロセス情報表示

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-lt"); // TCP listening
            }
            "udp" => {
                cmd.arg("-lu"); // UDP
            }
            "all" => {
                cmd.arg("-ltu"); // TCP and UDP
            }
            _ => {
                cmd.arg("-lt"); // デフォルトはTCP
            }
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("ss command failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!("ss failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_ss_output(&stdout, protocol).await
    }

    async fn try_netstat_unix(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut cmd = TokioCommand::new("netstat");
        cmd.arg("-n") // 数値表示
            .arg("-p"); // プロセス情報表示

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-lt"); // TCP listening
            }
            "udp" => {
                cmd.arg("-lu"); // UDP
            }
            "all" => {
                cmd.arg("-ltu"); // TCP and UDP
            }
            _ => {
                cmd.arg("-lt"); // デフォルトはTCP
            }
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("netstat command failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!(
                "netstat failed: {stderr}"
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_netstat_unix_output(&stdout, protocol).await
    }

    async fn parse_lsof_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        let mut basic_process_info = Vec::new();

        // First pass: collect basic process info and PIDs
        for line in output.lines().skip(1) {
            // ヘッダー行をスキップ
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 9 {
                continue;
            }

            let command = fields[0];
            let pid_str = fields[1];
            let type_field = fields[4];
            let protocol_field = if fields.len() > 7 { fields[7] } else { "" };
            let node = fields[8];

            // TCPまたはUDPポートのみ処理
            if !type_field.contains("IPv4") && !type_field.contains("IPv6") {
                continue;
            }

            let pid = match pid_str.parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            // ポート番号を抽出
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

            // プロトコルを複数の列から判定
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
                // lsofのデフォルト動作から推測：リスニングポートは通常TCP
                "tcp"
            }
            .to_string();

            basic_process_info.push((pid, command, port, protocol, address));
        }

        // Extract unique PIDs for batch processing
        let pids: Vec<u32> = basic_process_info
            .iter()
            .map(|(pid, _, _, _, _)| *pid)
            .collect();

        // Get all process details in a single lsof call
        let process_details = self.get_all_process_details(&pids).await?;

        // Second pass: build ProcessInfo with detailed information
        for (pid, command, port, protocol, address) in basic_process_info {
            // Use command from lsof as fallback instead of calling ps individually
            let full_command = command.to_string();

            let name = self.extract_process_name(&full_command);

            // Get details from batch result
            let (executable_path, working_directory) =
                if let Some(details) = process_details.get(&pid) {
                    let executable_path = if details.executable_path != "Unknown" {
                        details.executable_path.clone()
                    } else {
                        // Fallback to extracting from command line
                        self.extract_executable_path(&full_command)
                    };

                    (executable_path, details.working_directory.clone())
                } else {
                    // Fallback if batch processing failed
                    (
                        self.extract_executable_path(&full_command),
                        "Unknown".to_string(),
                    )
                };

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                executable_path,
                working_directory,
                port,
                protocol,
                address,
                inode: None, // Legacy implementation doesn't track inodes
            });
        }

        Ok(processes)
    }

    async fn parse_lsof_output_with_callback<F>(
        &self,
        output: &str,
        _protocol: &str,
        callback: &Option<F>,
    ) -> Result<Vec<ProcessInfo>>
    where
        F: Fn(&str),
    {
        if let Some(ref cb) = callback {
            cb("Parsing lsof output...");
        }
        let mut processes = Vec::new();
        let mut basic_process_info = Vec::new();

        // First pass: collect basic process info and PIDs
        if let Some(ref cb) = callback {
            cb("Extracting port information...");
        }
        for line in output.lines().skip(1) {
            // ヘッダー行をスキップ
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 9 {
                continue;
            }

            let command = fields[0];
            let pid_str = fields[1];
            let type_field = fields[4];
            let protocol_field = if fields.len() > 7 { fields[7] } else { "" };
            let node = fields[8];

            // TCPまたはUDPポートのみ処理
            if !type_field.contains("IPv4") && !type_field.contains("IPv6") {
                continue;
            }

            let pid = match pid_str.parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
            };

            // ポート番号を抽出
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

            // プロトコルを複数の列から判定
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
                // lsofのデフォルト動作から推測：リスニングポートは通常TCP
                "tcp"
            }
            .to_string();

            basic_process_info.push((pid, command, port, protocol, address));
        }

        // Extract unique PIDs for batch processing
        let pids: Vec<u32> = basic_process_info
            .iter()
            .map(|(pid, _, _, _, _)| *pid)
            .collect();

        // Get all process details in a single lsof call
        if let Some(ref cb) = callback {
            cb("Collecting detailed process information...");
        }
        let process_details = self.get_all_process_details(&pids).await?;

        // Second pass: build ProcessInfo with detailed information
        if let Some(ref cb) = callback {
            cb("Building process list...");
        }
        for (pid, command, port, protocol, address) in basic_process_info {
            // Use command from lsof as fallback instead of calling ps individually
            let full_command = command.to_string();

            let name = self.extract_process_name(&full_command);

            // Get details from batch result
            let (executable_path, working_directory) =
                if let Some(details) = process_details.get(&pid) {
                    let executable_path = if details.executable_path != "Unknown" {
                        details.executable_path.clone()
                    } else {
                        // Fallback to extracting from command line
                        self.extract_executable_path(&full_command)
                    };

                    (executable_path, details.working_directory.clone())
                } else {
                    // Fallback if batch processing failed
                    (
                        self.extract_executable_path(&full_command),
                        "Unknown".to_string(),
                    )
                };

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                executable_path,
                working_directory,
                port,
                protocol,
                address,
                inode: None, // Legacy implementation doesn't track inodes
            });
        }

        Ok(processes)
    }

    async fn parse_ss_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for line in output.lines().skip(1) {
            // ヘッダー行をスキップ
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

            // ポート番号を抽出
            let port = if let Some(colon_pos) = local_address.rfind(':') {
                match local_address[colon_pos + 1..].parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            // プロセス情報からPIDを抽出 (users:(("process",pid=1234,fd=5)))
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

            let full_command = match self.get_process_command(pid).await {
                Ok(cmd) => cmd,
                Err(_) => "Unknown".to_string(),
            };

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
                inode: None, // Legacy implementation doesn't track inodes
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

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }

            let protocol = parts[0].to_lowercase();
            let local_address = parts[3];
            let state = parts[5];
            let process_info = parts[6];

            // リスニング状態のみ処理
            if !state.contains("LISTEN") {
                continue;
            }

            // ポート番号を抽出
            let port = if let Some(colon_pos) = local_address.rfind(':') {
                match local_address[colon_pos + 1..].parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => continue,
                }
            } else {
                continue;
            };

            // プロセス情報からPIDを抽出 (1234/process_name)
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

            let full_command = match self.get_process_command(pid).await {
                Ok(cmd) => cmd,
                Err(_) => process_info.to_string(),
            };

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
                inode: None, // Legacy implementation doesn't track inodes
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
            // パスから実行ファイル名だけを抽出
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
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}
