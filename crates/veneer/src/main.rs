//! Veneer CLI - Rust-powered component documentation generator.

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

mod commands;

#[derive(Parser)]
#[command(name = "veneer")]
#[command(about = "Rust-powered component documentation generator")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract documentation from a target project
    Extract(commands::extract::ExtractArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    fmt().with_env_filter(filter).with_target(false).init();

    match cli.command {
        Commands::Extract(args) => {
            commands::extract::run(args).await?;
        }
    }

    Ok(())
}
