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
        if cfg!(target_os = "windows") {
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
                return Err(crate::Error::CommandFailed(format!("netstat failed: {}", stderr)));
            }
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.parse_netstat_output(&stdout, protocol).await
        } else {
            self.list_processes_unix(protocol).await
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    async fn list_processes_unix(&self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let protocol_flag = match protocol.to_lowercase().as_str() {
            "tcp" => "-iTCP",
            "udp" => "-iUDP",
            "all" => "-i",
            _ => "-iTCP",
        };
        
        let output = TokioCommand::new("lsof")
            .arg("-n")  // 数値表示（ホスト名解決なし）
            .arg("-P")  // ポート番号を数値表示
            .arg(protocol_flag)
            .arg("-sTCP:LISTEN") // リスニング状態のみ（TCP）
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("lsof command failed: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::CommandFailed(format!("lsof failed: {}", stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_lsof_output(&stdout, protocol).await
    }
    
    
    async fn parse_lsof_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        
        for line in output.lines().skip(1) { // ヘッダー行をスキップ
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
            
            let protocol = if type_field.contains("TCP") { "tcp" } else { "udp" }.to_string();
            
            // プロセス情報を取得（完全なコマンドライン）
            let full_command = match self.get_process_command(pid).await {
                Ok(cmd) => cmd,
                Err(_) => command.to_string(),
            };
            
            processes.push(ProcessInfo {
                pid,
                name: command.to_string(),
                command: full_command,
                port,
                protocol,
                address,
            });
        }
        
        Ok(processes)
    }
    
    #[cfg(target_os = "windows")]
    async fn parse_netstat_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
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
            
            // Windowsでプロセス名を取得（簡単な実装）
            let (name, command) = ("Unknown".to_string(), "Unknown".to_string());
            
            processes.push(ProcessInfo {
                pid,
                name,
                command,
                port,
                protocol,
                address,
            });
        }
        
        Ok(processes)
    }

    async fn parse_netstat_output(&self, output: &str, _protocol: &str) -> Result<Vec<ProcessInfo>> {
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
            
            // Windowsでプロセス名を取得（簡単な実装）
            let (name, command) = ("Unknown".to_string(), "Unknown".to_string());
            
            processes.push(ProcessInfo {
                pid,
                name,
                command,
                port,
                protocol,
                address,
            });
        }
        
        Ok(processes)
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
    
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}