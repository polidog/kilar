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
                println!("{} 使用中のポートが見つかりませんでした", "○".blue());
            }
        } else {
            if !quiet && !kill {
                Self::print_table(&processes, true);
            }
            
            // 対話的kill機能
            if kill {
                if processes.is_empty() {
                    if !quiet {
                        println!("{} 終了可能なプロセスが見つかりませんでした", "○".blue());
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
                .map_err(|_| crate::Error::InvalidPort(format!("無効な開始ポート: {}", start_str)))?;
            let end = end_str.parse::<u16>()
                .map_err(|_| crate::Error::InvalidPort(format!("無効な終了ポート: {}", end_str)))?;
            
            if start > end {
                return Err(crate::Error::InvalidPort("開始ポートが終了ポートより大きいです".to_string()));
            }
            
            Ok((start, end))
        } else {
            Err(crate::Error::InvalidPort("ポート範囲の形式が無効です (例: 3000-4000)".to_string()))
        }
    }
    
    fn print_table(processes: &[crate::port::ProcessInfo], verbose: bool) {
        println!("{}", "使用中のポート一覧:".bold());
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
        println!("合計: {} プロセス", processes.len().to_string().bold());
    }
    
    async fn interactive_kill(processes: Vec<crate::port::ProcessInfo>, quiet: bool) -> Result<()> {
        if !quiet {
            println!("{}", "使用中のポートから終了するプロセスを選択してください:".bold());
            println!();
        }
        
        // MultiSelect用のオプション作成（カラー付き）
        let options: Vec<String> = processes.iter().map(|p| {
            format!("{} ({}) - {} ({})", 
                p.port.to_string().white(), 
                p.protocol.to_uppercase().green(), 
                p.name.yellow(),
                p.pid.to_string().blue()
            )
        }).collect();
        
        let selections = MultiSelect::new()
            .with_prompt("プロセスを選択")
            .items(&options)
            .interact()?;
        
        if selections.is_empty() {
            if !quiet {
                println!("{} プロセスが選択されませんでした", "×".yellow());
            }
            return Ok(());
        }
        
        // 選択されたプロセス一覧を表示
        if !quiet {
            println!();
            println!("{}", "選択されたプロセス:".bold());
            for &idx in &selections {
                let process = &processes[idx];
                println!("• {} (PID: {}) - ポート {}", 
                    process.name, process.pid, process.port);
            }
            println!();
        }
        
        // 確認プロンプト
        let confirmed = if selections.len() == 1 {
            Confirm::new()
                .with_prompt(format!("1個のプロセスを終了しますか？"))
                .default(false)
                .interact()?
        } else {
            Confirm::new()
                .with_prompt(format!("{}個のプロセスを終了しますか？", selections.len()))
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
                        println!("{} {} (PID: {}) を終了しました", 
                            "✓".green(), process.name, process.pid);
                    }
                },
                Err(e) => {
                    if !quiet {
                        println!("{} {} (PID: {}) の終了に失敗: {}", 
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
                println!("{} {}個のプロセスを正常に終了しました", 
                    "✓".green(), success_count);
            }
            if !errors.is_empty() {
                println!("{} {}個のプロセスの終了に失敗しました", 
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