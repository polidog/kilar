use crate::{
    port::{adaptive::PerformanceProfile, incremental::IncrementalPortManager},
    process::ProcessManager,
    Result,
};
use colored::Colorize;
use dialoguer::{Confirm, MultiSelect};

pub struct ListCommand;

impl ListCommand {
    pub async fn execute(
        ports_range: Option<String>,
        filter: Option<String>,
        sort: &str,
        protocol: &str,
        kill: bool,
        quiet: bool,
        json: bool,
        performance_mode: Option<&str>,
        watch: bool,
    ) -> Result<()> {
        let profile = match performance_mode {
            Some("fast") => PerformanceProfile::Fast,
            Some("complete") => PerformanceProfile::Complete,
            _ => PerformanceProfile::Balanced,
        };

        let mut manager = IncrementalPortManager::new(profile);

        if watch {
            Self::execute_watch_mode(&mut manager, protocol, ports_range, filter, sort, quiet).await
        } else {
            Self::execute_single_run(
                &mut manager,
                ports_range,
                filter,
                sort,
                protocol,
                kill,
                quiet,
                json,
            )
            .await
        }
    }

    async fn execute_single_run(
        manager: &mut IncrementalPortManager,
        ports_range: Option<String>,
        filter: Option<String>,
        sort: &str,
        protocol: &str,
        kill: bool,
        quiet: bool,
        json: bool,
    ) -> Result<()> {
        let mut processes = manager.get_processes(protocol).await?;

        // ポート範囲フィルタリング
        if let Some(range) = ports_range {
            let (start, end) = Self::parse_port_range(&range)?;
            processes.retain(|p| p.port >= start && p.port <= end);
        }

        // プロセス名フィルタリング
        if let Some(filter_name) = filter {
            processes.retain(|p| p.name.to_lowercase().contains(&filter_name.to_lowercase()));
        }

        // ソート
        match sort {
            "port" => processes.sort_by_key(|p| p.port),
            "pid" => processes.sort_by_key(|p| p.pid),
            "name" => processes.sort_by(|a, b| a.name.cmp(&b.name)),
            _ => processes.sort_by_key(|p| p.port),
        }

        if json {
            let stats = manager.get_performance_stats().await;
            let json_output = serde_json::json!({
                "protocol": protocol,
                "total_processes": processes.len(),
                "processes": processes,
                "performance": {
                    "procfs_available": stats.procfs_available,
                    "profile": format!("{:?}", stats.current_profile),
                    "procfs_time_ms": stats.procfs_performance.map(|d| d.as_millis()),
                    "legacy_time_ms": stats.legacy_performance.map(|d| d.as_millis()),
                }
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        } else if processes.is_empty() {
            if !quiet {
                println!("{} No ports in use found", "○".blue());
            }
        } else {
            if !quiet && !kill {
                Self::print_table(&processes);
            }

            if kill {
                if processes.is_empty() {
                    if !quiet {
                        println!("{} No killable processes found", "○".blue());
                    }
                    return Ok(());
                }
                Self::interactive_kill(processes, quiet).await?;
            }
        }

        Ok(())
    }

    pub(crate) fn parse_port_range(range: &str) -> Result<(u16, u16)> {
        if let Some((start_str, end_str)) = range.split_once('-') {
            let start = start_str.parse::<u16>().map_err(|_| {
                crate::Error::InvalidPort(format!("Invalid start port: {}", start_str))
            })?;
            let end = end_str
                .parse::<u16>()
                .map_err(|_| crate::Error::InvalidPort(format!("Invalid end port: {}", end_str)))?;

            if start > end {
                return Err(crate::Error::InvalidPort(
                    "Start port is greater than end port".to_string(),
                ));
            }

            Ok((start, end))
        } else {
            Err(crate::Error::InvalidPort(
                "Invalid port range format (e.g., 3000-4000)".to_string(),
            ))
        }
    }

    pub(crate) fn print_table(processes: &[crate::port::ProcessInfo]) {
        println!("{}", "Ports in use:".bold().green());
        println!();

        println!(
            "{:<8} {:<12} {:<20} {:<10} {:<40} {}",
            "PORT".cyan().bold(),
            "PROTOCOL".cyan().bold(),
            "PROCESS".cyan().bold(),
            "PID".cyan().bold(),
            "PATH".cyan().bold(),
            "COMMAND".cyan().bold()
        );
        println!("{}", "-".repeat(130));

        for process in processes {
            let display_path = Self::get_display_path(process);
            println!(
                "{:<8} {:<12} {:<20} {:<10} {:<40} {}",
                process.port.to_string().white(),
                process.protocol.to_uppercase().green(),
                process.name.truncate_with_ellipsis(18).yellow(),
                process.pid.to_string().blue(),
                display_path.truncate_with_ellipsis(38).cyan(),
                process.command.truncate_with_ellipsis(40).dimmed()
            );
        }

        println!();
        println!(
            "{} {} processes",
            "Total:".cyan(),
            processes.len().to_string().bold()
        );
    }

    async fn interactive_kill(processes: Vec<crate::port::ProcessInfo>, quiet: bool) -> Result<()> {
        if !quiet {
            println!("{}", "Select processes to kill:".bold().yellow());
            println!();
        }

        // MultiSelect用のオプション作成（詳細情報付き）
        let options: Vec<String> = processes
            .iter()
            .map(|p| {
                let display_path = Self::get_display_path(p);
                format!(
                    "Port {} ({}) | {} (PID:{}) | Path: {} | Cmd: {}",
                    p.port.to_string().white(),
                    p.protocol.to_uppercase().green(),
                    p.name.yellow(),
                    p.pid.to_string().blue(),
                    display_path.truncate_with_ellipsis(45).cyan(),
                    p.command.truncate_with_ellipsis(40).dimmed()
                )
            })
            .collect();

        let selections = match MultiSelect::new()
            .with_prompt("Select processes (Space: select, Enter: confirm, Esc/q: cancel)")
            .items(&options)
            .interact_opt()?
        {
            Some(selected) => selected,
            None => {
                if !quiet {
                    println!("{} Operation cancelled", "×".yellow());
                }
                return Ok(());
            }
        };

        if selections.is_empty() {
            if !quiet {
                println!("{} No processes selected", "×".yellow());
            }
            return Ok(());
        }

        // 選択されたプロセス一覧を表示
        if !quiet {
            println!();
            println!("{}", "Selected processes:".bold().cyan());
            for &idx in &selections {
                let process = &processes[idx];
                let display_path = Self::get_display_path(process);
                println!(
                    "• {} (PID: {}) - Port {} - Path: {}",
                    process.name.yellow(),
                    process.pid.to_string().blue(),
                    process.port.to_string().white(),
                    display_path.cyan()
                );
            }
            println!();
        }

        // 確認プロンプト
        let confirmed = if selections.len() == 1 {
            Confirm::new()
                .with_prompt("Kill 1 process?")
                .default(false)
                .interact()?
        } else {
            Confirm::new()
                .with_prompt(format!("Kill {} processes?", selections.len()))
                .default(false)
                .interact()?
        };

        if !confirmed {
            if !quiet {
                println!("{} Operation cancelled", "×".yellow());
            }
            return Ok(());
        }

        // プロセス終了実行
        Self::kill_selected_processes(processes, selections, quiet).await?;

        Ok(())
    }

    async fn kill_selected_processes(
        processes: Vec<crate::port::ProcessInfo>,
        selections: Vec<usize>,
        quiet: bool,
    ) -> Result<()> {
        let process_manager = ProcessManager::new();
        let mut success_count = 0;
        let mut errors = Vec::new();

        for &idx in &selections {
            let process = &processes[idx];

            match process_manager.kill_process(process.pid).await {
                Ok(()) => {
                    success_count += 1;
                    if !quiet {
                        println!(
                            "{} Killed {} (PID: {})",
                            "✓".green(),
                            process.name,
                            process.pid
                        );
                    }
                }
                Err(e) => {
                    if !quiet {
                        println!(
                            "{} Failed to kill {} (PID: {}): {}",
                            "×".red(),
                            process.name,
                            process.pid,
                            e
                        );
                    }
                    errors.push((process, e));
                }
            }
        }

        // 結果サマリー
        if !quiet && selections.len() > 1 {
            println!();
            if success_count > 0 {
                println!(
                    "{} Successfully killed {} processes",
                    "✓".green(),
                    success_count
                );
            }
            if !errors.is_empty() {
                println!("{} Failed to kill {} processes", "×".red(), errors.len());
            }
        }

        // エラーがあった場合は最初のエラーを返す
        if let Some((_, first_error)) = errors.first() {
            return Err(first_error.clone());
        }

        Ok(())
    }
}

trait StringExt {
    fn truncate_with_ellipsis(&self, max_len: usize) -> String;
}

impl StringExt for String {
    fn truncate_with_ellipsis(&self, max_len: usize) -> String {
        if self.len() <= max_len {
            self.clone()
        } else {
            format!("{}...", &self[..max_len.saturating_sub(3)])
        }
    }
}

impl ListCommand {
    fn get_display_path(process_info: &crate::port::ProcessInfo) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_port_range() {
        let result = ListCommand::parse_port_range("3000-4000").unwrap();
        assert_eq!(result, (3000, 4000));

        let result = ListCommand::parse_port_range("80-443").unwrap();
        assert_eq!(result, (80, 443));

        let result = ListCommand::parse_port_range("8080-8080").unwrap();
        assert_eq!(result, (8080, 8080));
    }

    #[test]
    fn test_parse_port_range_invalid() {
        let result = ListCommand::parse_port_range("4000-3000");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("greater than end"));

        let result = ListCommand::parse_port_range("abc-def");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid start port"));

        let result = ListCommand::parse_port_range("3000");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid port range format"));

        let result = ListCommand::parse_port_range("3000-abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid end port"));
    }

    #[test]
    fn test_string_truncate_with_ellipsis() {
        let s = String::from("short");
        assert_eq!(s.truncate_with_ellipsis(10), "short");

        let s = String::from("this is a long string");
        assert_eq!(s.truncate_with_ellipsis(10), "this is...");

        let s = String::from("exact");
        assert_eq!(s.truncate_with_ellipsis(5), "exact");

        let s = String::from("toolong");
        assert_eq!(s.truncate_with_ellipsis(5), "to...");
    }
}
