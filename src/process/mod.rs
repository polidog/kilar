use crate::Result;
use tokio::process::Command as TokioCommand;

pub struct ProcessManager;

impl ProcessManager {
    pub fn new() -> Self {
        Self
    }

    /// Check if a system command is available
    #[allow(dead_code)] // Only used on Windows platforms
    async fn is_command_available(&self, command: &str) -> bool {
        match TokioCommand::new(command).arg("--help").output().await {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub async fn kill_process(&self, pid: u32) -> Result<()> {
        if cfg!(target_os = "windows") {
            // Check if taskkill is available first
            if !self.is_command_available("taskkill").await {
                return Err(crate::Error::CommandFailed(
                    "taskkill command not available. This may occur in restricted CI environments or containers.".to_string()
                ));
            }

            let output = TokioCommand::new("taskkill")
                .arg("/F")
                .arg("/PID")
                .arg(pid.to_string())
                .output()
                .await
                .map_err(|e| {
                    crate::Error::CommandFailed(format!("taskkill command failed: {}. This may indicate restricted CI environment permissions.", e))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("not found") {
                    return Err(crate::Error::ProcessNotFound(pid));
                } else if stderr.contains("Access is denied") {
                    return Err(crate::Error::PermissionDenied(
                        "プロセス終了の権限がありません。管理者として実行してください。"
                            .to_string(),
                    ));
                }
                return Err(crate::Error::CommandFailed(format!(
                    "Failed to kill process: {}",
                    stderr
                )));
            }

            Ok(())
        } else {
            self.kill_process_unix(pid).await
        }
    }

    #[cfg(not(target_os = "windows"))]
    async fn kill_process_unix(&self, pid: u32) -> Result<()> {
        // まずSIGTERMで優雅な終了を試行
        let output = TokioCommand::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("kill command failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No such process") {
                return Err(crate::Error::ProcessNotFound(pid));
            } else if stderr.contains("Operation not permitted") {
                return Err(crate::Error::PermissionDenied(
                    "プロセス終了の権限がありません。sudoで実行してください。".to_string(),
                ));
            }
            return Err(crate::Error::CommandFailed(format!(
                "Failed to kill process: {}",
                stderr
            )));
        }

        // 少し待ってプロセスが終了したか確認
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // プロセスがまだ存在するかチェック
        if self.process_exists(pid).await? {
            // SIGKILLで強制終了
            let output = TokioCommand::new("kill")
                .arg("-KILL")
                .arg(pid.to_string())
                .output()
                .await
                .map_err(|e| crate::Error::CommandFailed(format!("kill -KILL failed: {}", e)))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::Error::CommandFailed(format!(
                    "Failed to force kill process: {}",
                    stderr
                )));
            }
        }

        Ok(())
    }

    async fn process_exists(&self, pid: u32) -> Result<bool> {
        if cfg!(target_os = "windows") {
            // Check if tasklist is available first
            if !self.is_command_available("tasklist").await {
                // If tasklist isn't available, assume process doesn't exist or is not accessible
                return Ok(false);
            }

            let output = TokioCommand::new("tasklist")
                .arg("/FI")
                .arg(format!("PID eq {}", pid))
                .arg("/FO")
                .arg("CSV")
                .arg("/NH")
                .output()
                .await
                .map_err(|e| {
                    crate::Error::CommandFailed(format!("tasklist command failed: {}. This may indicate restricted CI environment permissions.", e))
                })?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(!stdout.trim().is_empty() && !stdout.contains("INFO: No tasks are running"))
            } else {
                Ok(false)
            }
        } else {
            self.process_exists_unix(pid).await
        }
    }

    #[cfg(not(target_os = "windows"))]
    async fn process_exists_unix(&self, pid: u32) -> Result<bool> {
        let output = TokioCommand::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("ps command failed: {}", e)))?;

        Ok(output.status.success())
    }

    pub async fn get_process_info(&self, pid: u32) -> Result<(String, String)> {
        if cfg!(target_os = "windows") {
            // Check if tasklist is available first
            if !self.is_command_available("tasklist").await {
                return Ok((
                    "Unknown".to_string(),
                    "Unknown (tasklist not available in CI)".to_string(),
                ));
            }

            let output = TokioCommand::new("tasklist")
                .arg("/FI")
                .arg(format!("PID eq {}", pid))
                .arg("/FO")
                .arg("CSV")
                .arg("/NH")
                .output()
                .await
                .map_err(|e| {
                    crate::Error::CommandFailed(format!("tasklist command failed: {}. This may indicate restricted CI environment permissions.", e))
                })?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    let fields: Vec<&str> = line.split(',').collect();
                    if !fields.is_empty() {
                        let name = fields[0].trim_matches('"').to_string();
                        return Ok((name.clone(), name));
                    }
                }
            }

            Err(crate::Error::ProcessNotFound(pid))
        } else {
            self.get_process_info_unix(pid).await
        }
    }

    #[cfg(not(target_os = "windows"))]
    async fn get_process_info_unix(&self, pid: u32) -> Result<(String, String)> {
        let output = TokioCommand::new("ps")
            .arg("-p")
            .arg(pid.to_string())
            .arg("-o")
            .arg("comm=,command=")
            .output()
            .await
            .map_err(|e| crate::Error::CommandFailed(format!("ps command failed: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().nth(1) {
                // Skip header
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() >= 2 {
                    return Ok((parts[0].to_string(), parts[1].to_string()));
                } else if parts.len() == 1 {
                    return Ok((parts[0].to_string(), parts[0].to_string()));
                }
            }
        }

        Err(crate::Error::ProcessNotFound(pid))
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
