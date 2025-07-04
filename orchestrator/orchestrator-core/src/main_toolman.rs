//! Toolman MCP Server
//!
//! A tool management proxy server that filters and controls access to MCP tools
//! for agents. This server acts as a middleware layer between agents and actual
//! MCP tool servers, providing fine-grained control over tool availability.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Configuration for the toolman server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolmanConfig {
    /// Map of server names to their configurations
    pub servers: HashMap<String, ServerConfig>,
    /// Global list of exposed tools per server (subset filtering)
    pub exposed_tools: HashMap<String, Vec<String>>,
    /// Agent-specific policies
    pub agent_policies: HashMap<String, AgentPolicy>,
    /// Default policy settings
    pub default_policy: DefaultPolicy,
}

/// Configuration for a backend MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Command to run the server
    pub command: String,
    /// Arguments to pass to the server
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Whether this server is enabled
    pub enabled: bool,
    /// Description of this server
    #[serde(default)]
    pub description: String,
}

/// Agent-specific policy for tool access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicy {
    /// List of server names this agent can access
    pub allowed_servers: Vec<String>,
    /// Per-server tool overrides (if not specified, uses global exposed_tools)
    #[serde(default)]
    pub tool_overrides: HashMap<String, Vec<String>>,
}

/// Default policy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultPolicy {
    /// Whether to allow tools not in the exposed list
    pub allow_unknown_tools: bool,
    /// Whether to allow servers not in the agent policy
    pub allow_unknown_servers: bool,
}

/// MCP message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum McpMessage {
    #[serde(rename = "initialize")]
    Initialize { id: Value, params: Value },
    #[serde(rename = "tools/list")]
    ListTools { id: Value, params: Option<Value> },
    #[serde(rename = "tools/call")]
    CallTool { id: Value, params: Value },
    #[serde(rename = "resources/list")]
    ListResources { id: Value, params: Option<Value> },
    #[serde(rename = "resources/read")]
    ReadResource { id: Value, params: Value },
    #[serde(rename = "prompts/list")]
    ListPrompts { id: Value, params: Option<Value> },
    #[serde(rename = "prompts/get")]
    GetPrompt { id: Value, params: Value },
    #[serde(other)]
    Other,
}

/// Toolman server state
#[derive(Debug, Clone)]
pub struct ToolmanServer {
    config: Arc<RwLock<ToolmanConfig>>,
    available_tools: Arc<RwLock<HashMap<String, Value>>>,
    server_processes: Arc<RwLock<HashMap<String, std::process::Child>>>,
}

impl ToolmanServer {
    /// Create a new toolman server
    pub fn new(config: ToolmanConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            available_tools: Arc::new(RwLock::new(HashMap::new())),
            server_processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the server and connect to backend MCP servers
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing toolman server");

        let config = self.config.read().await;

        // Start all enabled backend servers
        for (name, server_config) in &config.servers {
            if server_config.enabled {
                self.start_backend_server(name, server_config).await?;
            }
        }

        // Discover available tools from all backend servers
        self.discover_tools().await?;

        Ok(())
    }

    /// Start a backend MCP server
    async fn start_backend_server(&self, name: &str, config: &ServerConfig) -> Result<()> {
        info!("Starting backend server: {}", name);

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to start backend server: {name}"))?;

        let mut processes = self.server_processes.write().await;
        processes.insert(name.to_string(), child);

        Ok(())
    }

    /// Discover tools from all backend servers
    async fn discover_tools(&self) -> Result<()> {
        info!("Discovering tools from backend servers");

        let mut all_tools = HashMap::new();

        // For each backend server, send a tools/list request
        let processes = self.server_processes.read().await;
        for (server_name, _) in processes.iter() {
            match self.list_tools_from_server(server_name).await {
                Ok(tools) => {
                    for tool in tools {
                        let tool_name = tool
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        all_tools.insert(format!("{server_name}:{tool_name}"), tool);
                    }
                }
                Err(e) => {
                    warn!("Failed to list tools from server {}: {}", server_name, e);
                }
            }
        }

        let mut available_tools = self.available_tools.write().await;
        *available_tools = all_tools;

        info!("Discovered {} tools", available_tools.len());
        Ok(())
    }

    /// List tools from a specific backend server
    async fn list_tools_from_server(&self, server_name: &str) -> Result<Vec<Value>> {
        // This is a simplified implementation
        // In a real implementation, you'd communicate with the backend server
        // via stdin/stdout using the MCP protocol

        debug!("Listing tools from server: {}", server_name);

        // For now, return some mock tools
        Ok(vec![
            json!({
                "name": "echo",
                "description": "Echo a message",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "message": {"type": "string"}
                    }
                }
            }),
            json!({
                "name": "get_weather",
                "description": "Get weather information",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                }
            }),
        ])
    }

    /// Check if a tool is allowed for the current agent
    async fn is_tool_allowed(
        &self,
        tool_name: &str,
        server_name: &str,
        agent_id: Option<&str>,
    ) -> bool {
        let config = self.config.read().await;

        // Get agent policy or use default
        let agent_policy = agent_id.and_then(|id| config.agent_policies.get(id));

        // Check if agent can access this server
        if let Some(policy) = agent_policy {
            if !policy.allowed_servers.contains(&server_name.to_string()) {
                debug!(
                    "Agent {} not allowed to access server {}",
                    agent_id.unwrap_or("unknown"),
                    server_name
                );
                return false;
            }

            // Check tool-specific overrides for this agent
            if let Some(allowed_tools) = policy.tool_overrides.get(server_name) {
                return allowed_tools.contains(&tool_name.to_string());
            }
        }

        // Fall back to global exposed tools list
        if let Some(exposed_tools) = config.exposed_tools.get(server_name) {
            if exposed_tools.contains(&tool_name.to_string()) {
                return true;
            }
        }

        // Check default policy
        if config.default_policy.allow_unknown_tools {
            debug!(
                "Allowing unknown tool {} from server {} due to default policy",
                tool_name, server_name
            );
            return true;
        }

        debug!(
            "Tool {} from server {} not allowed for agent {}",
            tool_name,
            server_name,
            agent_id.unwrap_or("unknown")
        );
        false
    }

    /// Handle an MCP message
    pub async fn handle_message(&self, message: Value) -> Result<Value> {
        debug!("Handling message: {:?}", message);

        // Parse the message
        let mcp_message: McpMessage =
            serde_json::from_value(message.clone()).unwrap_or(McpMessage::Other);

        match mcp_message {
            McpMessage::Initialize { id, params } => self.handle_initialize(id, params).await,
            McpMessage::ListTools { id, params } => self.handle_list_tools(id, params).await,
            McpMessage::CallTool { id, params } => self.handle_call_tool(id, params).await,
            McpMessage::ListResources { id, params } => {
                self.handle_list_resources(id, params).await
            }
            McpMessage::ReadResource { id, params } => self.handle_read_resource(id, params).await,
            McpMessage::ListPrompts { id, params } => self.handle_list_prompts(id, params).await,
            McpMessage::GetPrompt { id, params } => self.handle_get_prompt(id, params).await,
            McpMessage::Other => {
                // Pass through unknown messages
                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": message.get("id"),
                    "error": {
                        "code": -32601,
                        "message": "Method not found"
                    }
                }))
            }
        }
    }

    /// Handle initialize message
    async fn handle_initialize(&self, id: Value, _params: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "toolman",
                    "version": "0.1.0"
                }
            }
        }))
    }

    /// Handle list tools message
    async fn handle_list_tools(&self, id: Value, _params: Option<Value>) -> Result<Value> {
        // Get agent ID from environment
        let agent_id = std::env::var("AGENT_NAME").ok();

        debug!("Listing tools for agent: {:?}", agent_id);

        // Get all available tools from backend servers
        let available_tools = self.available_tools.read().await;
        let mut filtered_tools = Vec::new();

        // Filter tools based on configuration
        for (tool_key, tool_definition) in available_tools.iter() {
            // Parse server name from tool key (format: "server:tool_name")
            if let Some((server_name, tool_name)) = tool_key.split_once(':') {
                if self
                    .is_tool_allowed(tool_name, server_name, agent_id.as_deref())
                    .await
                {
                    // Add server context to tool definition
                    let mut tool_with_context = tool_definition.clone();
                    if let Some(obj) = tool_with_context.as_object_mut() {
                        obj.insert("server".to_string(), json!(server_name));
                        obj.insert("qualified_name".to_string(), json!(tool_key));
                    }
                    filtered_tools.push(tool_with_context);

                    debug!("Exposing tool: {} from server: {}", tool_name, server_name);
                } else {
                    debug!(
                        "Filtering out tool: {} from server: {}",
                        tool_name, server_name
                    );
                }
            }
        }

        info!(
            "Agent {} has access to {} tools (out of {} total)",
            agent_id.as_deref().unwrap_or("unknown"),
            filtered_tools.len(),
            available_tools.len()
        );

        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": filtered_tools
            }
        }))
    }

    /// Handle call tool message
    async fn handle_call_tool(&self, id: Value, params: Value) -> Result<Value> {
        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");

        let agent_id = std::env::var("AGENT_NAME").ok();

        debug!(
            "Agent {} calling tool: {}",
            agent_id.as_deref().unwrap_or("unknown"),
            tool_name
        );

        // Parse qualified tool name (server:tool_name or just tool_name)
        let (server_name, actual_tool_name) = if tool_name.contains(':') {
            // Qualified name: "github:create_pull_request"
            let (server, tool) = tool_name.split_once(':').unwrap();
            (server.to_string(), tool)
        } else {
            // Unqualified name - need to find which server has this tool
            match self.find_server_for_tool(tool_name).await {
                Some(server) => (server, tool_name),
                None => {
                    return Ok(json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {
                            "code": -32601,
                            "message": format!("Tool '{}' not found in any server", tool_name)
                        }
                    }));
                }
            }
        };

        // Check if tool is allowed
        if !self
            .is_tool_allowed(actual_tool_name, &server_name, agent_id.as_deref())
            .await
        {
            warn!(
                "Agent {} attempted to call unauthorized tool: {}:{}",
                agent_id.as_deref().unwrap_or("unknown"),
                server_name,
                actual_tool_name
            );
            return Ok(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("Tool '{}' from server '{}' is not allowed", actual_tool_name, server_name)
                }
            }));
        }

        info!(
            "Executing tool {}:{} for agent {}",
            server_name,
            actual_tool_name,
            agent_id.as_deref().unwrap_or("unknown")
        );

        // TODO: Forward the tool call to the appropriate backend server
        // For now, return a mock response indicating the call was authorized
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": format!("Tool '{}' from server '{}' executed successfully (mock response)",
                                       actual_tool_name, server_name)
                    }
                ]
            }
        }))
    }

    /// Find which server provides a given tool name
    async fn find_server_for_tool(&self, tool_name: &str) -> Option<String> {
        let available_tools = self.available_tools.read().await;

        for tool_key in available_tools.keys() {
            if let Some((server_name, tool)) = tool_key.split_once(':') {
                if tool == tool_name {
                    return Some(server_name.to_string());
                }
            }
        }

        None
    }

    /// Handle list resources message
    async fn handle_list_resources(&self, id: Value, _params: Option<Value>) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": []
            }
        }))
    }

    /// Handle read resource message
    async fn handle_read_resource(&self, id: Value, _params: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "contents": []
            }
        }))
    }

    /// Handle list prompts message
    async fn handle_list_prompts(&self, id: Value, _params: Option<Value>) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "prompts": []
            }
        }))
    }

    /// Handle get prompt message
    async fn handle_get_prompt(&self, id: Value, _params: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "messages": []
            }
        }))
    }

    /// Run the HTTP server
    pub async fn run(&self) -> Result<()> {
        use axum::{extract::State, http::StatusCode, response::Json, routing::post, Router};

        info!("Starting toolman HTTP server");

        let server = Arc::new(self.clone());

        // Create HTTP handler for MCP messages
        let handle_mcp = |State(server): State<Arc<ToolmanServer>>, Json(message): Json<Value>| async move {
            match server.handle_message(message).await {
                Ok(response) => Ok(Json(response)),
                Err(e) => {
                    error!("Error handling MCP message: {}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Internal error: {e}"),
                    ))
                }
            }
        };

        // Build the router
        let app = Router::new()
            .route("/mcp", post(handle_mcp))
            .route("/health", axum::routing::get(|| async { "OK" }))
            .with_state(server);

        // Get port from environment or use default
        let port = std::env::var("TOOLMAN_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);

        let addr = format!("0.0.0.0:{port}");
        info!("Toolman server listening on {}", addr);

        // Start the HTTP server
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Failed to bind to {addr}"))?;

        axum::serve(listener, app)
            .with_graceful_shutdown(ToolmanServer::shutdown_signal())
            .await
            .with_context(|| "HTTP server error")?;

        Ok(())
    }

    /// Graceful shutdown signal handler
    async fn shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received, starting graceful shutdown");
    }
}

/// Load configuration from environment or file
fn load_config() -> Result<ToolmanConfig> {
    let config_path =
        env::var("TOOLMAN_CONFIG_PATH").unwrap_or_else(|_| "toolman.json".to_string());

    if std::path::Path::new(&config_path).exists() {
        let config_str = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {config_path}"))?;

        serde_json::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {config_path}"))
    } else {
        // Return default configuration
        Ok(ToolmanConfig {
            servers: HashMap::new(),
            exposed_tools: HashMap::new(),
            agent_policies: HashMap::new(),
            default_policy: DefaultPolicy {
                allow_unknown_tools: false,
                allow_unknown_servers: false,
            },
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting toolman server");

    // Load configuration
    let config = load_config()?;

    // Create and initialize the server
    let server = ToolmanServer::new(config);
    server.initialize().await?;

    // Run the server
    server.run().await?;

    info!("Toolman server shutting down");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_toolman_server_creation() {
        let config = ToolmanConfig {
            servers: HashMap::new(),
            exposed_tools: HashMap::new(),
            agent_policies: HashMap::new(),
            default_policy: DefaultPolicy {
                allow_unknown_tools: false,
                allow_unknown_servers: false,
            },
        };

        let server = ToolmanServer::new(config);
        assert!(server.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let config = ToolmanConfig {
            servers: HashMap::new(),
            exposed_tools: HashMap::new(),
            agent_policies: HashMap::new(),
            default_policy: DefaultPolicy {
                allow_unknown_tools: false,
                allow_unknown_servers: false,
            },
        };

        let server = ToolmanServer::new(config);
        let response = server.handle_initialize(json!(1), json!({})).await.unwrap();

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"]["serverInfo"]["name"] == "toolman");
    }
}
