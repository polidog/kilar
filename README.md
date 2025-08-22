# kilar 🔌

[![CI](https://github.com/polidog/kilar/actions/workflows/ci.yml/badge.svg)](https://github.com/polidog/kilar/actions/workflows/ci.yml)
[![Release](https://github.com/polidog/kilar/actions/workflows/release.yml/badge.svg)](https://github.com/polidog/kilar/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/kilar.svg)](https://crates.io/crates/kilar)
[![Downloads](https://img.shields.io/crates/d/kilar.svg)](https://crates.io/crates/kilar)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

**A powerful CLI tool for managing port processes on your system.** Quickly find and terminate processes using specific ports with an intuitive interface.

## 📋 Table of Contents

- [🚀 Quick Start](#-quick-start)
- [✨ Key Features](#-key-features)
- [📦 Installation](#-installation)
- [🚀 Usage](#-usage)
- [🎛️ Command Options](#️-command-options)
- [📝 Examples](#-examples)
- [🎨 Output Format](#-output-format)
- [📋 Requirements](#-requirements)
- [🔨 Building from Source](#-building-from-source)
- [🤝 Contributing](#-contributing)
- [🔐 Security](#-security)
- [📈 Project Status](#-project-status)
- [📊 Performance & Compatibility](#-performance--compatibility)
- [🗺️ Roadmap](#️-roadmap)
- [📄 License](#-license)

## 🚀 Quick Start

```bash
# Install via Homebrew (macOS/Linux)
brew tap polidog/kilar && brew install kilar

# Install via Cargo
cargo install kilar

# Quick usage
kilar check 3000        # Check if port 3000 is in use
kilar kill 3000         # Kill process on port 3000
kilar list              # List all ports in use
```

## ✨ Key Features

### 🔍 **Smart Port Detection**
- **Lightning-fast port checking** - Instantly see if a port is in use and which process owns it
- **Protocol support** - Works with both TCP and UDP protocols
- **Detailed process information** - View PID, process name, and command line

### ⚡ **Safe Process Management** 
- **Graceful termination** - Kill processes with built-in safety confirmations
- **Force kill option** - Override confirmations when needed
- **Interactive selection** - Choose multiple processes to terminate with an intuitive UI

### 📊 **Comprehensive Listing**
- **Port range filtering** - View specific port ranges (e.g., 3000-4000)
- **Process name filtering** - Find ports by application name
- **Flexible sorting** - Sort by port, PID, or process name

### 🎨 **Developer-Friendly Output**
- **Color-coded terminal output** - Easy-to-read with intuitive color schemes
- **JSON export** - Perfect for scripting and automation
- **Verbose mode** - Get detailed information when troubleshooting

### 🌍 **Cross-Platform & Modern**
- **Universal compatibility** - Works on macOS and Linux
- **Multiple installation methods** - Homebrew, Cargo, or from source
- **Zero dependencies** - Single binary with no runtime requirements

## 📦 Installation

### 🍺 Homebrew (Recommended for macOS/Linux)

```bash
# Add tap and install
brew tap polidog/kilar
brew install kilar

# Or one-liner
brew install polidog/kilar/kilar
```

### 📦 Cargo (Universal)

```bash
# Install from crates.io
cargo install kilar

# Install from source
cargo install --git https://github.com/polidog/kilar.git
```

### 📥 Binary Downloads

Download pre-built binaries from the [releases page](https://github.com/polidog/kilar/releases):

- **macOS** (Intel): `kilar-x86_64-apple-darwin.tar.gz`
- **macOS** (Apple Silicon): `kilar-aarch64-apple-darwin.tar.gz`
- **Linux** (x86_64): `kilar-x86_64-unknown-linux-gnu.tar.gz`
- **Linux** (ARM64): `kilar-aarch64-unknown-linux-gnu.tar.gz`

### 🔨 From Source

```bash
git clone https://github.com/polidog/kilar.git
cd kilar
cargo build --release
sudo cp target/release/kilar /usr/local/bin/
```

> **Note**: Requires [Rust](https://rustup.rs/) 1.70 or later

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
- **Permissions**: Some operations may require sudo privileges

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

## 🤝 Contributing

We welcome contributions! Here's how you can help:

- 🐛 [Report bugs](https://github.com/polidog/kilar/issues/new?labels=bug)
- 💡 [Request features](https://github.com/polidog/kilar/issues/new?labels=enhancement)
- 📖 Improve documentation
- 🔧 Submit pull requests

See our [Contributing Guide](CONTRIBUTING.md) for detailed instructions.

## 🔐 Security

`kilar` handles system processes and requires appropriate permissions:

- **Process visibility**: Requires read access to system process information
- **Process termination**: May require elevated privileges (sudo) for some processes
- **Network data**: Accesses network connection information through system commands

For security issues, please see our [Security Policy](https://github.com/polidog/kilar/security).

## 📈 Project Status

### 🏗️ Development Status
- **Stable**: Core functionality is production-ready with v0.1.1 released
- **Active**: Regular updates and maintenance
- **Cross-platform**: Tested on macOS, Linux, and Windows
- **Package Distribution**: Available via Homebrew, Cargo, and GitHub Releases

### 🚀 CI/CD Status
[![CI](https://github.com/polidog/kilar/actions/workflows/ci.yml/badge.svg)](https://github.com/polidog/kilar/actions/workflows/ci.yml)

> **Note**: This tool is designed for Unix-like systems (macOS and Linux) and provides comprehensive port management functionality on these platforms.

## 📊 Performance & Compatibility

| Platform | Min Version | Status | Notes |
|----------|-------------|--------|--------|
| **macOS** | 10.15+ | ✅ Full Support | Intel & Apple Silicon |
| **Linux** | Any modern | ✅ Full Support | Requires `lsof` |

## 🗺️ Roadmap

### ✅ Completed (v0.1.x)
- [x] **v0.1.0**: Core port management functionality
- [x] **v0.1.1**: Improved Homebrew distribution and bug fixes

### 🔮 Future Releases
- [ ] **v0.2.0**: Configuration file support
- [ ] **v0.3.0**: Plugin system for custom output formats
- [ ] **v0.4.0**: Network interface filtering
- [ ] **v1.0.0**: Stable API and comprehensive documentation

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

## 👥 Authors & Contributors

**🚀 Maintainer**: [polidog](https://github.com/polidog)

Thanks to all [contributors](https://github.com/polidog/kilar/contributors) who help make this project better!

## 🙏 Acknowledgments

- **🦀 Built with Rust** - For memory safety and performance
- **⚡ Tokio** - Async runtime for efficient I/O operations  
- **🎨 Terminal UI** - Powered by `dialoguer` and `colored`
- **🏗️ Cross-compilation** - Thanks to GitHub Actions and `cross`

## 🔗 Related Projects & Resources

### 🛠️ System Tools Used
- [lsof](https://github.com/lsof-org/lsof) - List open files (Unix)
- [netstat](https://docs.microsoft.com/en-us/windows-server/administration/windows-commands/netstat) - Network statistics (Windows)
- [ss](https://man7.org/linux/man-pages/man8/ss.8.html) - Socket statistics (Linux)

### 📦 Distribution Channels
- [Homebrew Tap](https://github.com/polidog/homebrew-kilar) - Official Homebrew formula
- [Crates.io](https://crates.io/crates/kilar) - Rust package registry
- [GitHub Releases](https://github.com/polidog/kilar/releases) - Binary downloads

---

> **⚠️ Important**: This tool requires appropriate permissions to view and terminate processes. Some system processes may require elevated privileges (sudo).