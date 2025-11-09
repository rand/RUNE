//! RUNE CLI - Command-line interface for RUNE

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use rune_core::{Action, Principal, RUNEEngine, Request, RequestBuilder, Resource};
use std::fs;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "rune")]
#[command(about = "RUNE - High-performance authorization and configuration engine")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Evaluate an authorization request
    Eval {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,

        /// Action to evaluate
        #[arg(long)]
        action: String,

        /// Principal ID
        #[arg(long, default_value = "agent-1")]
        principal: String,

        /// Resource path or ID
        #[arg(long)]
        resource: String,

        /// Output format (json, text)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Validate a RUNE configuration file
    Validate {
        /// Configuration file path
        file: String,
    },

    /// Run benchmark tests
    Benchmark {
        /// Number of requests to generate
        #[arg(short, long, default_value = "10000")]
        requests: usize,

        /// Number of parallel threads
        #[arg(short, long, default_value = "8")]
        threads: usize,
    },

    /// Start RUNE server
    Serve {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("rune=debug")
            .init();
    }

    match cli.command {
        Commands::Eval {
            config,
            action,
            principal,
            resource,
            format,
        } => {
            eval_command(config, action, principal, resource, format).await?;
        }
        Commands::Validate { file } => {
            validate_command(file).await?;
        }
        Commands::Benchmark { requests, threads } => {
            benchmark_command(requests, threads).await?;
        }
        Commands::Serve { config, port } => {
            serve_command(config, port).await?;
        }
    }

    Ok(())
}

async fn eval_command(
    config: Option<String>,
    action: String,
    principal: String,
    resource: String,
    format: String,
) -> Result<()> {
    let start = Instant::now();

    // Create engine
    let engine = RUNEEngine::new();

    // Load configuration if provided
    if let Some(config_path) = config {
        println!(
            "{} Loading configuration from {}...",
            "→".blue(),
            config_path
        );
        // TODO: Implement configuration loading
        // engine.load_configuration(&config_path)?;
    }

    // Build request
    let request = RequestBuilder::new()
        .principal(Principal::agent(principal.clone()))
        .action(Action::new(action.clone()))
        .resource(Resource::file(resource.clone()))
        .build()?;

    // Evaluate
    println!("{} Evaluating request...", "→".blue());
    let result = engine.authorize(&request)?;

    // Output result
    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            let status = if result.decision.is_permitted() {
                "PERMITTED".green()
            } else {
                "DENIED".red()
            };

            println!("\n{} Authorization Result", "═".blue().bold());
            println!("{} Status: {}", "▸".blue(), status);
            println!("{} Action: {}", "▸".blue(), action);
            println!("{} Principal: {}", "▸".blue(), principal);
            println!("{} Resource: {}", "▸".blue(), resource);
            println!("{} Explanation: {}", "▸".blue(), result.explanation);
            println!(
                "{} Evaluation time: {:.3}ms",
                "▸".blue(),
                result.evaluation_time_ns as f64 / 1_000_000.0
            );

            if result.cached {
                println!("{} Result was cached", "▸".blue());
            }

            if !result.evaluated_rules.is_empty() {
                println!("{} Evaluated rules:", "▸".blue());
                for rule in &result.evaluated_rules {
                    println!("  {}", rule);
                }
            }
        }
    }

    let total_time = start.elapsed();
    println!(
        "\n{} Total time: {:.3}ms",
        "✓".green(),
        total_time.as_secs_f64() * 1000.0
    );

    Ok(())
}

async fn validate_command(file: String) -> Result<()> {
    println!("{} Validating {}...", "→".blue(), file);

    let contents =
        fs::read_to_string(&file).with_context(|| format!("Failed to read file: {}", file))?;

    match rune_core::parse_rune_file(&contents) {
        Ok(config) => {
            println!("{} Configuration is valid!", "✓".green());
            println!("  Version: {}", config.version);
            println!("  Rules: {}", config.rules.len());
            println!("  Policies: {}", config.policies.len());
        }
        Err(e) => {
            println!("{} Configuration is invalid:", "✗".red());
            println!("  {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn benchmark_command(requests: usize, threads: usize) -> Result<()> {
    use rayon::prelude::*;
    use std::sync::Arc;

    println!("{} Running benchmark...", "→".blue());
    println!("  Requests: {}", requests);
    println!("  Threads: {}", threads);

    let engine = Arc::new(RUNEEngine::new());

    // Generate test requests
    let test_requests: Vec<Request> = (0..requests)
        .map(|i| {
            RequestBuilder::new()
                .principal(Principal::agent(format!("agent-{}", i % 10)))
                .action(Action::new(if i % 2 == 0 { "read" } else { "write" }))
                .resource(Resource::file(format!("/tmp/file-{}.txt", i % 100)))
                .build()
                .unwrap()
        })
        .collect();

    println!("{} Warming up cache...", "→".blue());

    // Warmup
    for request in test_requests.iter().take(100) {
        let _ = engine.authorize(request);
    }

    println!("{} Running benchmark...", "→".blue());

    let start = Instant::now();

    // Run parallel benchmark
    let results: Vec<_> = test_requests
        .par_iter()
        .map(|request| {
            let result = engine.authorize(request);
            result.is_ok()
        })
        .collect();

    let duration = start.elapsed();

    // Calculate statistics
    let successful = results.iter().filter(|&&r| r).count();
    let failed = requests - successful;
    let throughput = requests as f64 / duration.as_secs_f64();

    println!("\n{} Benchmark Results", "═".blue().bold());
    println!("{} Total requests: {}", "▸".blue(), requests);
    println!("{} Successful: {}", "▸".blue(), successful);
    println!("{} Failed: {}", "▸".blue(), failed);
    println!("{} Duration: {:.3}s", "▸".blue(), duration.as_secs_f64());
    println!("{} Throughput: {:.0} req/sec", "▸".blue(), throughput);
    println!(
        "{} Avg latency: {:.3}ms",
        "▸".blue(),
        duration.as_secs_f64() * 1000.0 / requests as f64
    );

    // Cache stats
    let cache_stats = engine.cache_stats();
    println!("\n{} Cache Statistics", "═".blue().bold());
    println!("{} Cache size: {}", "▸".blue(), cache_stats.size);
    println!(
        "{} Hit rate: {:.1}%",
        "▸".blue(),
        cache_stats.hit_rate * 100.0
    );

    Ok(())
}

async fn serve_command(config: Option<String>, port: u16) -> Result<()> {
    println!("{} Starting RUNE server on port {}...", "→".blue(), port);

    if let Some(config_path) = config {
        println!(
            "{} Loading configuration from {}...",
            "→".blue(),
            config_path
        );
    }

    // TODO: Implement HTTP server
    println!("{} Server functionality not yet implemented", "!".yellow());

    Ok(())
}
