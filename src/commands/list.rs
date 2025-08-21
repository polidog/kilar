use crate::{port::PortManager, Result};
use colored::Colorize;

pub struct ListCommand;

impl ListCommand {
    pub async fn execute(
        ports_range: Option<String>,
        filter: Option<String>,
        sort: &str,
        protocol: &str,
        verbose: bool,
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
            if !quiet {
                Self::print_table(&processes, verbose);
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
            println!("{:<8} {:<12} {:<20} {:<15} {}", 
                "PORT".cyan().bold(), 
                "PROTOCOL".cyan().bold(), 
                "PROCESS".cyan().bold(), 
                "PID".cyan().bold(),
                "COMMAND".cyan().bold()
            );
            println!("{}", "-".repeat(80));
            
            for process in processes {
                println!("{:<8} {:<12} {:<20} {:<15} {}", 
                    process.port.to_string().white(),
                    process.protocol.to_uppercase().green(),
                    process.name.yellow(),
                    process.pid.to_string().blue(),
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