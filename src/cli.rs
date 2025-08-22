use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kilar",
    about = "Port process management CLI tool",
    version,
    author,
    args_conflicts_with_subcommands = true,
    subcommand_help_heading = "Commands",
    help_template = "{before-help}{name} {version}\n{author-with-newline}{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Suppress output")]
    pub quiet: bool,

    #[arg(short, long, global = true, help = "Output in JSON format")]
    pub json: bool,

    #[arg(short = 'v', long, global = true, help = "Enable verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Check port usage status")]
    Check {
        #[arg(help = "Port number to check")]
        port: u16,

        #[arg(short, long, default_value = "tcp", help = "Protocol (tcp/udp)")]
        protocol: String,
    },

    #[command(about = "Kill process using specified port")]
    Kill {
        #[arg(help = "Port number used by the process to kill")]
        port: u16,

        #[arg(short, long, help = "Force kill without confirmation")]
        force: bool,

        #[arg(short, long, default_value = "tcp", help = "Protocol (tcp/udp)")]
        protocol: String,
    },

    #[command(about = "List ports in use")]
    List {
        #[arg(short = 'r', long, help = "Port range to filter (e.g., 3000-4000)")]
        ports: Option<String>,

        #[arg(short, long, help = "Filter by process name")]
        filter: Option<String>,

        #[arg(
            short,
            long,
            default_value = "port",
            help = "Sort order (port/pid/name)"
        )]
        sort: String,

        #[arg(short, long, default_value = "tcp", help = "Protocol (tcp/udp/all)")]
        protocol: String,

        #[arg(long, help = "View only (no kill feature)")]
        view_only: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
