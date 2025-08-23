use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::process::Command as TokioCommand;

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
        // Use optimized check for better performance
        self.check_port_optimized(port, protocol).await
    }

    /// Optimized port check that only searches for specific port instead of listing all processes
    pub async fn check_port_optimized(
        &self,
        port: u16,
        protocol: &str,
    ) -> Result<Option<ProcessInfo>> {
        self.check_port_unix_optimized(port, protocol).await
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

    /// Optimized Unix port check for a specific port
    async fn check_port_unix_optimized(
        &self,
        port: u16,
        protocol: &str,
    ) -> Result<Option<ProcessInfo>> {
        // Try lsof for specific port first - much faster than scanning all ports
        if let Ok(result) = self.try_lsof_specific_port(port, protocol).await {
            return Ok(result);
        }

        // Fallback to ss for specific port
        if let Ok(result) = self.try_ss_specific_port(port, protocol).await {
            return Ok(result);
        }

        // Final fallback: netstat for specific port
        self.try_netstat_specific_port(port, protocol).await
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

    /// Fast lsof check for specific port
    async fn try_lsof_specific_port(
        &self,
        port: u16,
        protocol: &str,
    ) -> Result<Option<ProcessInfo>> {
        let protocol_flag = match protocol.to_lowercase().as_str() {
            "tcp" => "-iTCP",
            "udp" => "-iUDP",
            _ => "-i",
        };

        let combined_flag = format!("{}:{}", protocol_flag, port);

        let output = TokioCommand::new("lsof")
            .args(["-n", "-P", &combined_flag])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Skip header line
        for line in lines.iter().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                if let Ok(pid) = parts[1].parse::<u32>() {
                    // Parse address:port from lsof output
                    let name_col = parts[8];
                    if name_col.contains(&format!(":{}", port)) {
                        let full_command = self
                            .get_process_command(pid)
                            .await
                            .unwrap_or_else(|_| "Unknown".to_string());
                        let name = self.extract_process_name(&full_command);
                        let executable_path = self
                            .get_process_executable(pid)
                            .await
                            .unwrap_or_else(|_| self.extract_executable_path(&full_command));
                        let working_directory = self
                            .get_process_working_directory(pid)
                            .await
                            .unwrap_or_else(|_| "Unknown".to_string());

                        return Ok(Some(ProcessInfo {
                            pid,
                            name,
                            command: full_command,
                            executable_path,
                            working_directory,
                            port,
                            protocol: protocol.to_string(),
                            address: name_col.split(':').next().unwrap_or("*").to_string(),
                            inode: None,
                        }));
                    }
                }
            }
        }

        Ok(None)
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

    /// Fast ss check for specific port
    async fn try_ss_specific_port(&self, port: u16, protocol: &str) -> Result<Option<ProcessInfo>> {
        let mut cmd = TokioCommand::new("ss");
        cmd.arg("-n") // Numeric display
            .arg("-p"); // Process info

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-lt");
                cmd.arg(format!("sport = :{}", port));
            }
            "udp" => {
                cmd.arg("-lu");
                cmd.arg(format!("sport = :{}", port));
            }
            _ => {
                cmd.arg("-lt");
                cmd.arg(format!("sport = :{}", port));
            }
        }

        let output = cmd.output().await?;
        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }

            // Extract PID from process info (usually in format "users:(("command",pid=1234,fd=1))")
            if let Some(process_part) = parts.iter().find(|&p| p.contains("pid=")) {
                if let Some(pid_start) = process_part.find("pid=") {
                    if let Some(pid_end) = process_part[pid_start + 4..].find(',') {
                        if let Ok(pid) =
                            process_part[pid_start + 4..pid_start + 4 + pid_end].parse::<u32>()
                        {
                            let full_command = self
                                .get_process_command(pid)
                                .await
                                .unwrap_or_else(|_| "Unknown".to_string());
                            let name = self.extract_process_name(&full_command);
                            let executable_path = self
                                .get_process_executable(pid)
                                .await
                                .unwrap_or_else(|_| self.extract_executable_path(&full_command));
                            let working_directory = self
                                .get_process_working_directory(pid)
                                .await
                                .unwrap_or_else(|_| "Unknown".to_string());
                            {
                                // Parse the local address to get the port
                                if let Some(local_addr) = parts.get(4) {
                                    if local_addr.ends_with(&format!(":{}", port)) {
                                        let address = if let Some(colon_pos) = local_addr.rfind(':')
                                        {
                                            local_addr[..colon_pos].to_string()
                                        } else {
                                            "*".to_string()
                                        };

                                        return Ok(Some(ProcessInfo {
                                            pid,
                                            name,
                                            command: full_command,
                                            executable_path,
                                            working_directory,
                                            port,
                                            protocol: protocol.to_string(),
                                            address,
                                            inode: None,
                                        }));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
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

    /// Fast netstat check for specific port
    async fn try_netstat_specific_port(
        &self,
        port: u16,
        protocol: &str,
    ) -> Result<Option<ProcessInfo>> {
        let mut cmd = TokioCommand::new("netstat");
        cmd.arg("-n") // Numeric display
            .arg("-p"); // Process info

        match protocol.to_lowercase().as_str() {
            "tcp" => {
                cmd.arg("-t") // TCP only
                    .arg("-l"); // Listening
            }
            "udp" => {
                cmd.arg("-u") // UDP only
                    .arg("-l"); // Listening
            }
            _ => {
                cmd.arg("-t") // Default TCP
                    .arg("-l"); // Listening
            }
        }

        let output = cmd.output().await?;
        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.trim().is_empty() || line.contains("Proto") {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 7 {
                continue;
            }

            let local_address = fields[3];
            if !local_address.ends_with(&format!(":{}", port)) {
                continue;
            }

            // Extract PID from last field (format: pid/program_name)
            if let Some(pid_program) = fields.get(6) {
                if let Some(slash_pos) = pid_program.find('/') {
                    if let Ok(pid) = pid_program[..slash_pos].parse::<u32>() {
                        let address = if let Some(colon_pos) = local_address.rfind(':') {
                            local_address[..colon_pos].to_string()
                        } else {
                            "*".to_string()
                        };

                        let full_command = self
                            .get_process_command(pid)
                            .await
                            .unwrap_or_else(|_| "Unknown".to_string());
                        let name = self.extract_process_name(&full_command);
                        let executable_path = self
                            .get_process_executable(pid)
                            .await
                            .unwrap_or_else(|_| self.extract_executable_path(&full_command));
                        let working_directory = self
                            .get_process_working_directory(pid)
                            .await
                            .unwrap_or_else(|_| "Unknown".to_string());

                        return Ok(Some(ProcessInfo {
                            pid,
                            name,
                            command: full_command,
                            executable_path,
                            working_directory,
                            port,
                            protocol: protocol.to_string(),
                            address,
                            inode: None,
                        }));
                    }
                }
            }
        }

        Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_info_creation() {
        let process_info = ProcessInfo {
            pid: 1234,
            name: "test_process".to_string(),
            command: "/usr/bin/test_process --port 8080".to_string(),
            executable_path: "/usr/bin/test_process".to_string(),
            working_directory: "/home/user/project".to_string(),
            port: 8080,
            protocol: "tcp".to_string(),
            address: "127.0.0.1".to_string(),
            inode: Some(12345),
        };

        assert_eq!(process_info.pid, 1234);
        assert_eq!(process_info.name, "test_process");
        assert_eq!(process_info.port, 8080);
        assert_eq!(process_info.protocol, "tcp");
        assert_eq!(process_info.address, "127.0.0.1");
        assert_eq!(process_info.inode, Some(12345));
    }

    #[test]
    fn test_process_info_serialization() {
        let process_info = ProcessInfo {
            pid: 1234,
            name: "test_process".to_string(),
            command: "/usr/bin/test_process --port 8080".to_string(),
            executable_path: "/usr/bin/test_process".to_string(),
            working_directory: "/home/user/project".to_string(),
            port: 8080,
            protocol: "tcp".to_string(),
            address: "127.0.0.1".to_string(),
            inode: None, // Test with None value
        };

        // Test JSON serialization
        let json = serde_json::to_string(&process_info).expect("Failed to serialize");
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"name\":\"test_process\""));
        assert!(json.contains("\"port\":8080"));
        assert!(!json.contains("\"inode\"")); // Should be skipped when None

        // Test deserialization
        let deserialized: ProcessInfo = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.pid, process_info.pid);
        assert_eq!(deserialized.name, process_info.name);
        assert_eq!(deserialized.port, process_info.port);
        assert_eq!(deserialized.inode, None);
    }

    #[tokio::test]
    async fn test_port_manager_creation() {
        let port_manager = PortManager::new();
        // PortManagerが正常に作成されることを確認
        // PortManagerが正常に作成されることを確認（存在チェック）
        let _ = &port_manager;
    }

    #[tokio::test]
    async fn test_check_port_with_empty_list() {
        let port_manager = PortManager::new();

        // システムに依存するテストなので、エラーハンドリングを主にテスト
        match port_manager.check_port(65450, "tcp").await {
            Ok(result) => {
                // 結果がNoneまたはSomeのProcessInfoであることを確認
                match result {
                    Some(process_info) => {
                        assert!(process_info.pid > 0);
                        assert!(!process_info.name.is_empty());
                        assert_eq!(process_info.port, 65450);
                    }
                    None => {
                        // ポートが使用されていない場合
                    }
                }
            }
            Err(_) => {
                // システムツールがない場合のエラー
            }
        }
    }

    #[test]
    fn test_extract_process_name() {
        let port_manager = PortManager::new();

        // 様々な形式のコマンドラインからプロセス名を抽出するテスト
        assert_eq!(
            port_manager.extract_process_name("/usr/bin/node server.js"),
            "node"
        );
        assert_eq!(
            port_manager.extract_process_name("python3 app.py"),
            "python3"
        );
        assert_eq!(
            port_manager
                .extract_process_name("/Applications/Chrome.app/Contents/MacOS/Google Chrome"),
            "Google"
        );
        assert_eq!(port_manager.extract_process_name(""), "Unknown");
        assert_eq!(
            port_manager.extract_process_name("single_command"),
            "single_command"
        );
        assert_eq!(
            port_manager.extract_process_name("./local_binary --flag value"),
            "local_binary"
        );
    }

    #[test]
    fn test_extract_executable_path() {
        let port_manager = PortManager::new();

        // 様々な形式のコマンドラインから実行ファイルパスを抽出するテスト
        assert_eq!(
            port_manager.extract_executable_path("/usr/bin/node server.js"),
            "/usr/bin/node"
        );
        assert_eq!(
            port_manager.extract_executable_path("python3 app.py"),
            "python3"
        );
        assert_eq!(
            port_manager.extract_executable_path(
                "/Applications/Chrome.app/Contents/MacOS/Google Chrome --flag"
            ),
            "/Applications/Chrome.app/Contents/MacOS/Google"
        );
        assert_eq!(port_manager.extract_executable_path(""), "Unknown");
        assert_eq!(
            port_manager.extract_executable_path("single_command"),
            "single_command"
        );
    }

    #[test]
    fn test_get_display_path_development_processes() {
        let port_manager = PortManager::new();

        // 開発プロセス用のProcessInfoをテスト
        let dev_process_info = ProcessInfo {
            pid: 1234,
            name: "node".to_string(),
            command: "/usr/local/bin/node server.js".to_string(),
            executable_path: "/usr/local/bin/node".to_string(),
            working_directory: "/home/user/my-project".to_string(),
            port: 3000,
            protocol: "tcp".to_string(),
            address: "127.0.0.1".to_string(),
            inode: Some(12345),
        };

        // 開発プロセスの場合は作業ディレクトリが返されるべき
        let display_path = port_manager.get_display_path(&dev_process_info);
        assert_eq!(display_path, "/home/user/my-project");
    }

    #[test]
    fn test_get_display_path_system_processes() {
        let port_manager = PortManager::new();

        // システムプロセス用のProcessInfoをテスト
        let system_process_info = ProcessInfo {
            pid: 1234,
            name: "sshd".to_string(),
            command: "/usr/sbin/sshd -D".to_string(),
            executable_path: "/usr/sbin/sshd".to_string(),
            working_directory: "/".to_string(),
            port: 22,
            protocol: "tcp".to_string(),
            address: "0.0.0.0".to_string(),
            inode: Some(12345),
        };

        // システムプロセスの場合は実行ファイルパスが返されるべき
        let display_path = port_manager.get_display_path(&system_process_info);
        assert_eq!(display_path, "/usr/sbin/sshd");
    }

    #[test]
    fn test_get_display_path_unknown_working_directory() {
        let port_manager = PortManager::new();

        let process_info = ProcessInfo {
            pid: 1234,
            name: "test_process".to_string(),
            command: "/usr/bin/test_process".to_string(),
            executable_path: "/usr/bin/test_process".to_string(),
            working_directory: "Unknown".to_string(),
            port: 8080,
            protocol: "tcp".to_string(),
            address: "127.0.0.1".to_string(),
            inode: Some(12345),
        };

        // 作業ディレクトリが不明な場合は実行ファイルパスが返されるべき
        let display_path = port_manager.get_display_path(&process_info);
        assert_eq!(display_path, "/usr/bin/test_process");
    }

    #[tokio::test]
    async fn test_list_processes_error_handling() {
        let port_manager = PortManager::new();

        // 様々なプロトコルでのエラーハンドリングをテスト
        for protocol in ["tcp", "udp", "all", "invalid"] {
            match port_manager.list_processes(protocol).await {
                Ok(processes) => {
                    // 成功した場合、ProcessInfoのリストが返される
                    for process in processes {
                        assert!(process.pid > 0);
                        assert!(!process.name.is_empty());
                        assert!(process.port > 0);
                        assert!(!process.protocol.is_empty());
                    }
                }
                Err(_) => {
                    // システムツールがない場合やパーミッションエラー
                }
            }
        }
    }

    #[tokio::test]
    async fn test_list_processes_with_progress() {
        let port_manager = PortManager::new();

        // シンプルなプログレスコールバック（状態を変更しない）
        let progress_callback = |_message: &str| {
            // プログレスメッセージを受け取るだけのテスト
        };

        // プログレスコールバック付きでのリスト取得をテスト
        match port_manager
            .list_processes_with_progress("tcp", Some(progress_callback))
            .await
        {
            Ok(_processes) => {
                // コールバックが正常に動作したことを確認
            }
            Err(_) => {
                // システムツールがない場合のエラーも受け入れ
            }
        }
    }

    #[tokio::test]
    async fn test_empty_pid_list_get_all_process_details() {
        let port_manager = PortManager::new();

        // 空のPIDリストを渡した場合のテスト
        let result = port_manager.get_all_process_details(&[]).await;
        assert!(result.is_ok());

        let details_map = result.unwrap();
        assert!(details_map.is_empty());
    }

    #[test]
    fn test_process_details_creation() {
        // ProcessDetails構造体の作成をテスト
        let details = ProcessDetails {
            executable_path: "/usr/bin/test".to_string(),
            working_directory: "/home/user".to_string(),
        };

        assert_eq!(details.executable_path, "/usr/bin/test");
        assert_eq!(details.working_directory, "/home/user");
    }

    #[tokio::test]
    async fn test_port_manager_default() {
        let port_manager = PortManager;

        // デフォルトのPortManagerが正常に作成されることを確認
        // デフォルトのPortManagerが正常に作成されることを確認（存在チェック）
        let _ = &port_manager;
    }

    #[test]
    fn test_process_info_with_different_protocols() {
        // 異なるプロトコルでのProcessInfoをテスト
        for protocol in ["tcp", "udp", "tcp6", "udp6"] {
            let process_info = ProcessInfo {
                pid: 1234,
                name: "test_process".to_string(),
                command: "/usr/bin/test_process".to_string(),
                executable_path: "/usr/bin/test_process".to_string(),
                working_directory: "/home/user/project".to_string(),
                port: 8080,
                protocol: protocol.to_string(),
                address: "127.0.0.1".to_string(),
                inode: Some(12345),
            };

            assert_eq!(process_info.protocol, protocol);
            assert!(process_info.protocol.len() >= 3);
        }
    }

    #[test]
    fn test_process_info_edge_cases() {
        // エッジケースのProcessInfoをテスト
        let edge_cases = [
            (1, 1, "0.0.0.0"),          // 最小ポート、すべてのアドレス
            (65535, 65535, "::1"),      // 最大ポート、IPv6ローカルホスト
            (1, 22, "127.0.0.1"),       // 一般的なSSHポート
            (65535, 80, "192.168.1.1"), // HTTPポート、プライベートIP
        ];

        for (pid, port, address) in edge_cases {
            let process_info = ProcessInfo {
                pid,
                name: "test".to_string(),
                command: "test".to_string(),
                executable_path: "/usr/bin/test".to_string(),
                working_directory: "/".to_string(),
                port,
                protocol: "tcp".to_string(),
                address: address.to_string(),
                inode: Some(12345),
            };

            assert!(process_info.pid >= 1);
            // u16型なので範囲は自動的に0-65535に制限される
            assert!(process_info.port > 0);
            assert!(!process_info.address.is_empty());
        }
    }
}
