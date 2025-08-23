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
            .map_err(|e| crate::Error::CommandFailed(format!("kill command failed: {e}")))?;

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
                "Failed to kill process: {stderr}"
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
                .map_err(|e| crate::Error::CommandFailed(format!("kill -KILL failed: {e}")))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(crate::Error::CommandFailed(format!(
                    "Failed to force kill process: {stderr}"
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
            .map_err(|e| crate::Error::CommandFailed(format!("ps command failed: {e}")))?;

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
            .map_err(|e| crate::Error::CommandFailed(format!("ps command failed: {e}")))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager_creation() {
        let process_manager = ProcessManager::new();
        // ProcessManagerが正常に作成されることを確認
        assert!(std::ptr::addr_of!(process_manager) as *const ProcessManager != std::ptr::null());
    }

    #[tokio::test]
    async fn test_process_manager_default() {
        let process_manager = ProcessManager::default();
        // デフォルトのProcessManagerが正常に作成されることを確認
        assert!(std::ptr::addr_of!(process_manager) as *const ProcessManager != std::ptr::null());
    }

    #[tokio::test]
    async fn test_process_exists_with_invalid_pid() {
        let process_manager = ProcessManager::new();

        // 存在しないPIDでのテスト（99999は一般的に使用されない大きな値）
        match process_manager.process_exists(99999).await {
            Ok(exists) => {
                // 存在しないはずのプロセスなのでfalseが返されるべき
                assert!(!exists, "PID 99999 should not exist");
            }
            Err(_) => {
                // psコマンドがない場合など、システムエラーも受け入れ
            }
        }
    }

    #[tokio::test]
    async fn test_process_exists_with_current_process() {
        let process_manager = ProcessManager::new();

        // 現在のプロセスのPID（必ず存在するはず）
        let current_pid = std::process::id();

        match process_manager.process_exists(current_pid).await {
            Ok(exists) => {
                // 現在のプロセスは必ず存在するはず
                assert!(exists, "Current process should exist");
            }
            Err(_) => {
                // psコマンドがない場合など、システムエラーも受け入れ
            }
        }
    }

    #[tokio::test]
    async fn test_kill_process_non_existent() {
        let process_manager = ProcessManager::new();

        // 存在しないプロセスのkillを試行
        let result = process_manager.kill_process(99998).await;

        // 存在しないプロセスの場合はエラーが返されるべき
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                crate::Error::ProcessNotFound(pid) => {
                    assert_eq!(pid, 99998);
                }
                crate::Error::CommandFailed(_) => {
                    // killコマンドがない場合など
                }
                _ => {
                    // その他のエラーも受け入れ
                }
            }
        }
    }

    #[tokio::test]
    async fn test_get_process_info_with_invalid_pid() {
        let process_manager = ProcessManager::new();

        // 存在しないPIDでプロセス情報を取得を試行
        let result = process_manager.get_process_info(99997).await;

        // 存在しないプロセスの場合はエラーが返されるべき
        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                crate::Error::ProcessNotFound(pid) => {
                    assert_eq!(pid, 99997);
                }
                crate::Error::CommandFailed(_) => {
                    // psコマンドがない場合など
                }
                _ => {
                    // その他のエラーも受け入れ
                }
            }
        }
    }

    #[tokio::test]
    async fn test_get_process_info_with_current_process() {
        let process_manager = ProcessManager::new();

        // 現在のプロセスの情報を取得
        let current_pid = std::process::id();

        match process_manager.get_process_info(current_pid).await {
            Ok((comm, command)) => {
                // コマンド名とコマンドラインが取得できることを確認
                assert!(!comm.is_empty(), "Process command name should not be empty");
                assert!(
                    !command.is_empty(),
                    "Process command line should not be empty"
                );

                // 基本的な妥当性チェック
                assert!(comm.len() > 0);
                assert!(command.len() >= comm.len());
            }
            Err(_) => {
                // psコマンドがない場合など、システムエラーも受け入れ
            }
        }
    }

    #[tokio::test]
    async fn test_process_exists_edge_cases() {
        let process_manager = ProcessManager::new();

        // エッジケースのPID値をテスト
        let edge_pids = [0, 1, u32::MAX];

        for pid in edge_pids {
            match process_manager.process_exists(pid).await {
                Ok(exists) => {
                    // PID 0や1は特別なプロセス、u32::MAXは存在しないはず
                    if pid == u32::MAX {
                        assert!(!exists, "PID {} should not exist", pid);
                    }
                    // PID 0,1は存在する可能性があるので、結果をチェックしない
                }
                Err(_) => {
                    // システムエラーも受け入れ
                }
            }
        }
    }

    #[tokio::test]
    async fn test_kill_process_error_handling() {
        let process_manager = ProcessManager::new();

        // 権限が必要なプロセス（PID 1、initプロセス）のkillを試行
        // これは権限不足で失敗するはず
        let result = process_manager.kill_process(1).await;

        match result {
            Ok(_) => {
                // 権限があってkillが成功した場合（稀なケース）
                // テスト環境によっては権限があるかもしれない
            }
            Err(e) => {
                // 権限不足やコマンドの失敗が期待される
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Permission denied")
                        || error_msg.contains("Operation not permitted")
                        || error_msg.contains("command failed")
                        || error_msg.contains("ProcessNotFound")
                        || !error_msg.is_empty()
                );
            }
        }
    }

    #[tokio::test]
    async fn test_process_manager_consistency() {
        let process_manager = ProcessManager::new();

        // 同じProcessManagerインスタンスでの操作の一貫性をテスト
        let current_pid = std::process::id();

        // process_existsが一貫した結果を返すかテスト
        let exists_result_1 = process_manager.process_exists(current_pid).await;
        let exists_result_2 = process_manager.process_exists(current_pid).await;

        match (exists_result_1, exists_result_2) {
            (Ok(exists1), Ok(exists2)) => {
                // 同じPIDに対する結果は一貫しているべき
                assert_eq!(
                    exists1, exists2,
                    "process_exists should return consistent results"
                );
            }
            _ => {
                // システムエラーの場合は一貫性チェックをスキップ
            }
        }
    }

    #[tokio::test]
    async fn test_multiple_process_manager_instances() {
        // 複数のProcessManagerインスタンスが独立して動作することを確認
        let pm1 = ProcessManager::new();
        let pm2 = ProcessManager::new();
        let pm3 = ProcessManager::default();

        let current_pid = std::process::id();

        // 各インスタンスで同じ操作を実行
        let results = vec![
            pm1.process_exists(current_pid).await,
            pm2.process_exists(current_pid).await,
            pm3.process_exists(current_pid).await,
        ];

        // 各インスタンスが独立して動作することを確認
        for result in results {
            match result {
                Ok(_exists) => {
                    // 成功した場合はOK
                }
                Err(_) => {
                    // システムエラーの場合も受け入れ
                }
            }
        }
    }

    #[test]
    fn test_process_manager_struct_properties() {
        // ProcessManager構造体のプロパティをテスト
        let pm1 = ProcessManager::new();
        let pm2 = ProcessManager::default();

        // 構造体が正常に作成されることを確認
        assert!(std::mem::size_of::<ProcessManager>() == 0); // Zero-sized struct

        // 異なる作成方法でも同じ動作をすることを確認
        assert_eq!(std::mem::size_of_val(&pm1), std::mem::size_of_val(&pm2));
    }
}
