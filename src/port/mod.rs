use crate::Result;
use serde::{Deserialize, Serialize};
use tokio::process::Command as TokioCommand;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub port: u16,
    pub protocol: String,
    pub address: String,
    pub path: String,
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

    #[cfg(target_os = "windows")]
    pub async fn list_processes(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let protocol_flag = match protocol.to_lowercase().as_str() {
            "tcp" => "TCP",
            "udp" => "UDP",
            "all" => "",
            _ => "TCP",
        };

        let mut args = vec!["-ano"];
        if !protocol_flag.is_empty() {
            args.push("-p");
            args.push(protocol_flag);
        }

        let output = TokioCommand::new("netstat")
            .args(&args)
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
        self.parse_netstat_output(&stdout, protocol).await
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn list_processes(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        self.list_processes_unix(protocol).await
    }

    #[cfg(not(target_os = "windows"))]
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

    #[cfg(not(target_os = "windows"))]
    async fn try_lsof(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
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
        self.parse_lsof_output(&stdout, protocol).await
    }

    #[cfg(not(target_os = "windows"))]
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

    #[cfg(not(target_os = "windows"))]
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

    async fn parse_lsof_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for line in output.lines().skip(1) {
            // ヘッダー行をスキップ
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 9 {
                continue;
            }

            let command = fields[0];
            let pid_str = fields[1];
            let type_field = fields[4];
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

            // NAME列（node）からプロトコルを判定
            let protocol = if node.contains("TCP") {
                "tcp"
            } else if node.contains("UDP") {
                "udp"
            } else {
                // フォールバック：リクエストされたプロトコルを使用
                _protocol
            }
            .to_string();

            // プロセス情報を取得（完全なコマンドライン）
            let full_command = match self.get_process_command(pid).await {
                Ok(cmd) => cmd,
                Err(_) => command.to_string(),
            };

            let path = self.extract_executable_path(&full_command);

            let name = self.extract_process_name(&full_command);

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                port,
                protocol,
                address,
                path,
            });
        }

        Ok(processes)
    }

    #[cfg(target_os = "windows")]
    async fn parse_netstat_output(
        &self,
        output: &str,
        _protocol: &str,
    ) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for line in output.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 5 {
                continue;
            }

            let protocol = fields[0].to_lowercase();
            let local_address = fields[1];
            let state = fields[3];
            let pid_str = fields[4];

            // リスニング状態のみ処理
            if state != "LISTENING" {
                continue;
            }

            let pid = match pid_str.parse::<u32>() {
                Ok(pid) => pid,
                Err(_) => continue,
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

            let address = if let Some(colon_pos) = local_address.rfind(':') {
                local_address[..colon_pos].to_string()
            } else {
                "*".to_string()
            };

            // Windowsでプロセス情報を取得
            let (name, command) = match self.get_process_info_windows(pid).await {
                Ok((n, c)) => (n, c),
                Err(_) => ("Unknown".to_string(), "Unknown".to_string()),
            };

            let path = self.extract_executable_path(&command);

            processes.push(ProcessInfo {
                pid,
                name,
                command,
                port,
                protocol,
                address,
                path,
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
            let path = self.extract_executable_path(&full_command);

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                port,
                protocol,
                address,
                path,
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
            let path = self.extract_executable_path(&full_command);

            processes.push(ProcessInfo {
                pid,
                name,
                command: full_command,
                port,
                protocol,
                address,
                path,
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
            // パスから実行ファイル名だけを抽出（Windows/Unix両対応）
            #[cfg(target_os = "windows")]
            let separator = '\\';
            #[cfg(not(target_os = "windows"))]
            let separator = '/';

            if let Some(name) = first_part.split(separator).next_back() {
                // Windows環境では.exeを削除
                #[cfg(target_os = "windows")]
                {
                    if name.to_lowercase().ends_with(".exe") {
                        name[..name.len() - 4].to_string()
                    } else {
                        name.to_string()
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    name.to_string()
                }
            } else {
                first_part.to_string()
            }
        } else {
            "Unknown".to_string()
        }
    }

    fn extract_executable_path(&self, command_line: &str) -> String {
        // コマンドラインから実行ファイルのパスを抽出
        if command_line.is_empty() {
            return "Unknown".to_string();
        }

        // スペースで分割して最初の部分（実行ファイル）を取得
        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if let Some(first_part) = parts.first() {
            first_part.to_string()
        } else {
            "Unknown".to_string()
        }
    }

    #[cfg(not(target_os = "windows"))]
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

    #[cfg(target_os = "windows")]
    async fn get_process_info_windows(&self, pid: u32) -> Result<(String, String)> {
        // tasklist.exe を使用してプロセス情報を取得
        let output = TokioCommand::new("tasklist")
            .arg("/FI")
            .arg(&format!("PID eq {}", pid))
            .arg("/FO")
            .arg("CSV")
            .arg("/NH") // ヘッダーなし
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("tasklist command failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!(
                "tasklist failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if let Some(line) = lines.first() {
            // CSV形式のパース: "Image Name","PID","Session Name","Session#","Mem Usage"
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() >= 2 {
                let image_name = fields[0].trim_matches('"');
                let name = image_name.to_string();

                // 詳細なコマンド情報を取得するためにWMICを試行
                match self.get_process_command_wmic(pid).await {
                    Ok(command) => Ok((name, command)),
                    Err(_) => Ok((name.clone(), name)), // フォールバック
                }
            } else {
                Err(crate::Error::ProcessNotFound(pid))
            }
        } else {
            Err(crate::Error::ProcessNotFound(pid))
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_process_command_wmic(&self, pid: u32) -> Result<String> {
        let output = TokioCommand::new("wmic")
            .arg("process")
            .arg("where")
            .arg(&format!("ProcessId={}", pid))
            .arg("get")
            .arg("CommandLine,ExecutablePath")
            .arg("/format:csv")
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("wmic command failed: {}", e)))?;

        if !output.status.success() {
            return Err(crate::Error::ProcessNotFound(pid));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            // ヘッダーをスキップ
            if line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() >= 3 {
                let command_line = fields[1].trim();
                let executable_path = fields[2].trim();

                if !command_line.is_empty() && command_line != "NULL" {
                    return Ok(command_line.to_string());
                } else if !executable_path.is_empty() && executable_path != "NULL" {
                    return Ok(executable_path.to_string());
                }
            }
        }

        Err(crate::Error::ProcessNotFound(pid))
    }
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}
