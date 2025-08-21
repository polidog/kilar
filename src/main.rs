use colored::Colorize;
use kilar::{
    cli::{Cli, Commands},
    commands::{CheckCommand, KillCommand, ListCommand},
    utils::{validate_port, validate_protocol, validate_sort_option},
    Result,
};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{} {}", "エラー:".red(), e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse_args();

    match cli.command {
        Commands::Check { port, protocol } => {
            validate_port(port)?;
            validate_protocol(&protocol)?;
            
            CheckCommand::execute(port, &protocol, cli.verbose, cli.quiet, cli.json).await?;
        }
        Commands::Kill { port, force, protocol } => {
            validate_port(port)?;
            validate_protocol(&protocol)?;
            
            KillCommand::execute(port, &protocol, force, cli.verbose, cli.quiet, cli.json).await?;
        }
        Commands::List { ports, filter, sort, protocol, kill } => {
            validate_protocol(&protocol)?;
            validate_sort_option(&sort)?;
            
            ListCommand::execute(ports, filter, &sort, &protocol, kill, cli.verbose, cli.quiet, cli.json).await?;
        }
    }

    Ok(())
}
