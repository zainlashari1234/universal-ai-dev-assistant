mod commands;
mod config;
mod ui;
mod client;

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "uaida")]
#[command(about = "Universal AI Development Assistant CLI")]
#[command(version = "6.2.0")]
#[command(author = "Universal AI Team")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Server URL
    #[arg(short, long, default_value = "http://localhost:8080")]
    server: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Output format (json, yaml, table)
    #[arg(short, long, default_value = "table")]
    output: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize UAIDA configuration
    Init {
        /// Force overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
    
    /// Start interactive development environment
    Dev {
        /// Project directory
        #[arg(short, long, default_value = ".")]
        project: PathBuf,
        
        /// Language hint
        #[arg(short, long)]
        language: Option<String>,
    },
    
    /// Code completion
    Complete {
        /// Code prompt
        prompt: String,
        
        /// Programming language
        #[arg(short, long)]
        language: Option<String>,
        
        /// AI model to use
        #[arg(short, long)]
        model: Option<String>,
        
        /// AI provider to use
        #[arg(short, long)]
        provider: Option<String>,
        
        /// Max tokens
        #[arg(long, default_value = "1000")]
        max_tokens: u32,
        
        /// Temperature (0.0-1.0)
        #[arg(long, default_value = "0.7")]
        temperature: f32,
    },
    
    /// Analyze code
    Analyze {
        /// File to analyze
        file: PathBuf,
        
        /// Analysis type (security, performance, quality, bugs)
        #[arg(short, long, default_value = "quality")]
        analysis_type: String,
        
        /// Programming language (auto-detect if not specified)
        #[arg(short, long)]
        language: Option<String>,
    },
    
    /// Generate documentation
    Doc {
        /// File to document
        file: PathBuf,
        
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Documentation format (markdown, rst, html)
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },
    
    /// Generate tests
    Test {
        /// File to generate tests for
        file: PathBuf,
        
        /// Output file (auto-generate if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Test framework
        #[arg(short, long)]
        framework: Option<String>,
    },
    
    /// Explain code
    Explain {
        /// File to explain
        file: PathBuf,
        
        /// Specific function/class to explain
        #[arg(short, long)]
        symbol: Option<String>,
    },
    
    /// Refactor code
    Refactor {
        /// File to refactor
        file: PathBuf,
        
        /// Refactoring instructions
        instructions: String,
        
        /// Output file (overwrite original if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Create backup
        #[arg(short, long)]
        backup: bool,
    },
    
    /// Translate code between languages
    Translate {
        /// File to translate
        file: PathBuf,
        
        /// Target language
        #[arg(short, long)]
        target: String,
        
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Interactive terminal mode
    Terminal,
    
    /// Search code patterns
    Search {
        /// Search query
        query: String,
        
        /// Directory to search
        #[arg(short, long, default_value = ".")]
        directory: PathBuf,
        
        /// File extensions to include
        #[arg(short, long)]
        extensions: Option<Vec<String>>,
    },
    
    /// Fix code issues
    Fix {
        /// File to fix
        file: PathBuf,
        
        /// Issue type (auto, security, performance, style)
        #[arg(short, long, default_value = "auto")]
        issue_type: String,
        
        /// Apply fixes automatically
        #[arg(short, long)]
        auto_apply: bool,
    },
    
    /// Interactive chat mode
    Chat {
        /// Initial message
        message: Option<String>,
        
        /// Chat mode (code, general, debug)
        #[arg(short, long, default_value = "code")]
        mode: String,
    },
    
    /// Manage AI providers
    Providers {
        /// Provider action (list, add, remove, test)
        #[arg(short, long, default_value = "list")]
        action: String,
        
        /// Provider name
        #[arg(short, long)]
        name: Option<String>,
        
        /// API key
        #[arg(short, long)]
        key: Option<String>,
    },
    
    /// Show system status
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
        
        /// Check provider health
        #[arg(short, long)]
        health: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // Initialize configuration
    let config = config::Config::load(cli.config.as_deref())?;
    
    // Initialize client
    let client = client::Client::new(&cli.server, &config)?;
    
    // Print banner for non-interactive commands
    if !matches!(cli.command, Commands::Chat { .. } | Commands::Terminal) {
        print_banner();
    }
    
    // Execute command
    match cli.command {
        Commands::Init { force } => {
            commands::init::run(force).await?;
        }
        
        Commands::Dev { project, language } => {
            commands::dev::run(project, language, &client).await?;
        }
        
        Commands::Complete { 
            prompt, 
            language, 
            model, 
            provider, 
            max_tokens, 
            temperature 
        } => {
            commands::complete::run(
                prompt, 
                language, 
                model, 
                provider, 
                max_tokens, 
                temperature, 
                &client
            ).await?;
        }
        
        Commands::Analyze { file, analysis_type, language } => {
            commands::analyze::run(file, analysis_type, language, &client).await?;
        }
        
        Commands::Doc { file, output, format } => {
            commands::doc::run(file, output, format, &client).await?;
        }
        
        Commands::Test { file, output, framework } => {
            commands::test::run(file, output, framework, &client).await?;
        }
        
        Commands::Explain { file, symbol } => {
            commands::explain::run(file, symbol, &client).await?;
        }
        
        Commands::Refactor { file, instructions, output, backup } => {
            commands::refactor::run(file, instructions, output, backup, &client).await?;
        }
        
        Commands::Translate { file, target, output } => {
            commands::translate::run(file, target, output, &client).await?;
        }
        
        Commands::Terminal => {
            commands::terminal::run(&client).await?;
        }
        
        Commands::Search { query, directory, extensions } => {
            commands::search::run(query, directory, extensions, &client).await?;
        }
        
        Commands::Fix { file, issue_type, auto_apply } => {
            commands::fix::run(file, issue_type, auto_apply, &client).await?;
        }
        
        Commands::Chat { message, mode } => {
            commands::chat::run(message, mode, &client).await?;
        }
        
        Commands::Providers { action, name, key } => {
            commands::providers::run(action, name, key, &client).await?;
        }
        
        Commands::Status { detailed, health } => {
            commands::status::run(detailed, health, &client).await?;
        }
    }
    
    Ok(())
}

fn print_banner() {
    println!("{}", "🚀 Universal AI Development Assistant".bright_blue().bold());
    println!("{}", "   Your AI-powered coding companion".bright_white());
    println!();
}