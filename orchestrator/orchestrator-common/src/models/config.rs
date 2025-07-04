//! Configuration-related models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent types that can execute tasks
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    #[default]
    Claude,
    Gemini,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub image: String,
    pub version: String,
    pub env_vars: HashMap<String, String>,
    pub resources: ResourceLimits,
    pub capabilities: Vec<String>,
    pub mcp_servers: Vec<String>,
}

/// Resource limits and requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_request: String,
    pub cpu_limit: String,
    pub memory_request: String,
    pub memory_limit: String,
    pub ephemeral_storage_request: Option<String>,
    pub ephemeral_storage_limit: Option<String>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub capabilities: Vec<String>,
}

/// Toolman server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolmanConfig {
    /// Whether toolman is enabled
    pub enabled: bool,
    /// Path to toolman configuration file
    pub config_path: String,
    /// Default tool access policy
    pub default_allow_all: bool,
    /// Agent-specific tool policies
    pub agent_policies: HashMap<String, Vec<String>>,
}

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub namespace: String,
    pub agents: HashMap<AgentType, AgentConfig>,
    pub default_timeout_seconds: u32,
    pub max_retry_attempts: u32,
    pub workspace_pvc_template: String,
    pub prepare_job_image: String,
    pub node_selector: HashMap<String, String>,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub toolman_config: Option<ToolmanConfig>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_request: "100m".to_string(),
            cpu_limit: "1000m".to_string(),
            memory_request: "256Mi".to_string(),
            memory_limit: "2Gi".to_string(),
            ephemeral_storage_request: None,
            ephemeral_storage_limit: None,
        }
    }
}

impl AgentType {
    /// Get display name for the agent
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Claude => "Claude Code",
            AgentType::Gemini => "Gemini CLI",
        }
    }

    /// Get the default image for the agent
    pub fn default_image(&self) -> &'static str {
        match self {
            AgentType::Claude => "anthropic/claude-code:latest",
            AgentType::Gemini => "google/gemini-cli:latest",
        }
    }

    /// Check if this agent can be a primary implementer
    pub fn can_implement(&self) -> bool {
        match self {
            AgentType::Claude => true,
            AgentType::Gemini => false, // Gemini is assistance-only in our pattern
        }
    }

    /// Check if this agent can provide assistance
    pub fn can_assist(&self) -> bool {
        match self {
            AgentType::Claude => false, // Claude is implementation-only in our pattern
            AgentType::Gemini => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_capabilities() {
        assert!(AgentType::Claude.can_implement());
        assert!(!AgentType::Claude.can_assist());
        assert!(!AgentType::Gemini.can_implement());
        assert!(AgentType::Gemini.can_assist());
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.cpu_request, "100m");
        assert_eq!(limits.memory_limit, "2Gi");
    }

    #[test]
    fn test_agent_config_serialization() {
        let config = AgentConfig {
            agent_type: AgentType::Claude,
            image: "claude:v1".to_string(),
            version: "1.0.0".to_string(),
            env_vars: HashMap::new(),
            resources: ResourceLimits::default(),
            capabilities: vec!["code".to_string(), "test".to_string()],
            mcp_servers: vec!["taskmaster".to_string()],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.agent_type, deserialized.agent_type);
        assert_eq!(config.capabilities, deserialized.capabilities);
    }
}
