use crate::Result;
use std::collections::HashMap;
use std::net::Ipv6Addr;
use tokio::fs as tokio_fs;

use super::ProcessInfo;

/// High-performance port manager using direct procfs access
pub struct ProcfsPortManager {
    pid_cache: HashMap<u32, ProcessDetails>,
    last_update: std::time::Instant,
    cache_ttl: std::time::Duration,
}

#[derive(Debug, Clone)]
struct ProcessDetails {
    name: String,
    command: String,
    executable_path: String,
    working_directory: String,
}

impl ProcfsPortManager {
    pub fn new() -> Self {
        Self {
            pid_cache: HashMap::new(),
            last_update: std::time::Instant::now(),
            cache_ttl: std::time::Duration::from_secs(2),
        }
    }

    /// List all processes using ports with direct procfs access
    pub async fn list_processes(&mut self, protocol: &str) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        // Read network connections from procfs
        let tcp_processes = if protocol == "tcp" || protocol == "all" {
            self.read_tcp_connections().await?
        } else {
            Vec::new()
        };

        let udp_processes = if protocol == "udp" || protocol == "all" {
            self.read_udp_connections().await?
        } else {
            Vec::new()
        };

        processes.extend(tcp_processes);
        processes.extend(udp_processes);

        // Enrich with process information
        self.enrich_with_process_info(&mut processes).await?;

        Ok(processes)
    }

    /// Check specific port using procfs
    pub async fn check_port(&mut self, port: u16, protocol: &str) -> Result<Option<ProcessInfo>> {
        let processes = self.list_processes(protocol).await?;
        Ok(processes.into_iter().find(|p| p.port == port))
    }

    /// Read TCP connections from /proc/net/tcp and /proc/net/tcp6
    async fn read_tcp_connections(&self) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        // Read IPv4 TCP connections
        if let Ok(content) = tokio_fs::read_to_string("/proc/net/tcp").await {
            processes.extend(self.parse_tcp_content(&content, false)?);
        }

        // Read IPv6 TCP connections
        if let Ok(content) = tokio_fs::read_to_string("/proc/net/tcp6").await {
            processes.extend(self.parse_tcp_content(&content, true)?);
        }

        // Filter only listening connections
        processes.retain(|p| self.is_listening_connection(p));

        Ok(processes)
    }

    /// Read UDP connections from /proc/net/udp and /proc/net/udp6
    async fn read_udp_connections(&self) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        // Read IPv4 UDP connections
        if let Ok(content) = tokio_fs::read_to_string("/proc/net/udp").await {
            processes.extend(self.parse_udp_content(&content, false)?);
        }

        // Read IPv6 UDP connections
        if let Ok(content) = tokio_fs::read_to_string("/proc/net/udp6").await {
            processes.extend(self.parse_udp_content(&content, true)?);
        }

        Ok(processes)
    }

    /// Parse TCP procfs content
    fn parse_tcp_content(&self, content: &str, is_ipv6: bool) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for line in content.lines().skip(1) {
            // Skip header line
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }

            let local_address = parts[1];
            let state = parts[3];
            let inode = parts[9];

            // Parse local address and port
            if let Some((address, port)) = self.parse_address(local_address, is_ipv6) {
                // Only process listening connections (state 0A = LISTEN)
                if state == "0A" {
                    if let Ok(inode_num) = inode.parse::<u64>() {
                        processes.push(ProcessInfo {
                            pid: 0, // Will be filled later
                            name: String::new(),
                            command: String::new(),
                            executable_path: String::new(),
                            working_directory: String::new(),
                            port,
                            protocol: "tcp".to_string(),
                            address,
                            inode: Some(inode_num),
                        });
                    }
                }
            }
        }

        Ok(processes)
    }

    /// Parse UDP procfs content
    fn parse_udp_content(&self, content: &str, is_ipv6: bool) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }

            let local_address = parts[1];
            let inode = parts[9];

            if let Some((address, port)) = self.parse_address(local_address, is_ipv6) {
                if let Ok(inode_num) = inode.parse::<u64>() {
                    processes.push(ProcessInfo {
                        pid: 0, // Will be filled later
                        name: String::new(),
                        command: String::new(),
                        executable_path: String::new(),
                        working_directory: String::new(),
                        port,
                        protocol: "udp".to_string(),
                        address,
                        inode: Some(inode_num),
                    });
                }
            }
        }

        Ok(processes)
    }

    /// Parse address:port from procfs format
    fn parse_address(&self, address_port: &str, is_ipv6: bool) -> Option<(String, u16)> {
        let colon_pos = address_port.rfind(':')?;
        let address_hex = &address_port[..colon_pos];
        let port_hex = &address_port[colon_pos + 1..];

        let port = u16::from_str_radix(port_hex, 16).ok()?;

        let address = if is_ipv6 {
            self.parse_ipv6_address(address_hex)
        } else {
            self.parse_ipv4_address(address_hex)
        };

        Some((address, port))
    }

    /// Parse IPv4 address from hex format
    fn parse_ipv4_address(&self, hex: &str) -> String {
        if hex.len() != 8 {
            return "*".to_string();
        }

        let bytes = (0..4)
            .map(|i| u8::from_str_radix(&hex[i * 2..(i + 1) * 2], 16).unwrap_or(0))
            .collect::<Vec<_>>();

        if bytes == [0, 0, 0, 0] {
            "*".to_string()
        } else {
            format!("{}.{}.{}.{}", bytes[3], bytes[2], bytes[1], bytes[0])
        }
    }

    /// Parse IPv6 address from hex format
    fn parse_ipv6_address(&self, hex: &str) -> String {
        if hex.len() != 32 {
            return "*".to_string();
        }

        if hex == "00000000000000000000000000000000" {
            return "*".to_string();
        }

        // Convert hex string to IPv6 address
        let mut bytes = [0u8; 16];
        for i in 0..16 {
            bytes[i] = u8::from_str_radix(&hex[i * 2..(i + 1) * 2], 16).unwrap_or(0);
        }

        let addr = Ipv6Addr::from(bytes);
        addr.to_string()
    }

    /// Check if connection is in listening state
    fn is_listening_connection(&self, _process: &ProcessInfo) -> bool {
        // For TCP, we already filtered by state in parse_tcp_content
        // For UDP, all bound sockets are considered "listening"
        true
    }

    /// Enrich process info by finding PIDs via inode matching
    async fn enrich_with_process_info(&mut self, processes: &mut Vec<ProcessInfo>) -> Result<()> {
        // Create inode to process mapping
        let mut inode_to_pid: HashMap<u64, u32> = HashMap::new();

        // Scan all processes to find socket inodes
        if let Ok(proc_entries) = tokio_fs::read_dir("/proc").await {
            let mut entries = proc_entries;
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Some(filename) = entry.file_name().to_str() {
                    if let Ok(pid) = filename.parse::<u32>() {
                        self.scan_process_fds(pid, &mut inode_to_pid).await;
                    }
                }
            }
        }

        // Update processes with PID information
        for process in processes.iter_mut() {
            if let Some(inode) = process.inode {
                if let Some(&pid) = inode_to_pid.get(&inode) {
                    process.pid = pid;
                    self.update_process_details(process).await?;
                }
            }
        }

        // Filter out processes without PID (orphaned sockets)
        processes.retain(|p| p.pid != 0);

        Ok(())
    }

    /// Scan process file descriptors to find socket inodes
    async fn scan_process_fds(&self, pid: u32, inode_to_pid: &mut HashMap<u64, u32>) {
        let fd_path = format!("/proc/{pid}/fd");
        if let Ok(mut fd_entries) = tokio_fs::read_dir(&fd_path).await {
            while let Ok(Some(fd_entry)) = fd_entries.next_entry().await {
                if let Ok(link_target) = tokio_fs::read_link(fd_entry.path()).await {
                    if let Some(target_str) = link_target.to_str() {
                        // Look for socket inodes: socket:[12345]
                        if target_str.starts_with("socket:[") && target_str.ends_with(']') {
                            let inode_str = &target_str[8..target_str.len() - 1];
                            if let Ok(inode) = inode_str.parse::<u64>() {
                                inode_to_pid.insert(inode, pid);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Update process details from procfs
    async fn update_process_details(&mut self, process: &mut ProcessInfo) -> Result<()> {
        let now = std::time::Instant::now();

        // Use cache if available and fresh
        if now.duration_since(self.last_update) < self.cache_ttl {
            if let Some(cached) = self.pid_cache.get(&process.pid) {
                process.name = cached.name.clone();
                process.command = cached.command.clone();
                process.executable_path = cached.executable_path.clone();
                process.working_directory = cached.working_directory.clone();
                return Ok(());
            }
        }

        // Read from procfs
        let details = self.read_process_details(process.pid).await?;
        process.name = details.name.clone();
        process.command = details.command.clone();
        process.executable_path = details.executable_path.clone();
        process.working_directory = details.working_directory.clone();

        // Update cache
        self.pid_cache.insert(process.pid, details);
        self.last_update = now;

        Ok(())
    }

    /// Read detailed process information from procfs
    async fn read_process_details(&self, pid: u32) -> Result<ProcessDetails> {
        let mut details = ProcessDetails {
            name: "Unknown".to_string(),
            command: "Unknown".to_string(),
            executable_path: "Unknown".to_string(),
            working_directory: "Unknown".to_string(),
        };

        // Read process name from /proc/pid/comm
        if let Ok(name) = tokio_fs::read_to_string(format!("/proc/{pid}/comm")).await {
            details.name = name.trim().to_string();
        }

        // Read command line from /proc/pid/cmdline
        if let Ok(cmdline) = tokio_fs::read(format!("/proc/{pid}/cmdline")).await {
            let command = String::from_utf8_lossy(&cmdline)
                .replace('\0', " ")
                .trim()
                .to_string();
            if !command.is_empty() {
                details.command = command;
                // Extract executable path from command line
                if let Some(first_arg) = details.command.split_whitespace().next() {
                    details.executable_path = first_arg.to_string();
                }
            }
        }

        // Read working directory from /proc/pid/cwd
        if let Ok(cwd) = tokio_fs::read_link(format!("/proc/{pid}/cwd")).await {
            if let Some(cwd_str) = cwd.to_str() {
                details.working_directory = cwd_str.to_string();
            }
        }

        // Try to get actual executable path from /proc/pid/exe
        if let Ok(exe) = tokio_fs::read_link(format!("/proc/{pid}/exe")).await {
            if let Some(exe_str) = exe.to_str() {
                details.executable_path = exe_str.to_string();
            }
        }

        Ok(details)
    }

    /// Get display path for process (prefers working directory for dev processes)
    pub fn get_display_path(&self, process_info: &ProcessInfo) -> String {
        // Same logic as original PortManager
        if process_info.working_directory != "/" && process_info.working_directory != "Unknown" {
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

        process_info.executable_path.clone()
    }

    /// Clear cache (useful for forcing refresh)
    pub fn clear_cache(&mut self) {
        self.pid_cache.clear();
        self.last_update = std::time::Instant::now() - self.cache_ttl;
    }
}

impl Default for ProcfsPortManager {
    fn default() -> Self {
        Self::new()
    }
}
