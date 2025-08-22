use crate::Result;
use tokio::process::Command as TokioCommand;

pub struct ProcessManager;

impl ProcessManager {
    pub fn new() -> Self {
        Self
    }


    pub async fn kill_process(&self, pid: u32) -> Result<()> {
        self.kill_process_unix(pid).await
    }

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
        self.process_exists_unix(pid).await
    }

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
        self.get_process_info_unix(pid).await
    }

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
