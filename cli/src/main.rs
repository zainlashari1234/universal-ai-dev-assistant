use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::*;
use std::path::PathBuf;

mod commands;
mod config;
mod client;

use commands::*;
use config::CliConfig;

#[derive(Parser)]
#[command(name = "uaida")]
#[command(about = "Universal AI Development Assistant CLI")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    
    #[arg(long, global = true)]
    server: Option<String>,
    
    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize UAIDA in current directory
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Get AI code completion
    Complete {
        /// File to analyze
        file: PathBuf,
        /// Line number (1-based)
        #[arg(short, long)]
        line: Option<usize>,
        /// Column number (1-based)
        #[arg(short, long)]
        column: Option<usize>,
    },
    /// Analyze code for issues and improvements
    Analyze {
        /// File or directory to analyze
        path: PathBuf,
        /// Output format (text, json, sarif)
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Include security scan
        #[arg(long)]
        security: bool,
    },
    /// Generate documentation
    Docs {
        /// File or directory to document
        path: PathBuf,
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Documentation format (markdown, html)
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },
    /// Generate tests
    Test {
        /// File to generate tests for
        file: PathBuf,
        /// Test framework (pytest, jest, etc.)
        #[arg(short, long)]
        framework: Option<String>,
        /// Output directory
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Refactor code
    Refactor {
        /// File to refactor
        file: PathBuf,
        /// Refactoring type
        #[arg(short, long)]
        refactor_type: Option<String>,
        /// Apply changes automatically
        #[arg(long)]
        apply: bool,
    },
    /// Start local AI server
    Server {
        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Run in background
        #[arg(short, long)]
        daemon: bool,
    },
    /// Check server status
    Status,
    /// Configure UAIDA settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        key: String,
        value: String,
    },
    /// Get configuration value
    Get {
        key: String,
    },
    /// Reset to default configuration
    Reset,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    if cli.verbose {
        env_logger::init();
    }

    // Load configuration
    let config = CliConfig::load(cli.config.as_deref())?;
    
    // Override server URL if provided
    let server_url = cli.server.unwrap_or(config.server_url.clone());
    
    match cli.command {
        Commands::Init { force } => {
            init_command(force).await
        }
        Commands::Complete { file, line, column } => {
            complete_command(&server_url, file, line, column).await
        }
        Commands::Analyze { path, format, security } => {
            analyze_command(&server_url, path, &format, security).await
        }
        Commands::Docs { path, output, format } => {
            docs_command(&server_url, path, output, &format).await
        }
        Commands::Test { file, framework, output } => {
            test_command(&server_url, file, framework, output).await
        }
        Commands::Refactor { file, refactor_type, apply } => {
            refactor_command(&server_url, file, refactor_type, apply).await
        }
        Commands::Server { port, host, daemon } => {
            server_command(port, &host, daemon).await
        }
        Commands::Status => {
            status_command(&server_url).await
        }
        Commands::Config { action } => {
            config_command(action, &config).await
        }
    }
}

fn print_banner() {
    println!("{}", "
    ██╗   ██╗ █████╗ ██╗██████╗  █████╗ 
    ██║   ██║██╔══██╗██║██╔══██╗██╔══██╗
    ██║   ██║███████║██║██║  ██║███████║
    ██║   ██║██╔══██║██║██║  ██║██╔══██║
    ╚██████╔╝██║  ██║██║██████╔╝██║  ██║
     ╚═════╝ ╚═╝  ╚═╝╚═╝╚═════╝ ╚═╝  ╚═╝
    ".bright_blue());
    
    println!("{}", "Universal AI Development Assistant".bright_green());
    println!("{}", format!("Version {}", env!("CARGO_PKG_VERSION")).dimmed());
    println!();
}