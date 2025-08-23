use crate::{port::adaptive::PerformanceProfile, port::incremental::IncrementalPortManager, Result};
use colored::Colorize;
use dialoguer::{Confirm, MultiSelect};

pub struct ListCommandV2;

impl ListCommandV2 {
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

        // Apply filters and sorting (same as original)
        if let Some(range) = ports_range {
            let (start, end) = Self::parse_port_range(&range)?;
            processes.retain(|p| p.port >= start && p.port <= end);
        }

        if let Some(filter_name) = filter {
            processes.retain(|p| p.name.to_lowercase().contains(&filter_name.to_lowercase()));
        }

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

    async fn execute_watch_mode(
        manager: &mut IncrementalPortManager,
        protocol: &str,
        ports_range: Option<String>,
        filter: Option<String>,
        sort: &str,
        quiet: bool,
    ) -> Result<()> {
        if !quiet {
            println!("{} Starting port monitoring... (Press Ctrl+C to stop)", "●".green());
            println!();
        }

        // Start background monitoring
        let monitor_handle = manager.start_monitoring(vec![protocol.to_string()]).await;

        let mut last_display = std::time::Instant::now();
        let display_interval = std::time::Duration::from_secs(1);

        let result = loop {
            tokio::select! {
                _ = tokio::time::sleep(display_interval) => {
                    if last_display.elapsed() >= display_interval {
                        let mut processes = manager.get_processes(protocol).await?;
                        
                        // Apply same filters
                        if let Some(ref range) = ports_range {
                            let (start, end) = Self::parse_port_range(range)?;
                            processes.retain(|p| p.port >= start && p.port <= end);
                        }

                        if let Some(ref filter_name) = filter {
                            processes.retain(|p| p.name.to_lowercase().contains(&filter_name.to_lowercase()));
                        }

                        match sort {
                            "port" => processes.sort_by_key(|p| p.port),
                            "pid" => processes.sort_by_key(|p| p.pid),
                            "name" => processes.sort_by(|a, b| a.name.cmp(&b.name)),
                            _ => processes.sort_by_key(|p| p.port),
                        }

                        // Clear screen and show updated results
                        if !quiet {
                            print!("\x1B[2J\x1B[1;1H"); // Clear screen
                            println!("{} Port Monitor - {} | Last updated: {}", 
                                "●".green(), 
                                protocol.to_uppercase(),
                                chrono::Utc::now().format("%H:%M:%S")
                            );
                            println!();
                            
                            if processes.is_empty() {
                                println!("{} No ports in use found", "○".blue());
                            } else {
                                Self::print_table(&processes);
                            }
                        }

                        last_display = std::time::Instant::now();
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    break Ok(());
                }
            }
        };

        // Stop monitoring
        monitor_handle.abort();

        if !quiet {
            println!();
            println!("{} Monitoring stopped", "○".blue());
        }

        result
    }

    fn parse_port_range(range: &str) -> Result<(u16, u16)> {
        if let Some((start_str, end_str)) = range.split_once('-') {
            let start = start_str.parse::<u16>().map_err(|_| {
                crate::Error::InvalidPort(format!("Invalid start port: {start_str}"))
            })?;
            let end = end_str
                .parse::<u16>()
                .map_err(|_| crate::Error::InvalidPort(format!("Invalid end port: {end_str}")))?;

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

    fn print_table(processes: &[crate::port::ProcessInfo]) {
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
            let display_path = if process.working_directory != "/" && process.working_directory != "Unknown" {
                let is_dev_process = process.executable_path.contains("/node")
                    || process.executable_path.contains("/python")
                    || process.executable_path.contains("/ruby")
                    || process.executable_path.contains("/java")
                    || process.command.contains("npm")
                    || process.command.contains("yarn")
                    || process.command.contains("pnpm")
                    || process.command.contains("next")
                    || process.command.contains("serve")
                    || process.command.contains("dev");

                if is_dev_process {
                    &process.working_directory
                } else {
                    &process.executable_path
                }
            } else {
                &process.executable_path
            };

            println!(
                "{:<8} {:<12} {:<20} {:<10} {:<40} {}",
                process.port.to_string().white(),
                process.protocol.to_uppercase().green(),
                Self::truncate_with_ellipsis(&process.name, 18).yellow(),
                process.pid.to_string().blue(),
                Self::truncate_with_ellipsis(display_path, 38).cyan(),
                Self::truncate_with_ellipsis(&process.command, 40).dimmed()
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

        let options: Vec<String> = processes
            .iter()
            .map(|p| {
                let display_path = if p.working_directory != "/" && p.working_directory != "Unknown" {
                    &p.working_directory
                } else {
                    &p.executable_path
                };
                
                format!(
                    "Port {} ({}) | {} (PID:{}) | Path: {} | Cmd: {}",
                    p.port.to_string().white(),
                    p.protocol.to_uppercase().green(),
                    p.name.yellow(),
                    p.pid.to_string().blue(),
                    Self::truncate_with_ellipsis(display_path, 45).cyan(),
                    Self::truncate_with_ellipsis(&p.command, 40).dimmed()
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

        // Show selected processes and confirm
        if !quiet {
            println!();
            println!("{}", "Selected processes:".bold().cyan());
            for &idx in &selections {
                let process = &processes[idx];
                let display_path = if process.working_directory != "/" && process.working_directory != "Unknown" {
                    &process.working_directory
                } else {
                    &process.executable_path
                };
                
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

        // Kill processes
        let process_manager = crate::process::ProcessManager::new();
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

        // Show summary
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

        // Return first error if any
        if let Some((_, first_error)) = errors.first() {
            return Err(first_error.clone());
        }

        Ok(())
    }

    fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
}