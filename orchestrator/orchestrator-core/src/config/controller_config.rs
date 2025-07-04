//! TaskRun Controller Configuration
//!
//! This module defines the configuration structure for the TaskRun controller,
//! which replaces hard-coded values with ConfigMap-based configuration.

use k8s_openapi::api::core::v1::ConfigMap;
use kube::{api::Api, Client};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Main controller configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ControllerConfig {
    /// Job configuration
    pub job: JobConfig,

    /// Claude agent configuration
    pub agent: AgentConfig,

    /// Init container configuration
    #[serde(rename = "initContainer")]
    pub init_container: InitContainerConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Node affinity configuration
    #[serde(rename = "nodeAffinity")]
    pub node_affinity: NodeAffinityConfig,

    /// Pod configuration
    pub pod: PodConfig,

    /// Controller behavior
    pub controller: ControllerBehaviorConfig,

    /// Volume configuration
    pub volumes: VolumeConfig,

    /// Telemetry configuration
    pub telemetry: TelemetryConfig,

    /// Fluent-bit sidecar configuration
    #[serde(rename = "fluentBit")]
    pub fluent_bit: FluentBitConfig,

    /// Secrets configuration
    pub secrets: SecretsConfig,

    /// Claude-specific settings
    #[serde(rename = "claudeSettings")]
    pub claude_settings: ClaudeSettings,

    /// Toolman server configuration
    #[serde(default)]
    pub toolman: Option<ToolmanConfig>,

    /// Labels to apply to resources
    #[serde(default)]
    pub labels: BTreeMap<String, String>,

    /// Annotations to apply to pods
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
}

/// Job configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JobConfig {
    #[serde(rename = "backoffLimit")]
    pub backoff_limit: i32,

    #[serde(rename = "activeDeadlineSeconds")]
    pub active_deadline_seconds: i64,

    #[serde(rename = "ttlSecondsAfterFinished")]
    pub ttl_seconds_after_finished: i32,

    #[serde(rename = "restartPolicy")]
    pub restart_policy: String,
}

/// Agent configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    pub image: ImageConfig,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub resources: ResourceRequirements,

    #[serde(rename = "securityContext")]
    pub security_context: SecurityContext,

    pub env: Vec<EnvVar>,
}

/// Init container configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitContainerConfig {
    pub image: ImageConfig,
    pub resources: ResourceRequirements,

    #[serde(rename = "securityContext")]
    pub security_context: SecurityContext,

    pub script: String,
}

/// Image configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageConfig {
    pub repository: String,
    pub tag: String,

    #[serde(rename = "pullPolicy")]
    pub pull_policy: String,
}

/// Resource requirements
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceRequirements {
    pub requests: ResourceSpec,
    pub limits: ResourceSpec,
}

/// Resource specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceSpec {
    pub cpu: String,
    pub memory: String,
}

/// Security context
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityContext {
    #[serde(rename = "allowPrivilegeEscalation")]
    pub allow_privilege_escalation: bool,

    #[serde(default)]
    pub capabilities: Option<Capabilities>,

    #[serde(rename = "readOnlyRootFilesystem")]
    pub read_only_root_filesystem: bool,

    #[serde(rename = "runAsNonRoot")]
    pub run_as_non_root: bool,

    #[serde(rename = "runAsUser")]
    pub run_as_user: i64,

    #[serde(rename = "runAsGroup")]
    pub run_as_group: Option<i64>,
}

/// Capabilities
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Capabilities {
    #[serde(default)]
    pub drop: Vec<String>,
}

/// Environment variable
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    #[serde(rename = "storageClass")]
    pub storage_class: String,

    #[serde(rename = "accessMode")]
    pub access_mode: String,

    pub size: String,

    #[serde(rename = "autoCreate")]
    pub auto_create: bool,
}

/// Node affinity configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeAffinityConfig {
    pub enabled: bool,
    pub required: bool,
    pub key: String,

    #[serde(default)]
    pub values: Vec<String>,
}

/// Pod configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PodConfig {
    #[serde(rename = "serviceAccountName")]
    pub service_account_name: String,

    #[serde(rename = "imagePullSecrets")]
    pub image_pull_secrets: Vec<ImagePullSecret>,

    #[serde(rename = "securityContext")]
    pub security_context: PodSecurityContext,

    pub annotations: BTreeMap<String, String>,
}

/// Image pull secret
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImagePullSecret {
    pub name: String,
}

/// Pod security context
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PodSecurityContext {
    #[serde(rename = "fsGroup")]
    pub fs_group: i64,

    #[serde(rename = "runAsNonRoot")]
    pub run_as_non_root: bool,

    #[serde(rename = "runAsUser")]
    pub run_as_user: i64,
}

/// Controller behavior configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ControllerBehaviorConfig {
    #[serde(rename = "reconcileInterval")]
    pub reconcile_interval: u64,

    #[serde(rename = "historyLimit")]
    pub history_limit: i32,

    #[serde(rename = "autoRetry")]
    pub auto_retry: bool,

    #[serde(rename = "maxRetries")]
    pub max_retries: i32,
}

/// Volume configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolumeConfig {
    #[serde(rename = "claudeHome")]
    pub claude_home: ClaudeHomeConfig,

    #[serde(rename = "claudeSettings")]
    pub claude_settings: ClaudeSettingsConfig,

    pub logs: LogConfig,
}

/// Claude home configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaudeHomeConfig {
    pub persistent: bool,

    #[serde(rename = "existingClaim")]
    pub existing_claim: Option<String>,
}

/// Claude settings configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaudeSettingsConfig {
    pub settings: String,
}

/// Log configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    pub enabled: bool,
    pub path: String,
    pub filename: String,
}

/// Fluent-bit configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FluentBitConfig {
    pub enabled: bool,
    pub image: ImageConfig,
    pub resources: ResourceRequirements,
}

/// Telemetry configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelemetryConfig {
    pub enabled: bool,

    #[serde(rename = "otlpEndpoint")]
    pub otlp_endpoint: String,

    #[serde(rename = "otlpProtocol")]
    pub otlp_protocol: String,

    #[serde(rename = "otlpInsecure")]
    pub otlp_insecure: bool,

    #[serde(rename = "serviceName")]
    pub service_name: String,

    #[serde(rename = "serviceVersion")]
    pub service_version: String,

    #[serde(rename = "teamName")]
    pub team_name: String,

    pub department: String,
    pub environment: String,

    #[serde(rename = "costCenter")]
    pub cost_center: String,

    #[serde(rename = "metricsExportInterval")]
    pub metrics_export_interval: String,

    #[serde(rename = "metricsExportTimeout")]
    pub metrics_export_timeout: String,

    #[serde(rename = "logsExportInterval")]
    pub logs_export_interval: String,

    #[serde(rename = "logsExportTimeout")]
    pub logs_export_timeout: String,

    #[serde(rename = "includeSessionId")]
    pub include_session_id: bool,

    #[serde(rename = "includeAccountUuid")]
    pub include_account_uuid: bool,

    #[serde(rename = "includeVersion")]
    pub include_version: bool,

    #[serde(rename = "logLevel")]
    pub log_level: String,

    #[serde(rename = "logUserPrompts")]
    pub log_user_prompts: bool,

    #[serde(rename = "customAttributes")]
    pub custom_attributes: String,

    #[serde(rename = "clusterName")]
    pub cluster_name: String,
}

/// Secrets configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecretsConfig {
    #[serde(rename = "apiKeySecretName")]
    pub api_key_secret_name: String,

    #[serde(rename = "apiKeySecretKey")]
    pub api_key_secret_key: String,

    #[serde(rename = "githubTokenSecretName")]
    pub github_token_secret_name: String,

    #[serde(rename = "githubTokenSecretKey")]
    pub github_token_secret_key: String,

    // Optional secrets
    #[serde(rename = "openaiApiKeySecretName")]
    pub openai_api_key_secret_name: Option<String>,

    #[serde(rename = "openaiApiKeySecretKey")]
    pub openai_api_key_secret_key: Option<String>,

    #[serde(rename = "googleApiKeySecretName")]
    pub google_api_key_secret_name: Option<String>,

    #[serde(rename = "googleApiKeySecretKey")]
    pub google_api_key_secret_key: Option<String>,

    #[serde(rename = "awsBedrockAccessKeySecretName")]
    pub aws_bedrock_access_key_secret_name: Option<String>,

    #[serde(rename = "awsBedrockAccessKeySecretKey")]
    pub aws_bedrock_access_key_secret_key: Option<String>,

    #[serde(rename = "awsBedrockSecretKeySecretName")]
    pub aws_bedrock_secret_key_secret_name: Option<String>,

    #[serde(rename = "awsBedrockSecretKeySecretKey")]
    pub aws_bedrock_secret_key_secret_key: Option<String>,
}

/// Claude-specific settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaudeSettings {
    #[serde(rename = "toolPermissions")]
    pub tool_permissions: ToolPermissions,

    #[serde(rename = "environmentSettings")]
    pub environment_settings: EnvironmentSettings,
}

/// Tool permissions configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolPermissions {
    pub bash: ToolPermission,
    pub edit: ToolPermission,
    pub write: ToolPermission,
    pub read: ToolPermission,
    #[serde(rename = "multiEdit")]
    pub multi_edit: ToolPermission,
    #[serde(rename = "webFetch")]
    pub web_fetch: ToolPermission,
    pub grep: ToolPermission,
    pub glob: ToolPermission,
    pub ls: ToolPermission,
}

/// Individual tool permission
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolPermission {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

/// Environment settings for Claude
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvironmentSettings {
    #[serde(rename = "disableAutoupdater")]
    pub disable_autoupdater: bool,

    #[serde(rename = "disableErrorReporting")]
    pub disable_error_reporting: bool,

    #[serde(rename = "disableNonEssentialModelCalls")]
    pub disable_non_essential_model_calls: bool,

    #[serde(rename = "maintainProjectWorkingDir")]
    pub maintain_project_working_dir: bool,

    #[serde(rename = "cleanupPeriodDays")]
    pub cleanup_period_days: i32,

    #[serde(rename = "includeCoAuthoredBy")]
    pub include_co_authored_by: bool,
}

/// Toolman server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolmanConfig {
    /// Whether toolman is enabled
    pub enabled: bool,

    /// Image configuration for toolman
    pub image: ImageConfig,

    /// Resource requirements for toolman
    pub resources: ResourceRequirements,

    /// Environment variables for toolman
    pub env: Vec<EnvVar>,

    /// Toolman server configuration file path
    #[serde(rename = "configPath")]
    pub config_path: String,

    /// Port for toolman server
    pub port: u16,
}

impl ControllerConfig {
    /// Load configuration from a ConfigMap
    pub async fn from_configmap(
        client: &Client,
        namespace: &str,
        name: &str,
    ) -> Result<Self, anyhow::Error> {
        let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
        let cm = api.get(name).await?;

        let data = cm
            .data
            .ok_or_else(|| anyhow::anyhow!("ConfigMap has no data"))?;
        let config_str = data
            .get("config.yaml")
            .ok_or_else(|| anyhow::anyhow!("ConfigMap missing config.yaml"))?;

        let config: ControllerConfig = serde_yaml::from_str(config_str)?;
        Ok(config)
    }
}

impl Default for ControllerConfig {
    fn default() -> Self {
        // This would typically be loaded from a default YAML embedded in the binary
        // For now, return a reasonable default
        serde_yaml::from_str(include_str!("default_config.yaml"))
            .expect("Default config should be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialization() {
        // Use a minimal config that includes all required fields
        let yaml = r#"
job:
  backoffLimit: 3
  activeDeadlineSeconds: 3600
  ttlSecondsAfterFinished: 86400
  restartPolicy: "OnFailure"

agent:
  image:
    repository: "test/image"
    tag: "latest"
    pullPolicy: "Always"
  command: ["test"]
  args: ["arg1"]
  resources:
    requests:
      cpu: "1"
      memory: "1Gi"
    limits:
      cpu: "2"
      memory: "2Gi"
  securityContext:
    allowPrivilegeEscalation: false
    readOnlyRootFilesystem: false
    runAsNonRoot: true
    runAsUser: 1000
  env:
    - name: TEST
      value: "value"

initContainer:
  image:
    repository: "busybox"
    tag: "1.36"
    pullPolicy: "IfNotPresent"
  resources:
    requests:
      cpu: "100m"
      memory: "128Mi"
    limits:
      cpu: "200m"
      memory: "256Mi"
  securityContext:
    allowPrivilegeEscalation: false
    readOnlyRootFilesystem: false
    runAsNonRoot: true
    runAsUser: 1000
  script: "echo test"

storage:
  storageClass: "standard"
  accessMode: "ReadWriteOnce"
  size: "10Gi"
  autoCreate: true

nodeAffinity:
  enabled: false
  required: false
  key: "test"

pod:
  serviceAccountName: "default"
  imagePullSecrets: []
  securityContext:
    fsGroup: 2000
    runAsNonRoot: true
    runAsUser: 1000
  annotations: {}

controller:
  reconcileInterval: 30
  historyLimit: 5
  autoRetry: false
  maxRetries: 3

volumes:
  claudeHome:
    persistent: false
  claudeSettings:
    settings: "{}"
  logs:
    enabled: true
    path: "/logs"
    filename: "test.log"

telemetry:
  enabled: true
  otlpEndpoint: "localhost:4317"
  otlpProtocol: "grpc"
  otlpInsecure: true
  serviceName: "test-service"
  serviceVersion: "1.0.0"
  teamName: "test-team"
  department: "test-dept"
  environment: "test"
  costCenter: ""
  metricsExportInterval: "60000"
  metricsExportTimeout: "30000"
  logsExportInterval: "5000"
  logsExportTimeout: "30000"
  includeSessionId: true
  includeAccountUuid: true
  includeVersion: false
  logLevel: "info"
  logUserPrompts: false
  customAttributes: ""
  clusterName: "test-cluster"

fluentBit:
  enabled: false
  image:
    repository: "fluent/fluent-bit"
    tag: "3.0"
    pullPolicy: "IfNotPresent"
  resources:
    requests:
      cpu: "50m"
      memory: "64Mi"
    limits:
      cpu: "100m"
      memory: "128Mi"

secrets:
  apiKeySecretName: "test-secret"
  apiKeySecretKey: "key"
  githubTokenSecretName: "github"
  githubTokenSecretKey: "token"

claudeSettings:
  toolPermissions:
    bash:
      allow: ["*"]
      deny: []
    edit:
      allow: ["*"]
      deny: []
    write:
      allow: ["*"]
      deny: []
    read:
      allow: ["*"]
      deny: []
    multiEdit:
      allow: ["*"]
      deny: []
    webFetch:
      allow: ["*"]
      deny: []
    grep:
      allow: ["*"]
      deny: []
    glob:
      allow: ["*"]
      deny: []
    ls:
      allow: ["*"]
      deny: []
  environmentSettings:
    disableAutoupdater: true
    disableErrorReporting: true
    disableNonEssentialModelCalls: true
    maintainProjectWorkingDir: true
    cleanupPeriodDays: 7
    includeCoAuthoredBy: true
"#;

        let config: ControllerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.job.backoff_limit, 3);
        assert_eq!(config.agent.image.repository, "test/image");
        assert_eq!(config.telemetry.service_name, "test-service");
        assert!(config.telemetry.enabled);
    }
}
