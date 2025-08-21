use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kilar",
    about = "ポートプロセス管理CLIツール",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, help = "詳細出力を有効にする")]
    pub verbose: bool,

    #[arg(short, long, help = "出力を抑制する")]
    pub quiet: bool,

    #[arg(short, long, help = "JSON形式で出力する")]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "指定ポートの使用状況を確認する")]
    Check {
        #[arg(help = "確認するポート番号")]
        port: u16,
        
        #[arg(short, long, default_value = "tcp", help = "プロトコル (tcp/udp)")]
        protocol: String,
    },
    
    #[command(about = "指定ポートを使用しているプロセスを終了する")]
    Kill {
        #[arg(help = "終了するプロセスが使用しているポート番号")]
        port: u16,
        
        #[arg(short, long, help = "確認なしで強制終了する")]
        force: bool,
        
        #[arg(short, long, default_value = "tcp", help = "プロトコル (tcp/udp)")]
        protocol: String,
    },
    
    #[command(about = "使用中のポート一覧を表示する")]
    List {
        #[arg(short = 'r', long, help = "フィルタリングするポート範囲 (例: 3000-4000)")]
        ports: Option<String>,
        
        #[arg(short, long, help = "プロセス名でフィルタリング")]
        filter: Option<String>,
        
        #[arg(short, long, default_value = "port", help = "ソート順 (port/pid/name)")]
        sort: String,
        
        #[arg(short, long, default_value = "tcp", help = "プロトコル (tcp/udp/all)")]
        protocol: String,
        
        #[arg(short, long, help = "対話的にプロセスを選択して終了する")]
        kill: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}