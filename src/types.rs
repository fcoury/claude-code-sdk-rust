//! Type definitions for Claude Code SDK messages and configuration.
//!
//! This module contains all the core types used throughout the SDK, including
//! message types, content blocks, and configuration options.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents different types of messages in the Claude Code protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// A message from the user to Claude
    User(UserMessage),
    /// A response message from Claude
    Assistant(AssistantMessage),
    /// A system message containing metadata or control information
    System(SystemMessage),
    /// A result message indicating query completion with metadata
    Result(ResultMessage),
}

/// A message from the user to Claude.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    /// The content of the user message, can be text or structured blocks
    pub content: MessageContent,
}

/// A response message from Claude.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// The content blocks in Claude's response
    pub content: Vec<ContentBlock>,
}

/// A system message containing metadata or control information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    /// The subtype of the system message
    pub subtype: String,
    /// Additional data associated with the system message
    pub data: HashMap<String, serde_json::Value>,
}

/// A result message indicating query completion with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultMessage {
    /// The subtype of the result
    pub subtype: String,
    /// Total duration of the query in milliseconds
    pub duration_ms: i64,
    /// API processing time in milliseconds
    pub duration_api_ms: i64,
    /// Whether the query resulted in an error
    pub is_error: bool,
    /// Number of conversation turns
    pub num_turns: i32,
    /// Session identifier
    pub session_id: String,
    /// Total cost in USD (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// Usage statistics (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<HashMap<String, serde_json::Value>>,
    /// Result data (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

/// Content that can be either plain text or structured blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Structured content blocks
    Blocks(Vec<ContentBlock>),
}

/// Different types of content blocks that can appear in messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// A text content block
    Text(TextBlock),
    /// A tool use request block
    ToolUse(ToolUseBlock),
    /// A tool result response block
    ToolResult(ToolResultBlock),
}

/// A text content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    /// The text content
    pub text: String,
}

/// A tool use request block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUseBlock {
    /// Unique identifier for this tool use
    pub id: String,
    /// Name of the tool to use
    pub name: String,
    /// Input parameters for the tool
    pub input: HashMap<String, serde_json::Value>,
}

/// A tool result response block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultBlock {
    /// The ID of the tool use this result corresponds to
    pub tool_use_id: String,
    /// The result content (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ToolResultContent>,
    /// Whether this result represents an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Content of a tool result, can be text or structured data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain text result
    Text(String),
    /// Structured result data
    Structured(Vec<HashMap<String, serde_json::Value>>),
}

/// Permission modes for Claude Code operations.
///
/// This enum controls how Claude handles permission requests for potentially
/// destructive operations like file edits or command execution.
///
/// # Examples
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, PermissionMode};
///
/// // Require explicit permission for each operation
/// let strict_options = ClaudeCodeOptions::builder()
///     .permission_mode(PermissionMode::Default)
///     .build();
///
/// // Automatically accept edit operations
/// let permissive_options = ClaudeCodeOptions::builder()
///     .permission_mode(PermissionMode::AcceptEdits)
///     .build();
///
/// // Bypass all permission prompts (use with caution)
/// let bypass_options = ClaudeCodeOptions::builder()
///     .permission_mode(PermissionMode::BypassPermissions)
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Default permission behavior
    #[serde(rename = "default")]
    Default,
    /// Automatically accept edit operations
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    /// Bypass permission prompts
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

/// Configuration for MCP (Model Context Protocol) servers.
///
/// MCP servers provide additional capabilities to Claude through standardized protocols.
/// This enum supports different transport mechanisms for connecting to MCP servers.
///
/// # Examples
///
/// ## Standard I/O Server
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, McpServerConfig};
/// use std::collections::HashMap;
///
/// let mut env = HashMap::new();
/// env.insert("LOG_LEVEL".to_string(), "info".to_string());
///
/// let options = ClaudeCodeOptions::builder()
///     .mcp_server("filesystem", McpServerConfig::Stdio {
///         command: "npx".to_string(),
///         args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
///         env: Some(env),
///     })
///     .build();
/// ```
///
/// ## Server-Sent Events Server
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, McpServerConfig};
/// use std::collections::HashMap;
///
/// let mut headers = HashMap::new();
/// headers.insert("Authorization".to_string(), "Bearer token".to_string());
///
/// let options = ClaudeCodeOptions::builder()
///     .mcp_server("remote", McpServerConfig::Sse {
///         url: "https://api.example.com/mcp".to_string(),
///         headers: Some(headers),
///     })
///     .build();
/// ```
///
/// ## HTTP Server
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, McpServerConfig};
///
/// let options = ClaudeCodeOptions::builder()
///     .mcp_server("http_server", McpServerConfig::Http {
///         url: "http://localhost:8080/mcp".to_string(),
///         headers: None,
///     })
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerConfig {
    /// Standard I/O based server
    Stdio {
        /// Command to execute
        command: String,
        /// Command arguments (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        /// Environment variables (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    /// Server-Sent Events based server
    Sse {
        /// Server URL
        url: String,
        /// HTTP headers (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    /// HTTP based server
    Http {
        /// Server URL
        url: String,
        /// HTTP headers (optional)
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

/// Configuration options for Claude Code queries.
///
/// This struct contains all the configuration options that can be passed to Claude Code
/// queries. Use the builder pattern via [`ClaudeCodeOptions::builder()`] for ergonomic
/// configuration.
///
/// # Examples
///
/// ## Basic Configuration
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, PermissionMode};
/// use std::path::PathBuf;
///
/// let options = ClaudeCodeOptions::builder()
///     .system_prompt("You are a helpful coding assistant")
///     .permission_mode(PermissionMode::AcceptEdits)
///     .cwd(PathBuf::from("./my-project"))
///     .build();
/// ```
///
/// ## Advanced Configuration
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, PermissionMode, McpServerConfig};
/// use std::collections::HashMap;
/// use std::path::PathBuf;
///
/// let options = ClaudeCodeOptions::builder()
///     .system_prompt("You are an expert Rust developer")
///     .append_system_prompt("Always write safe, idiomatic code")
///     .permission_mode(PermissionMode::BypassPermissions)
///     .allowed_tools(vec!["file_editor".to_string(), "bash".to_string()])
///     .disallowed_tools(vec!["web_search".to_string()])
///     .max_thinking_tokens(2000)
///     .max_turns(10)
///     .cwd(PathBuf::from("./rust-project"))
///     .model("claude-3-5-sonnet-20241022")
///     .mcp_server("filesystem", McpServerConfig::Stdio {
///         command: "npx".to_string(),
///         args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
///         env: None,
///     })
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptions {
    /// List of allowed tools
    pub allowed_tools: Vec<String>,
    /// Maximum thinking tokens
    pub max_thinking_tokens: i32,
    /// System prompt to use
    pub system_prompt: Option<String>,
    /// Additional system prompt to append
    pub append_system_prompt: Option<String>,
    /// MCP tools to enable
    pub mcp_tools: Vec<String>,
    /// MCP server configurations
    pub mcp_servers: HashMap<String, McpServerConfig>,
    /// Permission mode for operations
    pub permission_mode: Option<PermissionMode>,
    /// Whether to continue previous conversation
    pub continue_conversation: bool,
    /// Session ID to resume
    pub resume: Option<String>,
    /// Maximum number of turns
    pub max_turns: Option<i32>,
    /// List of disallowed tools
    pub disallowed_tools: Vec<String>,
    /// Model to use
    pub model: Option<String>,
    /// Tool name for permission prompts
    pub permission_prompt_tool_name: Option<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Settings file path
    pub settings: Option<String>,
}

impl ClaudeCodeOptions {
    /// Create a new builder for ClaudeCodeOptions.
    pub fn builder() -> ClaudeCodeOptionsBuilder {
        ClaudeCodeOptionsBuilder::default()
    }
}

/// Builder for ClaudeCodeOptions with fluent interface.
///
/// This builder provides a fluent API for constructing [`ClaudeCodeOptions`] instances.
/// All methods return `Self` to enable method chaining.
///
/// # Examples
///
/// ```rust
/// use claude_code_sdk::{ClaudeCodeOptions, PermissionMode};
/// use std::path::PathBuf;
///
/// let options = ClaudeCodeOptions::builder()
///     .system_prompt("You are a helpful assistant")
///     .permission_mode(PermissionMode::AcceptEdits)
///     .allowed_tools(vec!["file_editor".to_string()])
///     .max_thinking_tokens(1500)
///     .cwd(PathBuf::from("./workspace"))
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptionsBuilder {
    inner: ClaudeCodeOptions,
}

impl ClaudeCodeOptionsBuilder {
    /// Set the allowed tools list.
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.inner.allowed_tools = tools;
        self
    }

    /// Set the maximum thinking tokens.
    pub fn max_thinking_tokens(mut self, tokens: i32) -> Self {
        self.inner.max_thinking_tokens = tokens;
        self
    }

    /// Set the system prompt.
    pub fn system_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.inner.system_prompt = Some(prompt.into());
        self
    }

    /// Set the append system prompt.
    pub fn append_system_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.inner.append_system_prompt = Some(prompt.into());
        self
    }

    /// Set the MCP tools list.
    pub fn mcp_tools(mut self, tools: Vec<String>) -> Self {
        self.inner.mcp_tools = tools;
        self
    }

    /// Add an MCP server configuration.
    pub fn mcp_server<S: Into<String>>(mut self, name: S, config: McpServerConfig) -> Self {
        self.inner.mcp_servers.insert(name.into(), config);
        self
    }

    /// Set the permission mode.
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.inner.permission_mode = Some(mode);
        self
    }

    /// Set whether to continue conversation.
    pub fn continue_conversation(mut self, continue_conv: bool) -> Self {
        self.inner.continue_conversation = continue_conv;
        self
    }

    /// Set the session ID to resume.
    pub fn resume<S: Into<String>>(mut self, session_id: S) -> Self {
        self.inner.resume = Some(session_id.into());
        self
    }

    /// Set the maximum number of turns.
    pub fn max_turns(mut self, turns: i32) -> Self {
        self.inner.max_turns = Some(turns);
        self
    }

    /// Set the disallowed tools list.
    pub fn disallowed_tools(mut self, tools: Vec<String>) -> Self {
        self.inner.disallowed_tools = tools;
        self
    }

    /// Set the model to use.
    pub fn model<S: Into<String>>(mut self, model: S) -> Self {
        self.inner.model = Some(model.into());
        self
    }

    /// Set the permission prompt tool name.
    pub fn permission_prompt_tool_name<S: Into<String>>(mut self, name: S) -> Self {
        self.inner.permission_prompt_tool_name = Some(name.into());
        self
    }

    /// Set the working directory.
    pub fn cwd<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.inner.cwd = Some(path.into());
        self
    }

    /// Set the settings file path.
    pub fn settings<S: Into<String>>(mut self, settings: S) -> Self {
        self.inner.settings = Some(settings.into());
        self
    }

    /// Build the ClaudeCodeOptions.
    pub fn build(self) -> ClaudeCodeOptions {
        self.inner
    }
}
