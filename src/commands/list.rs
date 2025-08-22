use crate::{port::PortManager, process::ProcessManager, Result};
use colored::Colorize;
use dialoguer::{MultiSelect, Confirm};

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
        verbose: bool,
    ) -> Result<()> {
        let port_manager = PortManager::new();
        
        let mut processes = port_manager.list_processes(protocol).await?;
        
        // ポート範囲フィルタリング
        if let Some(range) = ports_range {
            let (start, end) = Self::parse_port_range(&range)?;
            processes.retain(|p| p.port >= start && p.port <= end);
        }
        
        // プロセス名フィルタリング
        if let Some(filter_name) = filter {
            processes.retain(|p| 
                p.name.to_lowercase().contains(&filter_name.to_lowercase())
            );
        }
        
        // ソート
        match sort {
            "port" => processes.sort_by_key(|p| p.port),
            "pid" => processes.sort_by_key(|p| p.pid),
            "name" => processes.sort_by(|a, b| a.name.cmp(&b.name)),
            _ => processes.sort_by_key(|p| p.port),
        }
        
        if json {
            let json_output = serde_json::json!({
                "protocol": protocol,
                "total_processes": processes.len(),
                "processes": processes
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        } else if processes.is_empty() {
            if !quiet {
                println!("{} No ports in use found", "○".blue());
            }
        } else {
            if !quiet && !kill {
                Self::print_table(&processes, verbose);
            }
            
            // 対話的kill機能
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
    
    fn parse_port_range(range: &str) -> Result<(u16, u16)> {
        if let Some((start_str, end_str)) = range.split_once('-') {
            let start = start_str.parse::<u16>()
                .map_err(|_| crate::Error::InvalidPort(format!("Invalid start port: {}", start_str)))?;
            let end = end_str.parse::<u16>()
                .map_err(|_| crate::Error::InvalidPort(format!("Invalid end port: {}", end_str)))?;
            
            if start > end {
                return Err(crate::Error::InvalidPort("Start port is greater than end port".to_string()));
            }
            
            Ok((start, end))
        } else {
            Err(crate::Error::InvalidPort("Invalid port range format (e.g., 3000-4000)".to_string()))
        }
    }
    
    fn print_table(processes: &[crate::port::ProcessInfo], verbose: bool) {
        println!("{}", "Ports in use:".bold().green());
        println!();
        
        if verbose {
            println!("{:<8} {:<12} {:<20} {:<15} {:<40} {}", 
                "PORT".cyan().bold(), 
                "PROTOCOL".cyan().bold(), 
                "PROCESS".cyan().bold(), 
                "PID".cyan().bold(),
                "PATH".cyan().bold(),
                "COMMAND".cyan().bold()
            );
            println!("{}", "-".repeat(120));
            
            for process in processes {
                println!("{:<8} {:<12} {:<20} {:<15} {:<40} {}", 
                    process.port.to_string().white(),
                    process.protocol.to_uppercase().green(),
                    process.name.yellow(),
                    process.pid.to_string().blue(),
                    process.path.truncate_with_ellipsis(38).magenta(),
                    process.command.truncate_with_ellipsis(30).dimmed()
                );
            }
        } else {
            println!("{:<8} {:<12} {:<20} {}", 
                "PORT".cyan().bold(), 
                "PROTOCOL".cyan().bold(), 
                "PROCESS".cyan().bold(), 
                "PID".cyan().bold()
            );
            println!("{}", "-".repeat(50));
            
            for process in processes {
                println!("{:<8} {:<12} {:<20} {}", 
                    process.port.to_string().white(),
                    process.protocol.to_uppercase().green(),
                    process.name.yellow(),
                    process.pid.to_string().blue()
                );
            }
        }
        
        println!();
        println!("{} {} processes", "Total:".cyan(), processes.len().to_string().bold());
    }
    
    async fn interactive_kill(processes: Vec<crate::port::ProcessInfo>, quiet: bool) -> Result<()> {
        // killモードでは重要な操作のため、常に詳細テーブルを表示（--quietに関係なく）
        Self::print_table(&processes, true);
        println!();
        if !quiet {
            println!("{}", "Select processes to kill:".bold().yellow());
            println!();
        }
        
        // MultiSelect用のオプション作成（詳細情報付き）
        let options: Vec<String> = processes.iter().map(|p| {
            format!("{} ({}) - {} ({}) - {} | {}", 
                p.port.to_string().white(), 
                p.protocol.to_uppercase().green(), 
                p.name.yellow(),
                p.pid.to_string().blue(),
                p.path.truncate_with_ellipsis(25).magenta(),
                p.command.truncate_with_ellipsis(40).dimmed()
            )
        }).collect();
        
        let selections = match MultiSelect::new()
            .with_prompt("Select processes (Space: select, Enter: confirm, Esc/q: cancel)")
            .items(&options)
            .interact_opt()? {
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
                println!("• {} (PID: {}) - Port {}", 
                    process.name, process.pid, process.port);
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
                println!("{} 操作がキャンセルされました", "×".yellow());
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
        quiet: bool
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
                        println!("{} Killed {} (PID: {})", 
                            "✓".green(), process.name, process.pid);
                    }
                },
                Err(e) => {
                    if !quiet {
                        println!("{} Failed to kill {} (PID: {}): {}", 
                            "×".red(), process.name, process.pid, e);
                    }
                    errors.push((process, e));
                }
            }
        }
        
        // 結果サマリー
        if !quiet && selections.len() > 1 {
            println!();
            if success_count > 0 {
                println!("{} Successfully killed {} processes", 
                    "✓".green(), success_count);
            }
            if !errors.is_empty() {
                println!("{} Failed to kill {} processes", 
                    "×".red(), errors.len());
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
        assert!(result.unwrap_err().to_string().contains("Invalid start port"));

        let result = ListCommand::parse_port_range("3000");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid port range format"));

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