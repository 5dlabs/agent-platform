//! MCP Wrapper - Lightweight HTTP forwarder for MCP protocol
//!
//! This binary acts as a bridge between agents that expect stdin/stdout MCP
//! and the toolman HTTP server. It reads MCP messages from stdin, forwards
//! them to the toolman server via HTTP, and writes responses back to stdout.

use anyhow::{Context, Result};
use serde_json::Value;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufWriter};
use tracing::{debug, error, info};

/// Configuration for the MCP wrapper
#[derive(Debug)]
struct WrapperConfig {
    /// URL of the toolman server
    toolman_url: String,
    /// Timeout for HTTP requests (seconds)
    timeout_seconds: u64,
    /// Enable debug logging
    debug: bool,
}

impl WrapperConfig {
    fn from_env() -> Self {
        Self {
            toolman_url: env::var("MCP_TOOLMAN_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:3000/mcp".to_string()),
            timeout_seconds: env::var("MCP_WRAPPER_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            debug: env::var("MCP_WRAPPER_DEBUG")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
        }
    }
}

/// MCP Wrapper that forwards messages between stdin/stdout and HTTP
struct McpWrapper {
    config: WrapperConfig,
    client: reqwest::Client,
}

impl McpWrapper {
    fn new(config: WrapperConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Run the wrapper with Claude as subprocess
    async fn run_with_subprocess(&self, claude_args: Vec<String>) -> Result<()> {
        info!("Starting MCP wrapper with Claude subprocess");
        info!("Claude command: claude {}", claude_args.join(" "));

        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::process::Command;

        // Start Claude as subprocess
        let mut claude = Command::new("claude")
            .args(&claude_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit()) // Pass through stderr for debugging
            .spawn()
            .context("Failed to start Claude subprocess")?;

        let claude_stdin = claude.stdin.take().context("Failed to get Claude stdin")?;
        let claude_stdout = claude
            .stdout
            .take()
            .context("Failed to get Claude stdout")?;

        let mut claude_writer = BufWriter::new(claude_stdin);
        let mut claude_reader = BufReader::new(claude_stdout);

        // Handle our own stdin/stdout
        let stdin = tokio::io::stdin();
        let mut our_reader = BufReader::new(stdin);
        let stdout = tokio::io::stdout();
        let mut our_writer = BufWriter::new(stdout);

        info!("MCP wrapper ready - proxying between external input and Claude");

        let mut our_line = String::new();
        let mut claude_line = String::new();

        loop {
            tokio::select! {
                // Read from our stdin and forward to Claude
                result = our_reader.read_line(&mut our_line) => {
                    match result {
                        Ok(0) => {
                            debug!("EOF on input, shutting down");
                            break;
                        }
                        Ok(_) => {
                            if self.config.debug {
                                debug!("Input -> Claude: {}", our_line.trim());
                            }

                            claude_writer.write_all(our_line.as_bytes()).await
                                .context("Failed to write to Claude")?;
                            claude_writer.flush().await
                                .context("Failed to flush to Claude")?;
                            our_line.clear();
                        }
                        Err(e) => {
                            error!("Error reading input: {}", e);
                            break;
                        }
                    }
                }

                // Read from Claude and decide whether to forward or proxy
                result = claude_reader.read_line(&mut claude_line) => {
                    match result {
                        Ok(0) => {
                            debug!("Claude process ended");
                            break;
                        }
                        Ok(_) => {
                            let line = claude_line.trim();

                            // Check if this looks like an MCP message
                            if self.is_mcp_message(line) {
                                if self.config.debug {
                                    debug!("Claude -> Toolman (MCP): {}", line);
                                }

                                // Parse and forward to toolman
                                match serde_json::from_str::<Value>(line) {
                                    Ok(message) => {
                                        match self.forward_message(message).await {
                                            Ok(response) => {
                                                let response_str = serde_json::to_string(&response)
                                                    .context("Failed to serialize toolman response")?;

                                                if self.config.debug {
                                                    debug!("Toolman -> Claude: {}", response_str);
                                                }

                                                // Send response back to Claude (this is tricky - Claude expects it on stdin)
                                                claude_writer.write_all(response_str.as_bytes()).await
                                                    .context("Failed to write toolman response to Claude")?;
                                                claude_writer.write_all(b"\n").await
                                                    .context("Failed to write newline to Claude")?;
                                                claude_writer.flush().await
                                                    .context("Failed to flush to Claude")?;
                                            }
                                            Err(e) => {
                                                error!("Failed to forward MCP message: {}", e);
                                                // Send error to Claude
                                                let error_response = serde_json::json!({
                                                    "jsonrpc": "2.0",
                                                    "error": {"code": -32603, "message": format!("Toolman error: {}", e)}
                                                });
                                                let error_str = serde_json::to_string(&error_response).unwrap();
                                                claude_writer.write_all(error_str.as_bytes()).await.ok();
                                                claude_writer.write_all(b"\n").await.ok();
                                                claude_writer.flush().await.ok();
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to parse MCP message from Claude: {}", e);
                                        // Pass through as regular output
                                        our_writer.write_all(claude_line.as_bytes()).await
                                            .context("Failed to write Claude output")?;
                                        our_writer.flush().await
                                            .context("Failed to flush Claude output")?;
                                    }
                                }
                            } else {
                                // Not an MCP message - pass through directly
                                if self.config.debug {
                                    debug!("Claude -> Output (passthrough): {}", line);
                                }

                                our_writer.write_all(claude_line.as_bytes()).await
                                    .context("Failed to write Claude output")?;
                                our_writer.flush().await
                                    .context("Failed to flush Claude output")?;
                            }

                            claude_line.clear();
                        }
                        Err(e) => {
                            error!("Error reading from Claude: {}", e);
                            break;
                        }
                    }
                }

                // Check if Claude process has exited
                _ = claude.wait() => {
                    info!("Claude process has exited");
                    break;
                }
            }
        }

        // Clean up
        let _ = claude.kill().await;
        info!("MCP wrapper shutting down");
        Ok(())
    }

    /// Check if a line looks like an MCP JSON-RPC message
    fn is_mcp_message(&self, line: &str) -> bool {
        if line.trim().is_empty() {
            return false;
        }

        // Quick check for JSON-RPC structure
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            if let Some(obj) = value.as_object() {
                // MCP messages should have jsonrpc and method fields
                return obj.contains_key("jsonrpc")
                    && (obj.contains_key("method")
                        || obj.contains_key("result")
                        || obj.contains_key("error"));
            }
        }

        false
    }

    /// Run the wrapper main loop (direct mode)
    async fn run_direct(&self) -> Result<()> {
        info!(
            "Starting MCP wrapper in direct mode, forwarding to: {}",
            self.config.toolman_url
        );

        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);
        let stdout = tokio::io::stdout();
        let mut writer = BufWriter::new(stdout);

        let mut line = String::new();
        loop {
            line.clear();

            // Read line from stdin
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .context("Failed to read from stdin")?;

            if bytes_read == 0 {
                // EOF reached
                debug!("EOF reached, shutting down wrapper");
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if self.config.debug {
                debug!("Received MCP message: {}", line);
            }

            // Parse JSON to validate it's a proper MCP message
            let message: Value = match serde_json::from_str(line) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to parse JSON message: {} - Raw: {}", e, line);
                    continue;
                }
            };

            // Forward to toolman server
            match self.forward_message(message.clone()).await {
                Ok(response) => {
                    // Write response to stdout
                    let response_str =
                        serde_json::to_string(&response).context("Failed to serialize response")?;

                    if self.config.debug {
                        debug!("Sending response: {}", response_str);
                    }

                    writer
                        .write_all(response_str.as_bytes())
                        .await
                        .context("Failed to write to stdout")?;
                    writer
                        .write_all(b"\n")
                        .await
                        .context("Failed to write newline")?;
                    writer.flush().await.context("Failed to flush stdout")?;
                }
                Err(e) => {
                    error!("Failed to forward message: {}", e);

                    // Send error response back to agent
                    let error_response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": message.get("id"),
                        "error": {
                            "code": -32603,
                            "message": format!("Toolman communication error: {}", e)
                        }
                    });

                    let error_str = serde_json::to_string(&error_response)
                        .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"}}"#.to_string());

                    writer.write_all(error_str.as_bytes()).await.ok();
                    writer.write_all(b"\n").await.ok();
                    writer.flush().await.ok();
                }
            }
        }

        info!("MCP wrapper shutting down");
        Ok(())
    }

    /// Forward a message to the toolman server via HTTP
    async fn forward_message(&self, message: Value) -> Result<Value> {
        debug!("Forwarding message to toolman: {}", self.config.toolman_url);

        let response = self
            .client
            .post(&self.config.toolman_url)
            .json(&message)
            .send()
            .await
            .with_context(|| format!("Failed to send request to {}", self.config.toolman_url))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!("HTTP {} from toolman: {}", status, body));
        }

        let response_json: Value = response
            .json()
            .await
            .context("Failed to parse response JSON from toolman")?;

        debug!("Received response from toolman");
        Ok(response_json)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("Starting MCP wrapper");

    // Load configuration
    let config = WrapperConfig::from_env();

    if config.debug {
        debug!("Configuration: {:?}", config);
    }

    // Get command line arguments for Claude
    let args: Vec<String> = env::args().skip(1).collect();

    // Create wrapper
    let wrapper = McpWrapper::new(config);

    // Decide whether to run with subprocess or direct mode
    if args.is_empty() {
        // Direct mode - wrapper acts as MCP proxy
        info!("Running in direct mode (no Claude subprocess)");
        wrapper.run_direct().await?;
    } else {
        // Subprocess mode - launch Claude and proxy its MCP communication
        info!("Running with Claude subprocess: {:?}", args);
        wrapper.run_with_subprocess(args).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        std::env::set_var("MCP_TOOLMAN_SERVER_URL", "http://test:8080/mcp");
        std::env::set_var("MCP_WRAPPER_TIMEOUT", "60");
        std::env::set_var("MCP_WRAPPER_DEBUG", "true");

        let config = WrapperConfig::from_env();
        assert_eq!(config.toolman_url, "http://test:8080/mcp");
        assert_eq!(config.timeout_seconds, 60);
        assert!(config.debug);

        // Cleanup
        std::env::remove_var("MCP_TOOLMAN_SERVER_URL");
        std::env::remove_var("MCP_WRAPPER_TIMEOUT");
        std::env::remove_var("MCP_WRAPPER_DEBUG");
    }

    #[test]
    fn test_config_defaults() {
        // Ensure clean environment
        std::env::remove_var("MCP_TOOLMAN_SERVER_URL");
        std::env::remove_var("MCP_WRAPPER_TIMEOUT");
        std::env::remove_var("MCP_WRAPPER_DEBUG");

        let config = WrapperConfig::from_env();
        assert_eq!(config.toolman_url, "http://localhost:3000/mcp");
        assert_eq!(config.timeout_seconds, 30);
        assert!(!config.debug);
    }

    #[tokio::test]
    async fn test_wrapper_creation() {
        let config = WrapperConfig {
            toolman_url: "http://localhost:3000/mcp".to_string(),
            timeout_seconds: 30,
            debug: false,
        };

        let wrapper = McpWrapper::new(config);
        assert_eq!(wrapper.config.toolman_url, "http://localhost:3000/mcp");
    }
}
