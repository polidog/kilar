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

        #[arg(short, long, help = "Enable interactive mode with kill option")]
        interactive: bool,
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

        #[arg(long, help = "Watch mode - continuously monitor port changes")]
        watch: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_structure() {
        // CLI構造体の基本構造をテスト
        let cli =
            Cli::try_parse_from(["kilar", "check", "3000"]).expect("Failed to parse check command");

        assert!(!cli.quiet);
        assert!(!cli.json);
        assert!(!cli.verbose);

        match cli.command {
            Commands::Check {
                port,
                protocol,
                interactive,
            } => {
                assert_eq!(port, 3000);
                assert_eq!(protocol, "tcp");
                assert!(!interactive);
            }
            _ => panic!("Expected Check command"),
        }
    }

    #[test]
    fn test_check_command_parsing() {
        // Check コマンドのパースをテスト
        let test_cases = vec![
            (vec!["kilar", "check", "8080"], 8080, "tcp", false),
            (
                vec!["kilar", "check", "3000", "--protocol", "udp"],
                3000,
                "udp",
                false,
            ),
            (
                vec!["kilar", "check", "5000", "--interactive"],
                5000,
                "tcp",
                true,
            ),
            (
                vec!["kilar", "check", "9000", "-p", "tcp", "-i"],
                9000,
                "tcp",
                true,
            ),
        ];

        for (args, expected_port, expected_protocol, expected_interactive) in test_cases {
            let cli = Cli::try_parse_from(&args)
                .unwrap_or_else(|_| panic!("Failed to parse: {:?}", args));

            match cli.command {
                Commands::Check {
                    port,
                    protocol,
                    interactive,
                } => {
                    assert_eq!(port, expected_port, "Port mismatch for args: {:?}", args);
                    assert_eq!(
                        protocol, expected_protocol,
                        "Protocol mismatch for args: {:?}",
                        args
                    );
                    assert_eq!(
                        interactive, expected_interactive,
                        "Interactive mismatch for args: {:?}",
                        args
                    );
                }
                _ => panic!("Expected Check command for args: {:?}", args),
            }
        }
    }

    #[test]
    fn test_kill_command_parsing() {
        // Kill コマンドのパースをテスト
        let test_cases = vec![
            (vec!["kilar", "kill", "8080"], 8080, "tcp", false),
            (
                vec!["kilar", "kill", "3000", "--protocol", "udp"],
                3000,
                "udp",
                false,
            ),
            (vec!["kilar", "kill", "5000", "--force"], 5000, "tcp", true),
            (
                vec!["kilar", "kill", "9000", "-p", "tcp", "-f"],
                9000,
                "tcp",
                true,
            ),
        ];

        for (args, expected_port, expected_protocol, expected_force) in test_cases {
            let cli = Cli::try_parse_from(&args)
                .unwrap_or_else(|_| panic!("Failed to parse: {:?}", args));

            match cli.command {
                Commands::Kill {
                    port,
                    protocol,
                    force,
                } => {
                    assert_eq!(port, expected_port, "Port mismatch for args: {:?}", args);
                    assert_eq!(
                        protocol, expected_protocol,
                        "Protocol mismatch for args: {:?}",
                        args
                    );
                    assert_eq!(force, expected_force, "Force mismatch for args: {:?}", args);
                }
                _ => panic!("Expected Kill command for args: {:?}", args),
            }
        }
    }

    #[test]
    fn test_list_command_parsing() {
        // List コマンドのパースをテスト
        let cli = Cli::try_parse_from(["kilar", "list"]).expect("Failed to parse list command");

        match cli.command {
            Commands::List {
                ports,
                filter,
                sort,
                protocol,
                view_only,
                watch,
            } => {
                assert_eq!(ports, None);
                assert_eq!(filter, None);
                assert_eq!(sort, "port");
                assert_eq!(protocol, "tcp");
                assert!(!view_only);
                assert!(!watch);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_list_command_with_options() {
        let cli = Cli::try_parse_from([
            "kilar",
            "list",
            "--ports",
            "3000-4000",
            "--filter",
            "node",
            "--sort",
            "pid",
            "--protocol",
            "udp",
            "--view-only",
            "--watch",
        ])
        .expect("Failed to parse list command with options");

        match cli.command {
            Commands::List {
                ports,
                filter,
                sort,
                protocol,
                view_only,
                watch,
            } => {
                assert_eq!(ports, Some("3000-4000".to_string()));
                assert_eq!(filter, Some("node".to_string()));
                assert_eq!(sort, "pid");
                assert_eq!(protocol, "udp");
                assert!(view_only);
                assert!(watch);
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_global_flags() {
        // グローバルフラグのテスト
        let test_cases = vec![
            (
                vec!["kilar", "check", "3000", "--quiet"],
                true,
                false,
                false,
            ),
            (vec!["kilar", "check", "3000", "--json"], false, true, false),
            (
                vec!["kilar", "check", "3000", "--verbose"],
                false,
                false,
                true,
            ),
            (vec!["kilar", "check", "3000", "-q"], true, false, false),
            (vec!["kilar", "check", "3000", "-j"], false, true, false),
            (vec!["kilar", "check", "3000", "-v"], false, false, true),
            (
                vec!["kilar", "check", "3000", "--quiet", "--json", "--verbose"],
                true,
                true,
                true,
            ),
        ];

        for (args, expected_quiet, expected_json, expected_verbose) in test_cases {
            let cli = Cli::try_parse_from(&args).unwrap_or_else(|_| panic!("Failed to parse: {:?}", args));

            assert_eq!(
                cli.quiet, expected_quiet,
                "Quiet flag mismatch for args: {:?}",
                args
            );
            assert_eq!(
                cli.json, expected_json,
                "JSON flag mismatch for args: {:?}",
                args
            );
            assert_eq!(
                cli.verbose, expected_verbose,
                "Verbose flag mismatch for args: {:?}",
                args
            );
        }
    }

    #[test]
    fn test_port_range_validation() {
        // 有効なポート番号の範囲をテスト
        let valid_ports = [1, 80, 443, 3000, 8080, 65535];

        for port in valid_ports {
            let port_str = port.to_string();
            let args = vec!["kilar", "check", &port_str];
            let result = Cli::try_parse_from(&args);
            assert!(result.is_ok(), "Port {} should be valid", port);

            if let Ok(cli) = result {
                match cli.command {
                    Commands::Check {
                        port: parsed_port, ..
                    } => {
                        assert_eq!(parsed_port, port);
                    }
                    _ => panic!("Expected Check command"),
                }
            }
        }
    }

    #[test]
    fn test_invalid_port_numbers() {
        // 無効なポート番号のテスト（u16の範囲外や文字列）
        let invalid_ports = ["65536", "-1", "abc", ""];

        for invalid_port in invalid_ports {
            let args = vec!["kilar", "check", invalid_port];
            let result = Cli::try_parse_from(&args);
            assert!(result.is_err(), "Port '{}' should be invalid", invalid_port);
        }
    }

    #[test]
    fn test_protocol_values() {
        // プロトコル値のテスト（バリデーションは後で行われるので、文字列として受け入れられる）
        let protocols = ["tcp", "udp", "all", "invalid"];

        for protocol in protocols {
            let args = vec!["kilar", "check", "3000", "--protocol", protocol];
            let cli = Cli::try_parse_from(&args)
                .unwrap_or_else(|_| panic!("Failed to parse protocol: {}", protocol));

            match cli.command {
                Commands::Check {
                    protocol: parsed_protocol,
                    ..
                } => {
                    assert_eq!(parsed_protocol, protocol);
                }
                _ => panic!("Expected Check command"),
            }
        }
    }

    #[test]
    fn test_sort_values() {
        // ソートオプションのテスト（バリデーションは後で行われるので、文字列として受け入れられる）
        let sorts = ["port", "pid", "name", "invalid"];

        for sort in sorts {
            let args = vec!["kilar", "list", "--sort", sort];
            let cli = Cli::try_parse_from(&args)
                .unwrap_or_else(|_| panic!("Failed to parse sort: {}", sort));

            match cli.command {
                Commands::List {
                    sort: parsed_sort, ..
                } => {
                    assert_eq!(parsed_sort, sort);
                }
                _ => panic!("Expected List command"),
            }
        }
    }

    #[test]
    fn test_help_output() {
        // ヘルプ出力が正常に生成されることを確認
        let mut cmd = Cli::command();
        let help = cmd.render_help();
        let help_str = help.to_string();

        // 基本的な内容が含まれていることを確認
        assert!(help_str.contains("kilar"));
        assert!(help_str.contains("Port process management CLI tool"));
        assert!(help_str.contains("check"));
        assert!(help_str.contains("kill"));
        assert!(help_str.contains("list"));
    }

    #[test]
    fn test_version_output() {
        // バージョン情報のテスト
        let cmd = Cli::command();
        let version = cmd.get_version();

        // バージョンが設定されていることを確認
        assert!(version.is_some());
    }

    #[test]
    fn test_missing_required_arguments() {
        // 必須引数が不足している場合のテスト
        let invalid_args = vec![
            vec!["kilar", "check"], // ポート番号が不足
            vec!["kilar", "kill"],  // ポート番号が不足
            vec!["kilar"],          // サブコマンドが不足
        ];

        for args in invalid_args {
            let result = Cli::try_parse_from(&args);
            assert!(result.is_err(), "Args {:?} should be invalid", args);
        }
    }

    #[test]
    fn test_command_aliases() {
        // 短縮オプションのテスト
        let cli = Cli::try_parse_from(["kilar", "check", "3000", "-p", "udp", "-i"])
            .expect("Failed to parse with short options");

        match cli.command {
            Commands::Check {
                port,
                protocol,
                interactive,
            } => {
                assert_eq!(port, 3000);
                assert_eq!(protocol, "udp");
                assert!(interactive);
            }
            _ => panic!("Expected Check command"),
        }
    }

    #[test]
    fn test_default_values() {
        // デフォルト値のテスト
        let cli = Cli::try_parse_from(["kilar", "check", "3000"]).expect("Failed to parse");

        match cli.command {
            Commands::Check {
                protocol,
                interactive,
                ..
            } => {
                assert_eq!(protocol, "tcp"); // デフォルトプロトコル
                assert!(!interactive); // デフォルトでインタラクティブでない
            }
            _ => panic!("Expected Check command"),
        }

        let cli = Cli::try_parse_from(["kilar", "list"]).expect("Failed to parse");

        match cli.command {
            Commands::List { sort, protocol, .. } => {
                assert_eq!(sort, "port"); // デフォルトソート
                assert_eq!(protocol, "tcp"); // デフォルトプロトコル
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_cli_parse_args_method() {
        // parse_args メソッドのテスト（実際のコマンドライン引数をテストできないため、構造テスト）
        // この関数は実際のstd::env::args()を使用するため、単体テストでは直接テストできない
        // しかし、メソッドが存在することは確認できる

        // メソッドが存在し、正しいシグネチャを持つことを確認
        fn _test_parse_args_signature(_: fn() -> Cli) {}
        _test_parse_args_signature(Cli::parse_args);
    }
}
