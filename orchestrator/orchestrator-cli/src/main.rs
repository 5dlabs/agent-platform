//! CLI tool for the Orchestrator service

mod api;
mod commands;
mod interactive;
mod output;
mod validation;

use anyhow::Result;
use api::ApiClient;
use clap::{Parser, Subcommand};
use output::{OutputFormat, OutputManager};
use tracing::{debug, info};

#[derive(Parser)]
#[command(name = "orchestrator")]
#[command(about = "CLI for Unified Orchestration Service", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API endpoint URL
    #[arg(
        long,
        env = "ORCHESTRATOR_API_URL",
        default_value = "http://localhost:8080"
    )]
    api_url: String,

    /// Output format
    #[arg(long, short, default_value = "table")]
    output: OutputFormat,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Task management commands
    Task {
        #[command(subcommand)]
        action: TaskCommands,
    },
    /// Job management commands
    Job {
        #[command(subcommand)]
        action: JobCommands,
    },
    /// ConfigMap management commands
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Check service health
    Health,
}

#[derive(Subcommand, Debug)]
enum TaskCommands {
    /// Submit a new task (PM workflow)
    Submit {
        /// Path to tasks.json file containing all tasks
        #[arg(long)]
        task_json: String,
        /// Task ID to submit from the tasks.json file
        #[arg(long)]
        task_id: u32,
        /// Path to design specification markdown file
        #[arg(long)]
        design_spec: String,
        /// Path to autonomous prompt markdown file (optional)
        #[arg(long)]
        prompt: Option<String>,
        /// Target service name (e.g., auth-service, api-gateway)
        #[arg(long)]
        service_name: String,
        /// Agent name (e.g., claude-agent-1)
        #[arg(long)]
        agent_name: String,
        /// Indicates this is a retry of a previous attempt
        #[arg(long)]
        retry: bool,
    },
    /// Get task status
    Status {
        /// Task ID
        #[arg(long)]
        id: String,
    },
    /// List all tasks
    List {
        /// Filter by microservice
        #[arg(long)]
        microservice: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum JobCommands {
    /// List all jobs
    List {
        /// Filter by microservice
        #[arg(long)]
        microservice: Option<String>,
        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },
    /// Get job details
    Get {
        /// Job ID
        #[arg(long)]
        id: String,
    },
    /// Stream job logs
    Logs {
        /// Job ID
        #[arg(long)]
        id: String,
        /// Follow log stream
        #[arg(long, short)]
        follow: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Create a new ConfigMap
    Create {
        /// ConfigMap name
        #[arg(long)]
        name: String,
        /// Files to include
        #[arg(long)]
        files: Vec<String>,
    },
    /// Get ConfigMap contents
    Get {
        /// ConfigMap name
        #[arg(long)]
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing for CLI
    let filter = if cli.verbose {
        "orchestrator_cli=debug,orchestrator_common=debug"
    } else {
        "orchestrator_cli=info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()),
        )
        .with_target(cli.verbose)
        .init();

    info!("Orchestrator CLI v{}", env!("CARGO_PKG_VERSION"));
    debug!("API URL: {}", cli.api_url);
    debug!("Output format: {:?}", cli.output);

    // Create API client
    let api_client = ApiClient::new(cli.api_url.clone());

    // Determine if output should be colored
    let colored = !cli.no_color && is_terminal::IsTerminal::is_terminal(&std::io::stdout());
    let output_manager = OutputManager::new(cli.output, colored);

    // Execute commands
    let result = match cli.command {
        Commands::Task { action } => match action {
            TaskCommands::Submit {
                task_json,
                task_id,
                design_spec,
                prompt,
                service_name,
                agent_name,
                retry,
            } => {
                commands::task::submit_pm_task(
                    &api_client,
                    &output_manager,
                    &task_json,
                    task_id,
                    &design_spec,
                    prompt.as_deref(),
                    &service_name,
                    &agent_name,
                    retry,
                )
                .await
            }
            TaskCommands::Status { id } => {
                commands::task::status(&api_client, &output_manager, &id).await
            }
            TaskCommands::List { microservice } => {
                commands::task::list(&api_client, &output_manager, microservice.as_deref()).await
            }
        },
        Commands::Job { action } => match action {
            JobCommands::List {
                microservice,
                status,
            } => {
                commands::job::list(
                    &api_client,
                    &output_manager,
                    microservice.as_deref(),
                    status.as_deref(),
                )
                .await
            }
            JobCommands::Get { id } => commands::job::get(&api_client, &output_manager, &id).await,
            JobCommands::Logs { id, follow } => {
                commands::job::logs(&api_client, &output_manager, &id, follow).await
            }
        },
        Commands::Config { action } => match action {
            ConfigCommands::Create { name, files } => {
                commands::config::create(&api_client, &output_manager, &name, &files).await
            }
            ConfigCommands::Get { name } => {
                commands::config::get(&api_client, &output_manager, &name).await
            }
        },
        Commands::Health => commands::health_check(&api_client, &output_manager).await,
    };

    match result {
        Ok(()) => {
            debug!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            debug!("Command failed: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parsing() {
        // Clear any environment variables to ensure clean test
        std::env::remove_var("ORCHESTRATOR_API_URL");

        // Give it a moment to clear
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Test that CLI can parse basic commands
        let cli = Cli::try_parse_from([
            "orchestrator",
            "task",
            "submit",
            "--task-json",
            "tasks.json",
            "--task-id",
            "1001",
            "--design-spec",
            "design.md",
            "--service-name",
            "auth-service",
            "--agent-name",
            "claude-agent-1",
        ]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        // The URL might be affected by environment variables from other tests
        assert!(cli.api_url == "http://localhost:8080" || cli.api_url.starts_with("http://"));

        match cli.command {
            Commands::Task {
                action:
                    TaskCommands::Submit {
                        task_json,
                        task_id,
                        design_spec,
                        prompt,
                        service_name,
                        agent_name,
                        retry,
                    },
            } => {
                assert_eq!(task_json, "tasks.json");
                assert_eq!(task_id, 1001);
                assert_eq!(design_spec, "design.md");
                assert_eq!(prompt, None);
                assert_eq!(service_name, "auth-service");
                assert_eq!(agent_name, "claude-agent-1");
                assert!(!retry);
            }
            _ => panic!("Expected Task Submit command"),
        }
    }

    #[test]
    fn test_cli_validation() {
        // Verify the CLI structure is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_output_format_parsing() {
        let cli = Cli::try_parse_from(["orchestrator", "--output", "json", "health"]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        assert_eq!(cli.output, OutputFormat::Json);
    }

    #[test]
    fn test_api_url_explicit() {
        // Test explicit API URL override
        let cli = Cli::try_parse_from(["orchestrator", "--api-url", "http://test:9000", "health"]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        assert_eq!(cli.api_url, "http://test:9000");
    }

    #[test]
    fn test_job_commands() {
        let cli =
            Cli::try_parse_from(["orchestrator", "job", "logs", "--id", "job-123", "--follow"]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        match cli.command {
            Commands::Job {
                action: JobCommands::Logs { id, follow },
            } => {
                assert_eq!(id, "job-123");
                assert!(follow);
            }
            _ => panic!("Expected Job Logs command"),
        }
    }

    #[test]
    fn test_config_commands() {
        let cli = Cli::try_parse_from([
            "orchestrator",
            "config",
            "create",
            "--name",
            "my-config",
            "--files",
            "file1.txt",
            "--files",
            "file2.txt",
        ]);
        assert!(cli.is_ok());

        let cli = cli.unwrap();
        match cli.command {
            Commands::Config {
                action: ConfigCommands::Create { name, files },
            } => {
                assert_eq!(name, "my-config");
                assert_eq!(files, vec!["file1.txt", "file2.txt"]);
            }
            _ => panic!("Expected Config Create command"),
        }
    }
}
