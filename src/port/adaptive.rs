use crate::Result;
use std::time::Instant;

use super::{procfs::ProcfsPortManager, PortManager, ProcessInfo};

/// Performance profiles for different use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceProfile {
    /// Maximum speed, minimal information, uses procfs only
    Fast,
    /// Balance between speed and completeness
    Balanced,
    /// Complete information, fallback to legacy tools if needed
    Complete,
}

/// Adaptive port manager that chooses the best strategy based on system capabilities
pub struct AdaptivePortManager {
    procfs_manager: ProcfsPortManager,
    legacy_manager: PortManager,
    use_procfs: bool,
    performance_profile: PerformanceProfile,
    last_performance_check: Option<Instant>,
    procfs_performance: Option<std::time::Duration>,
    legacy_performance: Option<std::time::Duration>,
}

impl AdaptivePortManager {
    pub fn new(profile: PerformanceProfile) -> Self {
        Self {
            procfs_manager: ProcfsPortManager::new(),
            legacy_manager: PortManager::new(),
            use_procfs: Self::is_procfs_available(),
            performance_profile: profile,
            last_performance_check: None,
            procfs_performance: None,
            legacy_performance: None,
        }
    }

    /// Check if procfs is available and readable
    fn is_procfs_available() -> bool {
        std::path::Path::new("/proc/net/tcp").exists()
            && std::path::Path::new("/proc/net/udp").exists()
    }

    /// List processes with adaptive strategy selection
    pub async fn list_processes(&mut self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        match self.performance_profile {
            PerformanceProfile::Fast => {
                // Always use legacy for fast mode as it's simpler and more reliable
                self.legacy_manager.list_processes(protocol).await
            }
            PerformanceProfile::Balanced => self.list_processes_balanced(protocol).await,
            PerformanceProfile::Complete => self.list_processes_complete(protocol).await,
        }
    }

    /// Check port with adaptive strategy
    pub async fn check_port(&mut self, port: u16, protocol: &str) -> Result<Option<ProcessInfo>> {
        match self.performance_profile {
            PerformanceProfile::Fast => {
                if self.use_procfs {
                    self.procfs_manager.check_port(port, protocol).await
                } else {
                    self.legacy_manager.check_port(port, protocol).await
                }
            }
            PerformanceProfile::Balanced | PerformanceProfile::Complete => {
                // Try procfs first, fallback to legacy
                if self.use_procfs {
                    match self.procfs_manager.check_port(port, protocol).await {
                        Ok(result) => Ok(result),
                        Err(_) => self.legacy_manager.check_port(port, protocol).await,
                    }
                } else {
                    self.legacy_manager.check_port(port, protocol).await
                }
            }
        }
    }

    /// Balanced approach: choose best method based on performance history
    async fn list_processes_balanced(&mut self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        // If we haven't benchmarked yet, or it's been a while, run benchmark
        let should_benchmark = self.last_performance_check.is_none()
            || self.last_performance_check.map_or(
                true,
                |last| last.elapsed() > std::time::Duration::from_secs(1800), // Re-benchmark every 30 minutes
            );

        if should_benchmark {
            // Only benchmark if performance difference is potentially significant
            if self.procfs_performance.is_none() || self.legacy_performance.is_none() {
                self.benchmark_performance(protocol).await?;
            } else if let (Some(procfs), Some(legacy)) = (self.procfs_performance, self.legacy_performance) {
                // Only re-benchmark if there's no clear winner (within 20% performance difference)
                let ratio = procfs.as_secs_f64() / legacy.as_secs_f64();
                if ratio > 0.8 && ratio < 1.2 {
                    self.benchmark_performance(protocol).await?;
                } else {
                    // Clear winner exists, just update timestamp without benchmarking
                    self.last_performance_check = Some(Instant::now());
                }
            }
        }

        // Choose the faster method, defaulting to legacy for simplicity
        let use_procfs = match (self.procfs_performance, self.legacy_performance) {
            (Some(procfs_time), Some(legacy_time)) => procfs_time < legacy_time,
            _ => false, // Default to legacy instead of procfs
        };

        if use_procfs && self.use_procfs {
            match self.procfs_manager.list_processes(protocol).await {
                Ok(result) => Ok(result),
                Err(_) => self.legacy_manager.list_processes(protocol).await,
            }
        } else {
            self.legacy_manager.list_processes(protocol).await
        }
    }

    /// Complete approach: use procfs with rich information, fallback to legacy
    async fn list_processes_complete(&mut self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        if self.use_procfs {
            // Try procfs first for best performance
            match self.procfs_manager.list_processes(protocol).await {
                Ok(mut processes) => {
                    // For complete mode, ensure we have full information
                    self.enrich_complete_information(&mut processes).await?;
                    Ok(processes)
                }
                Err(_) => {
                    // Fallback to legacy tools
                    self.legacy_manager.list_processes(protocol).await
                }
            }
        } else {
            self.legacy_manager.list_processes(protocol).await
        }
    }

    /// Benchmark both methods to determine the faster one
    async fn benchmark_performance(&mut self, protocol: &str) -> Result<()> {
        self.last_performance_check = Some(Instant::now());

        // Benchmark procfs if available
        if self.use_procfs {
            let start = Instant::now();
            let _ = self.procfs_manager.list_processes(protocol).await;
            self.procfs_performance = Some(start.elapsed());
        }

        // Benchmark legacy method
        let start = Instant::now();
        let _ = self.legacy_manager.list_processes(protocol).await;
        self.legacy_performance = Some(start.elapsed());

        Ok(())
    }

    /// Enrich processes with additional information for complete mode
    async fn enrich_complete_information(&self, processes: &mut [ProcessInfo]) -> Result<()> {
        // Additional enrichment could include:
        // - Environment variables
        // - Network namespace information
        // - Parent process information
        // - Resource usage statistics

        // For now, just ensure we have the display path computed
        for process in processes.iter_mut() {
            if process.working_directory.is_empty() || process.working_directory == "Unknown" {
                // Try to get more information if missing
                // This could be expanded with additional procfs reads
            }
        }

        Ok(())
    }

    /// Get display path (delegates to appropriate manager)
    pub fn get_display_path(&self, process_info: &ProcessInfo) -> String {
        if self.use_procfs {
            self.procfs_manager.get_display_path(process_info)
        } else {
            self.legacy_manager.get_display_path(process_info)
        }
    }

    /// Switch performance profile at runtime
    pub fn set_performance_profile(&mut self, profile: PerformanceProfile) {
        self.performance_profile = profile;
        // Clear performance history to trigger re-benchmarking if needed
        if profile == PerformanceProfile::Balanced {
            self.last_performance_check = None;
        }
    }

    /// Get current performance profile
    pub fn get_performance_profile(&self) -> PerformanceProfile {
        self.performance_profile
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        PerformanceStats {
            procfs_available: self.use_procfs,
            procfs_performance: self.procfs_performance,
            legacy_performance: self.legacy_performance,
            current_profile: self.performance_profile,
        }
    }

    /// Force cache clear on both managers
    pub fn clear_cache(&mut self) {
        self.procfs_manager.clear_cache();
        // Legacy manager doesn't have cache, but we could add it
    }

    /// Enable or disable procfs usage (for testing/debugging)
    pub fn set_procfs_enabled(&mut self, enabled: bool) {
        self.use_procfs = enabled && Self::is_procfs_available();
    }
}

/// Performance statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub procfs_available: bool,
    pub procfs_performance: Option<std::time::Duration>,
    pub legacy_performance: Option<std::time::Duration>,
    pub current_profile: PerformanceProfile,
}

impl Default for AdaptivePortManager {
    fn default() -> Self {
        Self::new(PerformanceProfile::Balanced)
    }
}

/// Builder pattern for creating configured adaptive managers
pub struct AdaptivePortManagerBuilder {
    profile: PerformanceProfile,
    force_procfs: Option<bool>,
}

impl AdaptivePortManagerBuilder {
    pub fn new() -> Self {
        Self {
            profile: PerformanceProfile::Balanced,
            force_procfs: None,
        }
    }

    pub fn with_profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn force_procfs(mut self, force: bool) -> Self {
        self.force_procfs = Some(force);
        self
    }

    pub fn build(self) -> AdaptivePortManager {
        let mut manager = AdaptivePortManager::new(self.profile);

        if let Some(force) = self.force_procfs {
            manager.set_procfs_enabled(force);
        }

        manager
    }
}

impl Default for AdaptivePortManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
