//! Transport layer for communicating with the Claude Code CLI.
//!
//! This module handles subprocess management, I/O operations, and message
//! buffering for reliable communication with the CLI process.

use crate::errors::{Result, SdkError};
use crate::types::ClaudeCodeOptions;
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio_stream::Stream;

/// Input for prompts, can be text or an async stream.
pub enum PromptInput {
    /// Static text prompt
    Text(String),
    /// Async stream of JSON values
    Stream(Pin<Box<dyn Stream<Item = serde_json::Value> + Send>>),
}

impl std::fmt::Debug for PromptInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromptInput::Text(text) => f.debug_tuple("Text").field(text).finish(),
            PromptInput::Stream(_) => f.debug_tuple("Stream").field(&"<stream>").finish(),
        }
    }
}

impl From<&str> for PromptInput {
    fn from(text: &str) -> Self {
        PromptInput::Text(text.to_string())
    }
}

impl From<String> for PromptInput {
    fn from(text: String) -> Self {
        PromptInput::Text(text)
    }
}

/// Transport implementation using subprocess CLI communication.
pub struct SubprocessCliTransport {
    prompt: PromptInput,
    options: ClaudeCodeOptions,
    cli_path: String,
    process: Option<Child>,
    stdout_stream: Option<BufReader<ChildStdout>>,
    stderr_stream: Option<BufReader<ChildStderr>>,
    stdin_stream: Option<ChildStdin>,
    is_streaming: bool,
    close_stdin_after_prompt: bool,
    request_counter: AtomicU64,
    pending_control_responses: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl SubprocessCliTransport {
    /// Create a new transport instance.
    pub fn new(
        prompt: PromptInput,
        options: ClaudeCodeOptions,
        cli_path: Option<PathBuf>,
        close_stdin_after_prompt: bool,
    ) -> Result<Self> {
        let cli_path = match cli_path {
            Some(path) => path.to_string_lossy().to_string(),
            None => Self::find_cli()?,
        };

        let is_streaming = matches!(prompt, PromptInput::Stream(_));

        Ok(Self {
            prompt,
            options,
            cli_path,
            process: None,
            stdout_stream: None,
            stderr_stream: None,
            stdin_stream: None,
            is_streaming,
            close_stdin_after_prompt,
            request_counter: AtomicU64::new(0),
            pending_control_responses: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Find the Claude Code CLI in the system PATH.
    /// 
    /// This function searches for the Claude CLI in standard installation paths
    /// and provides helpful error messages for common issues.
    pub fn find_cli() -> Result<String> {
        // First, check if Node.js is available
        if which::which("node").is_err() {
            return Err(SdkError::NodeJsNotFound);
        }

        // Try to find the claude binary in standard locations
        let cli_candidates = [
            "claude",
            "claude-code",
            // Common npm global installation paths
            "~/.npm-global/bin/claude",
            "/usr/local/bin/claude",
            // Windows paths
            "C:\\Users\\%USERNAME%\\AppData\\Roaming\\npm\\claude.cmd",
            "C:\\Program Files\\nodejs\\claude.cmd",
        ];

        for candidate in &cli_candidates {
            // Expand tilde and environment variables
            let expanded_path = Self::expand_path(candidate);
            
            if let Ok(path) = which::which(&expanded_path) {
                return Ok(path.to_string_lossy().to_string());
            }
        }

        // If not found in standard locations, try searching PATH
        match which::which("claude") {
            Ok(path) => Ok(path.to_string_lossy().to_string()),
            Err(e) => {
                // Provide helpful error message with installation instructions
                Err(SdkError::CliNotFound(e))
            }
        }
    }

    /// Expand path with tilde and environment variables.
    fn expand_path(path: &str) -> String {
        if path.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                return path.replacen('~', &home, 1);
            }
        }
        
        // Handle Windows environment variables
        if path.contains("%USERNAME%") {
            if let Ok(username) = std::env::var("USERNAME") {
                return path.replace("%USERNAME%", &username);
            }
        }
        
        path.to_string()
    }

    /// Build the command arguments for the CLI.
    /// 
    /// Converts ClaudeCodeOptions to CLI arguments, handling both streaming
    /// and string mode differences, and supporting all configuration options.
    pub fn build_command(&self) -> Vec<String> {
        let mut args = vec!["code".to_string()];

        // Add streaming mode flag if needed
        if self.is_streaming {
            args.push("--streaming".to_string());
        }

        // Tool configuration
        if !self.options.allowed_tools.is_empty() {
            args.push("--allowed-tools".to_string());
            args.push(self.options.allowed_tools.join(","));
        }

        if !self.options.disallowed_tools.is_empty() {
            args.push("--disallowed-tools".to_string());
            args.push(self.options.disallowed_tools.join(","));
        }

        // Model configuration
        if let Some(ref model) = self.options.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        // Thinking tokens configuration
        if self.options.max_thinking_tokens > 0 {
            args.push("--max-thinking-tokens".to_string());
            args.push(self.options.max_thinking_tokens.to_string());
        }

        // System prompt configuration
        if let Some(ref system_prompt) = self.options.system_prompt {
            args.push("--system-prompt".to_string());
            args.push(system_prompt.clone());
        }

        if let Some(ref append_system_prompt) = self.options.append_system_prompt {
            args.push("--append-system-prompt".to_string());
            args.push(append_system_prompt.clone());
        }

        // MCP (Model Context Protocol) configuration
        if !self.options.mcp_tools.is_empty() {
            args.push("--mcp-tools".to_string());
            args.push(self.options.mcp_tools.join(","));
        }

        // Add MCP server configurations
        if !self.options.mcp_servers.is_empty() {
            for (name, config) in &self.options.mcp_servers {
                args.push("--mcp-server".to_string());
                args.push(format!("{}={}", name, Self::serialize_mcp_server_config(config)));
            }
        }

        // Permission configuration
        if let Some(ref permission_mode) = self.options.permission_mode {
            args.push("--permission-mode".to_string());
            args.push(match permission_mode {
                crate::types::PermissionMode::Default => "default".to_string(),
                crate::types::PermissionMode::AcceptEdits => "acceptEdits".to_string(),
                crate::types::PermissionMode::BypassPermissions => "bypassPermissions".to_string(),
            });
        }

        if let Some(ref permission_tool) = self.options.permission_prompt_tool_name {
            args.push("--permission-prompt-tool-name".to_string());
            args.push(permission_tool.clone());
        }

        // Conversation flow configuration
        if self.options.continue_conversation {
            args.push("--continue".to_string());
        }

        if let Some(ref resume) = self.options.resume {
            args.push("--resume".to_string());
            args.push(resume.clone());
        }

        if let Some(max_turns) = self.options.max_turns {
            args.push("--max-turns".to_string());
            args.push(max_turns.to_string());
        }

        // Working directory configuration
        if let Some(ref cwd) = self.options.cwd {
            args.push("--cwd".to_string());
            args.push(cwd.to_string_lossy().to_string());
        }

        // Settings configuration
        if let Some(ref settings) = self.options.settings {
            args.push("--settings".to_string());
            args.push(settings.clone());
        }

        args
    }

    /// Serialize MCP server configuration to CLI argument format.
    pub fn serialize_mcp_server_config(config: &crate::types::McpServerConfig) -> String {
        match config {
            crate::types::McpServerConfig::Stdio { command, args, env } => {
                let mut parts = vec![format!("stdio:{}", command)];
                
                if let Some(args) = args {
                    if !args.is_empty() {
                        parts.push(format!("args={}", args.join(",")));
                    }
                }
                
                if let Some(env) = env {
                    if !env.is_empty() {
                        let env_pairs: Vec<String> = env
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect();
                        parts.push(format!("env={}", env_pairs.join(",")));
                    }
                }
                
                parts.join(";")
            }
            crate::types::McpServerConfig::Sse { url, headers } => {
                let mut parts = vec![format!("sse:{}", url)];
                
                if let Some(headers) = headers {
                    if !headers.is_empty() {
                        let header_pairs: Vec<String> = headers
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect();
                        parts.push(format!("headers={}", header_pairs.join(",")));
                    }
                }
                
                parts.join(";")
            }
            crate::types::McpServerConfig::Http { url, headers } => {
                let mut parts = vec![format!("http:{}", url)];
                
                if let Some(headers) = headers {
                    if !headers.is_empty() {
                        let header_pairs: Vec<String> = headers
                            .iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect();
                        parts.push(format!("headers={}", header_pairs.join(",")));
                    }
                }
                
                parts.join(";")
            }
        }
    }

    /// Connect to the CLI process.
    pub async fn connect(&mut self) -> Result<()> {
        let args = self.build_command();
        
        let mut command = Command::new(&self.cli_path);
        command.args(&args);
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        // Set working directory if specified
        if let Some(ref cwd) = self.options.cwd {
            if !cwd.exists() {
                return Err(SdkError::invalid_working_directory(
                    cwd.to_string_lossy().to_string(),
                ));
            }
            command.current_dir(cwd);
        }

        let mut child = command.spawn()?;

        // Set up streams
        let stdin = child.stdin.take().ok_or_else(|| {
            SdkError::transport("Failed to get stdin handle from child process")
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            SdkError::transport("Failed to get stdout handle from child process")
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            SdkError::transport("Failed to get stderr handle from child process")
        })?;

        self.stdin_stream = Some(stdin);
        self.stdout_stream = Some(BufReader::new(stdout));
        self.stderr_stream = Some(BufReader::new(stderr));
        self.process = Some(child);

        // Send initial prompt if it's text
        if let PromptInput::Text(text) = &self.prompt {
            let text = text.clone();
            self.send_text_prompt(&text).await?;
            
            if self.close_stdin_after_prompt {
                if let Some(mut stdin) = self.stdin_stream.take() {
                    stdin.shutdown().await?;
                }
            }
        }

        Ok(())
    }

    /// Send a text prompt to the CLI.
    async fn send_text_prompt(&mut self, text: &str) -> Result<()> {
        if let Some(ref mut stdin) = self.stdin_stream {
            stdin.write_all(text.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }
        Ok(())
    }

    /// Disconnect from the CLI process.
    pub async fn disconnect(&mut self) -> Result<()> {
        // Close stdin first
        if let Some(mut stdin) = self.stdin_stream.take() {
            let _ = stdin.shutdown().await;
        }

        // Wait for process to exit or kill it
        if let Some(mut process) = self.process.take() {
            // Try graceful shutdown first
            let exit_status = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                process.wait(),
            ).await;

            match exit_status {
                Ok(Ok(status)) => {
                    if !status.success() {
                        // Collect stderr if available
                        let stderr = self.collect_stderr().await.unwrap_or_default();
                        return Err(SdkError::process(status.code(), stderr));
                    }
                }
                Ok(Err(e)) => return Err(SdkError::CliConnection(e)),
                Err(_) => {
                    // Timeout - force kill
                    let _ = process.kill().await;
                    let _ = process.wait().await;
                }
            }
        }

        // Clean up streams
        self.stdout_stream = None;
        self.stderr_stream = None;

        Ok(())
    }

    /// Collect stderr output for error reporting.
    async fn collect_stderr(&mut self) -> Result<String> {
        if let Some(ref mut stderr) = self.stderr_stream {
            let mut stderr_content = String::new();
            let mut line = String::new();
            
            // Try to read available stderr content with timeout
            while let Ok(Ok(bytes_read)) = tokio::time::timeout(
                std::time::Duration::from_millis(100),
                stderr.read_line(&mut line),
            ).await {
                if bytes_read == 0 {
                    break;
                }
                stderr_content.push_str(&line);
                line.clear();
            }
            
            Ok(stderr_content)
        } else {
            Ok(String::new())
        }
    }

    /// Receive messages from the CLI as an async stream.
    pub async fn receive_messages(&mut self) -> impl Stream<Item = Result<serde_json::Value>> + '_ {
        async_stream::stream! {
            if let Some(ref mut stdout) = self.stdout_stream {
                let mut buffer = JsonBuffer::new();
                let mut line = String::new();

                loop {
                    line.clear();
                    match stdout.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            buffer.append(&line);
                            
                            // Try to parse complete JSON objects
                            while let Some(value) = buffer.try_parse_and_clear()? {
                                yield Ok(value);
                            }
                        }
                        Err(e) => {
                            yield Err(SdkError::CliConnection(e));
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Send a request in streaming mode.
    pub async fn send_request(
        &mut self,
        messages: Vec<serde_json::Value>,
        options: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        if let Some(ref mut stdin) = self.stdin_stream {
            let request = serde_json::json!({
                "messages": messages,
                "options": options
            });
            
            let request_str = serde_json::to_string(&request)?;
            stdin.write_all(request_str.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }
        Ok(())
    }

    /// Send an interrupt control signal.
    pub async fn interrupt(&mut self) -> Result<()> {
        let request_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        
        let control_request = serde_json::json!({
            "type": "control",
            "request_id": request_id.to_string(),
            "action": "interrupt"
        });

        if let Some(ref mut stdin) = self.stdin_stream {
            let request_str = serde_json::to_string(&control_request)?;
            stdin.write_all(request_str.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        // TODO: Wait for control response acknowledgment
        // This would involve tracking the request_id and waiting for a matching response
        
        Ok(())
    }
}

impl Drop for SubprocessCliTransport {
    fn drop(&mut self) {
        // Best-effort cleanup
        if let Some(mut process) = self.process.take() {
            tokio::spawn(async move {
                let _ = process.kill().await;
                let _ = process.wait().await;
            });
        }
    }
}

/// Buffer for handling JSON parsing from streaming input.
struct JsonBuffer {
    buffer: String,
    max_size: usize,
}

impl JsonBuffer {
    const MAX_BUFFER_SIZE: usize = 1024 * 1024; // 1MB limit

    fn new() -> Self {
        Self {
            buffer: String::new(),
            max_size: Self::MAX_BUFFER_SIZE,
        }
    }

    fn try_parse_and_clear(&mut self) -> Result<Option<serde_json::Value>> {
        if self.buffer.len() > self.max_size {
            return Err(SdkError::buffer_size_exceeded(self.max_size));
        }

        // Try to parse the buffer as JSON
        match serde_json::from_str(&self.buffer.trim()) {
            Ok(value) => {
                self.buffer.clear();
                Ok(Some(value))
            }
            Err(_) => {
                // Not yet complete JSON, keep buffering
                Ok(None)
            }
        }
    }

    fn append(&mut self, data: &str) {
        self.buffer.push_str(data);
    }
}