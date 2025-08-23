use crate::{port::incremental::IncrementalPortManager, Result};
use colored::Colorize;
use std::time::Duration;

impl super::ListCommand {
    pub(super) async fn execute_watch_mode(
        manager: &mut IncrementalPortManager,
        protocol: &str,
        ports_range: Option<String>,
        filter: Option<String>,
        sort: &str,
        quiet: bool,
    ) -> Result<()> {
        if !quiet {
            println!(
                "{} Starting port monitoring... (Press Ctrl+C to stop)",
                "●".green()
            );
            println!();
        }

        // Start background monitoring
        let monitor_handle = manager.start_monitoring(vec![protocol.to_string()]).await;

        let mut last_display = std::time::Instant::now();
        let display_interval = Duration::from_secs(1);

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
                            println!(
                                "{} Port Monitor - {} | Last updated: {}",
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
}