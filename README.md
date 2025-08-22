# kilar 🔌

A powerful CLI tool for managing port processes on your system. Quickly find and terminate processes using specific ports with an intuitive interface.

## Features ✨

- **Check Port Status**: Instantly see if a port is in use and which process is using it
- **Kill Processes**: Terminate processes using specific ports with safety confirmations
- **List Active Ports**: View all ports in use with detailed process information
- **Interactive Mode**: Select multiple processes to terminate with an interactive UI
- **Cross-Platform**: Works on macOS, Linux, and Windows
- **Multiple Output Formats**: Standard, JSON, and verbose modes
- **Color-Coded Output**: Easy-to-read terminal output with intuitive colors

## Installation 📦

### Using Homebrew (macOS/Linux)

```bash
# Add the tap and install
brew tap polidog/tap
brew install kilar

# Or install directly
brew install polidog/tap/kilar
```

### Using cargo

```bash
cargo install kilar
```

### From source

```bash
git clone https://github.com/polidog/kilar.git
cd kilar
cargo build --release
sudo cp target/release/kilar /usr/local/bin/
```

## Usage 🚀

### Check if a port is in use

```bash
# Check port 3000
kilar check 3000

# Check UDP port
kilar check 5353 -p udp

# JSON output
kilar check 3000 --json

# Verbose mode for detailed information
kilar check 3000 -v
```

### Kill a process using a specific port

```bash
# Kill process on port 3000
kilar kill 3000

# Force kill without confirmation
kilar kill 3000 --force

# Kill UDP process
kilar kill 5353 -p udp
```

### List all ports in use

```bash
# List all TCP ports
kilar list

# List all ports (TCP and UDP)
kilar list -p all

# Filter by port range
kilar list -r 3000-4000

# Filter by process name
kilar list -f node

# Sort by different criteria
kilar list -s pid    # Sort by PID
kilar list -s name   # Sort by process name
kilar list -s port   # Sort by port number (default)

# Interactive kill mode
kilar list          # Select processes to kill interactively
kilar list --view-only  # Just view, no kill option
```

## Command Options 🎛️

### Global Options
- `-q, --quiet`: Suppress output
- `-j, --json`: Output in JSON format
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Check Command
```bash
kilar check <PORT> [OPTIONS]
```
- `PORT`: Port number to check
- `-p, --protocol <PROTOCOL>`: Protocol (tcp/udp) [default: tcp]

### Kill Command
```bash
kilar kill <PORT> [OPTIONS]
```
- `PORT`: Port number of the process to kill
- `-f, --force`: Force kill without confirmation
- `-p, --protocol <PROTOCOL>`: Protocol (tcp/udp) [default: tcp]

### List Command
```bash
kilar list [OPTIONS]
```
- `-r, --ports <RANGE>`: Port range to filter (e.g., 3000-4000)
- `-f, --filter <NAME>`: Filter by process name
- `-s, --sort <ORDER>`: Sort order (port/pid/name) [default: port]
- `-p, --protocol <PROTOCOL>`: Protocol (tcp/udp/all) [default: tcp]
- `--view-only`: View only (no kill feature)

## Examples 📝

### Development Workflow

```bash
# Check if your development server port is free
kilar check 3000

# If occupied, see what's using it
kilar check 3000 -v

# Kill the process if needed
kilar kill 3000

# List all development-related ports
kilar list -r 3000-9000 -f node
```

### System Administration

```bash
# List all services
kilar list -p all

# Find specific service
kilar list -f nginx

# Check system ports
kilar list -r 1-1024

# Export port usage as JSON
kilar list --json > ports.json
```

## Output Format 🎨

### Standard Output
- ✓ Green checkmark: Success/Port in use
- × Red cross: Error/Failed operation
- ○ Blue circle: Information/Port available
- Yellow: Port numbers and process names
- Cyan: PIDs and labels
- Blue: Protocol information

### JSON Output
All commands support JSON output for scripting and automation:

```json
{
  "port": 3000,
  "protocol": "tcp",
  "status": "occupied",
  "process": {
    "pid": 12345,
    "name": "node",
    "command": "node server.js"
  }
}
```

## Requirements 📋

- **macOS/Linux**: `lsof` command (usually pre-installed)
- **Windows**: `netstat` command (pre-installed)
- **Permissions**: Some operations may require sudo/administrator privileges

## Building from Source 🔨

```bash
# Clone the repository
git clone https://github.com/polidog/kilar.git
cd kilar

# Build in release mode
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

### CI/CD Status

[![CI](https://github.com/polidog/kilar/actions/workflows/ci.yml/badge.svg)](https://github.com/polidog/kilar/actions/workflows/ci.yml)

**Note**: Windows CI tests are currently disabled due to environment instability. The project fully supports Windows platform, but automated testing is temporarily limited to macOS and Linux environments. Windows builds are still generated and released.

## License 📄

This project is licensed under the MIT License.

## Author 👤

**polidog**

- GitHub: [@polidog](https://github.com/polidog)

## Acknowledgments 🙏

- Built with Rust 🦀
- Uses `tokio` for async operations
- Terminal UI powered by `dialoguer` and `colored`

---

**Note**: This tool requires appropriate permissions to view and terminate processes. Some system processes may require elevated privileges (sudo/administrator).