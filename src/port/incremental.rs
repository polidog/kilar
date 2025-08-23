use crate::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::{adaptive::AdaptivePortManager, adaptive::PerformanceProfile, ProcessInfo};

/// Incremental update mechanism for port monitoring
pub struct IncrementalPortManager {
    manager: Arc<RwLock<AdaptivePortManager>>,
    cache: Arc<RwLock<PortCache>>,
    update_interval: Duration,
    last_full_update: Option<Instant>,
}

#[derive(Debug, Clone)]
struct PortCache {
    processes: HashMap<String, Vec<ProcessInfo>>, // protocol -> processes
    process_map: HashMap<u16, ProcessInfo>,       // port -> process (for quick lookup)
    last_updated: HashMap<String, Instant>,       // protocol -> timestamp
    change_log: Vec<PortChange>,
}

#[derive(Debug, Clone)]
pub struct PortChange {
    pub timestamp: Instant,
    pub change_type: ChangeType,
    pub process_info: ProcessInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Added,
    Removed,
    Modified,
}

impl IncrementalPortManager {
    pub fn new(profile: PerformanceProfile) -> Self {
        Self {
            manager: Arc::new(RwLock::new(AdaptivePortManager::new(profile))),
            cache: Arc::new(RwLock::new(PortCache::new())),
            update_interval: Duration::from_secs(5),
            last_full_update: None,
        }
    }

    /// Get current processes with incremental updates
    pub async fn get_processes(&mut self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let should_update = self.should_update(protocol).await;

        if should_update {
            self.update_processes(protocol).await?;
        }

        let cache = self.cache.read().await;
        Ok(cache.processes.get(protocol).cloned().unwrap_or_default())
    }

    /// Get a specific port's information (optimized for single port queries)
    pub async fn get_port(&mut self, port: u16, protocol: &str) -> Result<Option<ProcessInfo>> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_time) = cache.last_updated.get(protocol) {
                if cached_time.elapsed() < self.update_interval {
                    if let Some(process) = cache.process_map.get(&port) {
                        if process.protocol == protocol {
                            return Ok(Some(process.clone()));
                        }
                    }
                    return Ok(None);
                }
            }
        }

        // Cache miss or stale - check with manager
        let mut manager = self.manager.write().await;
        let result = manager.check_port(port, protocol).await?;

        // Update cache
        if let Some(ref process) = result {
            let mut cache = self.cache.write().await;
            cache.process_map.insert(port, process.clone());
            cache
                .last_updated
                .insert(protocol.to_string(), Instant::now());
        }

        Ok(result)
    }

    /// Get recent changes since a specific timestamp
    pub async fn get_changes_since(&self, since: Instant) -> Vec<PortChange> {
        let cache = self.cache.read().await;
        cache
            .change_log
            .iter()
            .filter(|change| change.timestamp > since)
            .cloned()
            .collect()
    }

    /// Get all changes in the change log
    pub async fn get_all_changes(&self) -> Vec<PortChange> {
        let cache = self.cache.read().await;
        cache.change_log.clone()
    }

    /// Start continuous monitoring in the background
    pub async fn start_monitoring(&self, protocols: Vec<String>) -> tokio::task::JoinHandle<()> {
        let manager = self.manager.clone();
        let cache = self.cache.clone();
        let update_interval = self.update_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);

            loop {
                interval.tick().await;

                for protocol in &protocols {
                    let _ = Self::background_update(&manager, &cache, protocol).await;
                }
            }
        })
    }

    /// Stop monitoring (by dropping the join handle)
    pub fn stop_monitoring(handle: tokio::task::JoinHandle<()>) {
        handle.abort();
    }

    /// Set update interval
    pub fn set_update_interval(&mut self, interval: Duration) {
        self.update_interval = interval;
    }

    /// Clear cache and force full refresh
    pub async fn force_refresh(&mut self) {
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }

        {
            let mut manager = self.manager.write().await;
            manager.clear_cache();
        }

        self.last_full_update = None;
    }

    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> super::adaptive::PerformanceStats {
        let manager = self.manager.read().await;
        manager.get_performance_stats()
    }

    async fn should_update(&self, protocol: &str) -> bool {
        let cache = self.cache.read().await;

        match cache.last_updated.get(protocol) {
            Some(last_update) => last_update.elapsed() >= self.update_interval,
            None => true,
        }
    }

    async fn update_processes(&mut self, protocol: &str) -> Result<()> {
        let current_processes = {
            let mut manager = self.manager.write().await;
            manager.list_processes(protocol).await?
        };

        let mut cache = self.cache.write().await;
        let old_processes = cache.processes.get(protocol).cloned().unwrap_or_default();

        let changes = Self::compute_changes(&old_processes, &current_processes);

        // Update cache
        cache
            .processes
            .insert(protocol.to_string(), current_processes.clone());
        cache
            .last_updated
            .insert(protocol.to_string(), Instant::now());

        // Update process map
        for process in &current_processes {
            cache.process_map.insert(process.port, process.clone());
        }

        // Remove old entries from process map
        let current_ports: HashSet<u16> = current_processes.iter().map(|p| p.port).collect();
        let old_ports: HashSet<u16> = old_processes.iter().map(|p| p.port).collect();
        for removed_port in old_ports.difference(&current_ports) {
            cache.process_map.remove(removed_port);
        }

        // Add changes to log
        cache.change_log.extend(changes);

        // Limit change log size
        if cache.change_log.len() > 1000 {
            cache.change_log.drain(0..500); // Keep last 500 changes
        }

        self.last_full_update = Some(Instant::now());

        Ok(())
    }

    async fn background_update(
        manager: &Arc<RwLock<AdaptivePortManager>>,
        cache: &Arc<RwLock<PortCache>>,
        protocol: &str,
    ) -> Result<()> {
        let current_processes = {
            let mut mgr = manager.write().await;
            mgr.list_processes(protocol).await?
        };

        let mut cache_guard = cache.write().await;
        let old_processes = cache_guard
            .processes
            .get(protocol)
            .cloned()
            .unwrap_or_default();

        let changes = Self::compute_changes(&old_processes, &current_processes);

        // Update cache
        cache_guard
            .processes
            .insert(protocol.to_string(), current_processes.clone());
        cache_guard
            .last_updated
            .insert(protocol.to_string(), Instant::now());

        // Update process map
        for process in &current_processes {
            cache_guard
                .process_map
                .insert(process.port, process.clone());
        }

        // Add changes to log
        cache_guard.change_log.extend(changes);

        // Limit change log size
        if cache_guard.change_log.len() > 1000 {
            cache_guard.change_log.drain(0..500);
        }

        Ok(())
    }

    fn compute_changes(
        old_processes: &[ProcessInfo],
        new_processes: &[ProcessInfo],
    ) -> Vec<PortChange> {
        let mut changes = Vec::new();
        let now = Instant::now();

        let old_map: HashMap<u16, &ProcessInfo> =
            old_processes.iter().map(|p| (p.port, p)).collect();
        let new_map: HashMap<u16, &ProcessInfo> =
            new_processes.iter().map(|p| (p.port, p)).collect();

        // Find added processes
        for (port, process) in &new_map {
            if !old_map.contains_key(port) {
                changes.push(PortChange {
                    timestamp: now,
                    change_type: ChangeType::Added,
                    process_info: (*process).clone(),
                });
            }
        }

        // Find removed processes
        for (port, process) in &old_map {
            if !new_map.contains_key(port) {
                changes.push(PortChange {
                    timestamp: now,
                    change_type: ChangeType::Removed,
                    process_info: (*process).clone(),
                });
            }
        }

        // Find modified processes
        for (port, new_process) in &new_map {
            if let Some(old_process) = old_map.get(port) {
                if Self::process_changed(old_process, new_process) {
                    changes.push(PortChange {
                        timestamp: now,
                        change_type: ChangeType::Modified,
                        process_info: (*new_process).clone(),
                    });
                }
            }
        }

        changes
    }

    fn process_changed(old: &ProcessInfo, new: &ProcessInfo) -> bool {
        old.pid != new.pid
            || old.name != new.name
            || old.command != new.command
            || old.executable_path != new.executable_path
    }
}

impl PortCache {
    fn new() -> Self {
        Self {
            processes: HashMap::new(),
            process_map: HashMap::new(),
            last_updated: HashMap::new(),
            change_log: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.processes.clear();
        self.process_map.clear();
        self.last_updated.clear();
        // Keep change log for history
    }
}

/// Builder for creating configured incremental managers
pub struct IncrementalPortManagerBuilder {
    profile: PerformanceProfile,
    update_interval: Duration,
}

impl IncrementalPortManagerBuilder {
    pub fn new() -> Self {
        Self {
            profile: PerformanceProfile::Balanced,
            update_interval: Duration::from_secs(5),
        }
    }

    pub fn with_profile(mut self, profile: PerformanceProfile) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_update_interval(mut self, interval: Duration) -> Self {
        self.update_interval = interval;
        self
    }

    pub fn build(self) -> IncrementalPortManager {
        let mut manager = IncrementalPortManager::new(self.profile);
        manager.set_update_interval(self.update_interval);
        manager
    }
}

impl Default for IncrementalPortManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
