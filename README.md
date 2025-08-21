# kilar

A CLI tool for managing port processes during development.

## Overview

`kilar` is a command-line tool designed to solve the common problem of "port already in use" during development. It allows you to easily check which processes are using specific ports and kill them when necessary.

## Problem it Solves

During web development, these issues frequently occur:
- Getting "port already in use" errors when starting development servers
- Manually finding which process is using a port is cumbersome
- Having to look up PIDs and manually kill processes is time-consuming

## Features

### 1. Check Port Usage
Display detailed information about processes using a specific port.

```bash
kilar check <port>
# or
kilar c <port>
```

**Information displayed:**
- Process ID (PID)
- Process name
- Command line
- Port number
- Protocol (TCP/UDP)

### 2. Kill Process
Stop processes using a specific port.

```bash
kilar kill <port>
# or
kilar k <port>
```

**Behavior:**
- Shows interactive confirmation prompt (default)
- Force kill option `-f, --force` to skip confirmation
- Allows selection when multiple processes use the same port

### 3. List Used Ports
Display all currently used ports and their processes.

```bash
kilar list
# or
kilar ls
```

**Display options:**
- `--ports <range>` : Show specific port range only (e.g., 3000-4000)
- `--sort <field>` : Sort order (port, pid, name)
- `--filter <keyword>` : Filter by process name

## Command Structure

```
kilar <command> [options] [arguments]

Commands:
  check, c <port>    Check port usage
  kill, k <port>     Kill process using the port
  list, ls           List all used ports
  help, h            Show help
  version, v         Show version

Global Options:
  -v, --verbose      Show verbose output
  -q, --quiet        Show minimal output only
  --json             Output in JSON format
```

## Usage Examples

### Example 1: Check port 3000
```bash
$ kilar check 3000
Port 3000 is in use by:
  PID: 12345
  Process: node
  Command: node server.js
  Protocol: TCP
```

### Example 2: Kill process on port 3000
```bash
$ kilar kill 3000
Found process using port 3000:
  PID: 12345
  Process: node
  Command: node server.js

Are you sure you want to kill this process? (y/N): y
Process 12345 terminated successfully.
```

### Example 3: List ports in range
```bash
$ kilar list --ports 3000-4000
Port    PID     Process         Protocol
3000    12345   node            TCP
3001    12346   python          TCP
3306    23456   mysqld          TCP
```

## Error Handling

- Shows appropriate message when port is not in use
- Suggests sudo execution when permission is insufficient
- Shows error message for invalid port numbers

## Platform Support

- macOS
- Linux
- Windows (limited support)

## Technical Specifications

### Development Language
- Rust

### Dependencies
- System commands: `lsof` (macOS/Linux), `netstat` (Windows)
- Rust crates:
  - `clap`: Command-line argument parser
  - `serde`: For JSON output
  - `colored`: Color output
  - `dialoguer`: Interactive prompts

### Build Requirements
- Rust 1.70.0 or higher
- Cargo

## Installation

```bash
# Using Cargo
cargo install kilar

# Or build from source
git clone https://github.com/polidog/kilar
cd kilar
cargo build --release
```

## Configuration File (Optional)

Customize settings with `~/.config/kilar/config.toml`:

```toml
[defaults]
force_kill = false
output_format = "table"  # table, json, minimal
color_output = true

[aliases]
# Custom alias definitions
dev = "check 3000"
```

## Future Enhancements

- [ ] Automatic port release and restart functionality
- [ ] Process group management
- [ ] Port usage history tracking
- [ ] Web UI dashboard
- [ ] Docker/container port management

## License

MIT

## Author

polidog