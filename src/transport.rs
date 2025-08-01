//! Transport layer for communicating with the Claude Code CLI.
//!
//! This module handles subprocess management, I/O operations, and message
//! buffering for reliable communication with the CLI process.

use crate::errors::{Result, SdkError};
use crate::types::ClaudeCodeOptions;
use async_stream;
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
                args.push(format!(
                    "{}={}",
                    name,
                    Self::serialize_mcp_server_config(config)
                ));
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
                        let env_pairs: Vec<String> =
                            env.iter().map(|(k, v)| format!("{k}={v}")).collect();
                        parts.push(format!("env={}", env_pairs.join(",")));
                    }
                }

                parts.join(";")
            }
            crate::types::McpServerConfig::Sse { url, headers } => {
                let mut parts = vec![format!("sse:{url}")];

                if let Some(headers) = headers {
                    if !headers.is_empty() {
                        let header_pairs: Vec<String> = headers
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect();
                        parts.push(format!("headers={}", header_pairs.join(",")));
                    }
                }

                parts.join(";")
            }
            crate::types::McpServerConfig::Http { url, headers } => {
                let mut parts = vec![format!("http:{url}")];

                if let Some(headers) = headers {
                    if !headers.is_empty() {
                        let header_pairs: Vec<String> = headers
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect();
                        parts.push(format!("headers={}", header_pairs.join(",")));
                    }
                }

                parts.join(";")
            }
        }
    }

    /// Connect to the CLI process with comprehensive error handling.
    ///
    /// This method handles process startup, stream setup, and initial prompt sending
    /// with detailed error reporting for common failure scenarios.
    pub async fn connect(&mut self) -> Result<()> {
        let args = self.build_command();

        // Validate CLI path exists and is executable
        if !std::path::Path::new(&self.cli_path).exists() {
            return Err(SdkError::transport(format!(
                "CLI executable not found at path: {}. Please ensure the Claude Code CLI is installed.",
                self.cli_path
            )));
        }

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
            if !cwd.is_dir() {
                return Err(SdkError::invalid_working_directory(format!(
                    "{} is not a directory",
                    cwd.to_string_lossy()
                )));
            }
            command.current_dir(cwd);
        }

        // Spawn the process with detailed error context
        let mut child = command.spawn().map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    SdkError::transport(format!(
                        "CLI executable not found: {}. Please ensure the Claude Code CLI is installed and in your PATH.",
                        self.cli_path
                    ))
                }
                std::io::ErrorKind::PermissionDenied => {
                    SdkError::transport(format!(
                        "Permission denied executing CLI: {}. Please check file permissions.",
                        self.cli_path
                    ))
                }
                _ => SdkError::CliConnection(e)
            }
        })?;

        // Verify the process started successfully by checking if it's still running
        // after a brief moment (some processes fail immediately)
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited immediately - this is likely an error
                let mut stderr_content = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let _ = stderr.read_to_string(&mut stderr_content).await;
                }

                return Err(SdkError::process(
                    status.code(),
                    if stderr_content.is_empty() {
                        format!("CLI process exited immediately with status: {status:?}")
                    } else {
                        stderr_content
                    },
                ));
            }
            Ok(None) => {
                // Process is still running - good
            }
            Err(e) => {
                return Err(SdkError::CliConnection(e));
            }
        }

        // Set up streams with error handling
        let stdin = child.stdin.take().ok_or_else(|| {
            SdkError::transport(
                "Failed to get stdin handle from child process - this should not happen",
            )
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            SdkError::transport(
                "Failed to get stdout handle from child process - this should not happen",
            )
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            SdkError::transport(
                "Failed to get stderr handle from child process - this should not happen",
            )
        })?;

        self.stdin_stream = Some(stdin);
        self.stdout_stream = Some(BufReader::new(stdout));
        self.stderr_stream = Some(BufReader::new(stderr));
        self.process = Some(child);

        // Send initial prompt if it's text
        if let PromptInput::Text(text) = &self.prompt {
            let text = text.clone();
            if let Err(e) = self.send_text_prompt(&text).await {
                // If sending the prompt fails, clean up and return error
                let _ = self.disconnect().await;
                return Err(SdkError::stream_with_context(
                    format!("Failed to send initial prompt: {e}"),
                    "The CLI process may not be ready to receive input",
                ));
            }

            if self.close_stdin_after_prompt {
                if let Some(mut stdin) = self.stdin_stream.take() {
                    if let Err(e) = stdin.shutdown().await {
                        eprintln!("Warning: Failed to close stdin after prompt: {e}");
                    }
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

    /// Disconnect from the CLI process with graceful shutdown and timeout handling.
    ///
    /// This method implements a multi-stage shutdown process:
    /// 1. Close stdin to signal the process to exit
    /// 2. Wait for graceful exit with timeout
    /// 3. Send SIGTERM if graceful exit times out
    /// 4. Send SIGKILL if SIGTERM times out
    /// 5. Collect and report any stderr output
    pub async fn disconnect(&mut self) -> Result<()> {
        // Close stdin first to signal the process to exit gracefully
        if let Some(mut stdin) = self.stdin_stream.take() {
            let _ = stdin.shutdown().await;
        }

        // Handle process termination with multiple timeout stages
        if let Some(mut process) = self.process.take() {
            // Stage 1: Wait for graceful exit (5 seconds)
            let graceful_exit =
                tokio::time::timeout(std::time::Duration::from_secs(5), process.wait()).await;

            match graceful_exit {
                Ok(Ok(status)) => {
                    // Process exited - check exit status
                    if !status.success() {
                        let stderr = self.collect_stderr_final().await;
                        return Err(SdkError::process(status.code(), stderr));
                    }
                }
                Ok(Err(e)) => {
                    // Error waiting for process
                    let stderr = self.collect_stderr_final().await;
                    return Err(SdkError::stream_with_context(
                        format!("Error waiting for process: {e}"),
                        format!("stderr: {stderr}"),
                    ));
                }
                Err(_) => {
                    // Timeout - try SIGTERM first (Unix-like systems)
                    #[cfg(unix)]
                    {
                        // Send SIGTERM
                        if let Err(e) = self.send_signal(&mut process, libc::SIGTERM).await {
                            eprintln!("Warning: Failed to send SIGTERM: {e}");
                        }

                        // Wait for SIGTERM to take effect (3 seconds)
                        let sigterm_exit =
                            tokio::time::timeout(std::time::Duration::from_secs(3), process.wait())
                                .await;

                        match sigterm_exit {
                            Ok(Ok(status)) => {
                                // Process exited after SIGTERM
                                if !status.success() && status.code().is_none() {
                                    // Process was terminated by signal, which is expected
                                    let stderr = self.collect_stderr_final().await;
                                    if !stderr.is_empty() {
                                        eprintln!("Process stderr: {stderr}");
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                eprintln!("Error waiting for process after SIGTERM: {e}");
                            }
                            Err(_) => {
                                // SIGTERM timeout - force kill with SIGKILL
                                if let Err(e) = process.kill().await {
                                    eprintln!("Warning: Failed to kill process: {e}");
                                }

                                // Final wait with timeout
                                let _ = tokio::time::timeout(
                                    std::time::Duration::from_secs(2),
                                    process.wait(),
                                )
                                .await;
                            }
                        }
                    }

                    // For non-Unix systems or if Unix-specific handling fails
                    #[cfg(not(unix))]
                    {
                        // Force kill immediately
                        if let Err(e) = process.kill().await {
                            let stderr = self.collect_stderr_final().await;
                            return Err(SdkError::stream_with_context(
                                format!("Failed to kill process: {}", e),
                                format!("stderr: {}", stderr),
                            ));
                        }

                        // Wait for kill to take effect
                        let _ =
                            tokio::time::timeout(std::time::Duration::from_secs(2), process.wait())
                                .await;
                    }
                }
            }
        }

        // Clean up streams
        self.stdout_stream = None;
        self.stderr_stream = None;

        Ok(())
    }

    /// Send a signal to the process (Unix-like systems only).
    #[cfg(unix)]
    async fn send_signal(&self, process: &mut tokio::process::Child, signal: i32) -> Result<()> {
        if let Some(pid) = process.id() {
            unsafe {
                if libc::kill(pid as i32, signal) == -1 {
                    return Err(SdkError::transport(format!(
                        "Failed to send signal {signal} to process {pid}"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Collect final stderr output during shutdown.
    ///
    /// This method attempts to read any remaining stderr content
    /// that might contain important error information.
    async fn collect_stderr_final(&mut self) -> String {
        if let Some(ref mut stderr) = self.stderr_stream {
            let mut stderr_content = String::new();
            let mut line = String::new();

            // Try to read remaining stderr with a reasonable timeout
            let start_time = std::time::Instant::now();
            let max_duration = std::time::Duration::from_millis(500);

            while start_time.elapsed() < max_duration {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    stderr.read_line(&mut line),
                )
                .await
                {
                    Ok(Ok(0)) => break, // EOF
                    Ok(Ok(_)) => {
                        stderr_content.push_str(&line);
                        line.clear();

                        // Limit stderr collection
                        if stderr_content.len() > 10240 {
                            stderr_content.push_str("\n... (stderr truncated)");
                            break;
                        }
                    }
                    Ok(Err(_)) | Err(_) => break, // Error or timeout
                }
            }

            stderr_content
        } else {
            String::new()
        }
    }

    /// Collect stderr output for error reporting.
    ///
    /// This method attempts to read available stderr content with timeout
    /// to avoid blocking indefinitely while still capturing error information.
    #[allow(dead_code)]
    async fn collect_stderr(&mut self) -> Result<String> {
        if let Some(ref mut stderr) = self.stderr_stream {
            let mut stderr_content = String::new();
            let mut line = String::new();

            // Try to read available stderr content with timeout
            let start_time = std::time::Instant::now();
            let max_duration = std::time::Duration::from_millis(200);

            while start_time.elapsed() < max_duration {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    stderr.read_line(&mut line),
                )
                .await
                {
                    Ok(Ok(0)) => break, // EOF
                    Ok(Ok(_)) => {
                        stderr_content.push_str(&line);
                        line.clear();

                        // Limit stderr collection to prevent memory issues
                        if stderr_content.len() > 10240 {
                            // 10KB limit
                            stderr_content.push_str("\n... (stderr truncated)");
                            break;
                        }
                    }
                    Ok(Err(_)) | Err(_) => break, // Error or timeout
                }
            }

            Ok(stderr_content)
        } else {
            Ok(String::new())
        }
    }

    /// Receive messages from the CLI as an async stream.
    ///
    /// This method returns a stream that:
    /// - Handles robust JSON buffering for split and concatenated messages
    /// - Provides proper backpressure handling
    /// - Supports stream cancellation
    /// - Monitors stderr concurrently for error reporting
    /// - Implements memory protection with buffer size limits
    pub async fn receive_messages(&mut self) -> impl Stream<Item = Result<serde_json::Value>> + '_ {
        async_stream::stream! {
            if let Some(ref mut stdout) = self.stdout_stream {
                let mut buffer = JsonBuffer::new();
                let mut line = String::new();

                // We'll collect stderr synchronously when needed instead of spawning a task
                // to avoid lifetime issues with the async stream

                loop {
                    // Check if we have any parsed objects waiting first
                    while buffer.has_parsed_objects() {
                        if let Some(value) = buffer.try_parse_and_clear()? {
                            // Check if this is a control response and handle it separately
                            if Self::is_control_response(&value) {
                                if let Err(e) = Self::handle_control_response_with_pending(value, &self.pending_control_responses) {
                                    yield Err(e);
                                }
                                // Don't yield control responses to the main message stream
                            } else {
                                yield Ok(value);
                            }
                        } else {
                            break;
                        }
                    }

                    line.clear();

                    // Use select to handle both stdout reading and potential cancellation
                    tokio::select! {
                        read_result = stdout.read_line(&mut line) => {
                            match read_result {
                                Ok(0) => {
                                    // EOF reached - try to parse any remaining data
                                    if buffer.buffer_size() > 0 {
                                        // Try one final parse of remaining buffer content
                                        match buffer.try_parse_and_clear() {
                                            Ok(Some(value)) => {
                                                if Self::is_control_response(&value) {
                                                    if let Err(e) = Self::handle_control_response_with_pending(value, &self.pending_control_responses) {
                                                        yield Err(e);
                                                    }
                                                } else {
                                                    yield Ok(value);
                                                }
                                            }
                                            Ok(None) => {
                                                // Incomplete JSON at EOF - this might be an error
                                                if buffer.buffer_size() > 0 {
                                                    yield Err(SdkError::stream_with_context(
                                                        "Incomplete JSON data at end of stream",
                                                        format!("Buffer size: {} bytes", buffer.buffer_size())
                                                    ));
                                                }
                                            }
                                            Err(e) => yield Err(e),
                                        }
                                    }
                                    break;
                                }
                                Ok(bytes_read) => {
                                    // Data received - append to buffer and try parsing
                                    buffer.append(&line);

                                    // Parse all available complete JSON objects
                                    loop {
                                        match buffer.try_parse_and_clear() {
                                            Ok(Some(value)) => {
                                                if Self::is_control_response(&value) {
                                                    if let Err(e) = Self::handle_control_response_with_pending(value, &self.pending_control_responses) {
                                                        yield Err(e);
                                                    }
                                                } else {
                                                    yield Ok(value);
                                                }
                                            }
                                            Ok(None) => break, // No more complete objects
                                            Err(e) => {
                                                yield Err(e);
                                                break;
                                            }
                                        }
                                    }

                                    // Yield control to allow for cancellation and backpressure
                                    if bytes_read > 0 {
                                        tokio::task::yield_now().await;
                                    }
                                }
                                Err(e) => {
                                    // I/O error - try to collect stderr synchronously
                                    let stderr_content = self.collect_stderr_sync().await.unwrap_or_default();

                                    if !stderr_content.is_empty() {
                                        yield Err(SdkError::stream_with_context(
                                            format!("I/O error reading from CLI: {e}"),
                                            format!("stderr: {stderr_content}")
                                        ));
                                    } else {
                                        yield Err(SdkError::CliConnection(e));
                                    }
                                    break;
                                }
                            }
                        }
                        // Handle potential cancellation or other async operations
                        _ = tokio::time::sleep(std::time::Duration::from_millis(1)) => {
                            // This allows the stream to be responsive to cancellation
                            // while not busy-waiting
                            continue;
                        }
                    }
                }

                // No cleanup needed for stderr monitoring
            } else {
                yield Err(SdkError::transport("No stdout stream available"));
            }
        }
    }

    /// Collect stderr output synchronously with timeout.
    ///
    /// This method attempts to read available stderr content without blocking
    /// indefinitely, useful for error reporting when I/O errors occur.
    async fn collect_stderr_sync(&mut self) -> Result<String> {
        if let Some(ref mut stderr) = self.stderr_stream {
            let mut stderr_content = String::new();
            let mut line = String::new();

            // Read stderr with short timeout to avoid blocking
            while let Ok(Ok(bytes_read)) = tokio::time::timeout(
                std::time::Duration::from_millis(50),
                stderr.read_line(&mut line),
            )
            .await
            {
                if bytes_read == 0 {
                    break; // EOF
                }
                stderr_content.push_str(&line);
                line.clear();

                // Limit stderr collection to prevent memory issues
                if stderr_content.len() > 10240 {
                    // 10KB limit
                    stderr_content.push_str("\n... (stderr truncated)");
                    break;
                }
            }

            Ok(stderr_content)
        } else {
            Ok(String::new())
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
        let request_id_str = request_id.to_string();

        let control_request = serde_json::json!({
            "type": "control",
            "request_id": request_id_str,
            "action": "interrupt"
        });

        // Register the pending control request
        {
            let mut pending = self.pending_control_responses.lock().unwrap();
            pending.insert(request_id_str.clone(), serde_json::Value::Null);
        }

        if let Some(ref mut stdin) = self.stdin_stream {
            let request_str = serde_json::to_string(&control_request)?;
            stdin.write_all(request_str.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        } else {
            // Remove from pending if we can't send
            let mut pending = self.pending_control_responses.lock().unwrap();
            pending.remove(&request_id_str);
            return Err(SdkError::transport("No stdin stream available"));
        }

        // Wait for control response acknowledgment with timeout
        self.wait_for_control_response(&request_id_str).await
    }

    /// Wait for a control response with the given request ID.
    async fn wait_for_control_response(&self, request_id: &str) -> Result<()> {
        let timeout_duration = std::time::Duration::from_secs(5);
        let start_time = std::time::Instant::now();

        loop {
            // Check if we have a response
            {
                let mut pending = self.pending_control_responses.lock().unwrap();
                if let Some(response) = pending.remove(request_id) {
                    // We got a response - check if it indicates success
                    if response.is_null() {
                        // Still waiting
                    } else {
                        // Got actual response data
                        return Ok(());
                    }
                }
            }

            // Check timeout
            if start_time.elapsed() > timeout_duration {
                // Clean up pending request
                let mut pending = self.pending_control_responses.lock().unwrap();
                pending.remove(request_id);
                return Err(SdkError::control_timeout(
                    timeout_duration.as_millis() as u64
                ));
            }

            // Wait a bit before checking again
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    /// Check if a JSON value is a control response message.
    fn is_control_response(value: &serde_json::Value) -> bool {
        value.get("type").and_then(|t| t.as_str()) == Some("control_response")
            || (value.get("type").and_then(|t| t.as_str()) == Some("control")
                && value.get("request_id").is_some())
    }

    /// Handle a control response message.
    /// This should be called when a control response is received from the CLI.
    fn handle_control_response_with_pending(
        response: serde_json::Value,
        pending_responses: &Arc<Mutex<HashMap<String, serde_json::Value>>>,
    ) -> Result<()> {
        if let Some(request_id) = response.get("request_id").and_then(|id| id.as_str()) {
            let mut pending = pending_responses.lock().unwrap();
            if pending.contains_key(request_id) {
                pending.insert(request_id.to_string(), response);
                return Ok(());
            }
        }

        // Unknown or unexpected control response - log but don't error
        eprintln!(
            "Warning: Received unexpected control response: {response:?}"
        );
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
///
/// This buffer handles complex scenarios including:
/// - Split JSON messages across multiple reads
/// - Multiple concatenated JSON objects in a single read
/// - Partial JSON objects that need to be accumulated
/// - Memory protection with configurable size limits
struct JsonBuffer {
    buffer: String,
    max_size: usize,
    parsed_objects: Vec<serde_json::Value>,
}

impl JsonBuffer {
    const MAX_BUFFER_SIZE: usize = 1024 * 1024; // 1MB limit
    const DEFAULT_CAPACITY: usize = 8192; // 8KB initial capacity

    fn new() -> Self {
        Self {
            buffer: String::with_capacity(Self::DEFAULT_CAPACITY),
            max_size: Self::MAX_BUFFER_SIZE,
            parsed_objects: Vec::new(),
        }
    }

    #[allow(dead_code)]
    fn with_max_size(max_size: usize) -> Self {
        Self {
            buffer: String::with_capacity(Self::DEFAULT_CAPACITY.min(max_size)),
            max_size,
            parsed_objects: Vec::new(),
        }
    }

    /// Try to parse complete JSON objects from the buffer.
    ///
    /// This method handles multiple scenarios:
    /// 1. Single complete JSON object
    /// 2. Multiple concatenated JSON objects
    /// 3. Partial JSON objects that need more data
    /// 4. Mixed complete and partial objects
    fn try_parse_and_clear(&mut self) -> Result<Option<serde_json::Value>> {
        // Return any previously parsed objects first
        if !self.parsed_objects.is_empty() {
            return Ok(Some(self.parsed_objects.remove(0)));
        }

        if self.buffer.len() > self.max_size {
            return Err(SdkError::buffer_size_exceeded(self.max_size));
        }

        if self.buffer.trim().is_empty() {
            return Ok(None);
        }

        // Try to parse multiple JSON objects from the buffer
        self.parse_multiple_objects()?;

        // Return the first parsed object if any
        if !self.parsed_objects.is_empty() {
            Ok(Some(self.parsed_objects.remove(0)))
        } else {
            Ok(None)
        }
    }

    /// Parse multiple JSON objects from the buffer.
    ///
    /// This handles cases where multiple JSON objects are concatenated
    /// in the buffer, separated by whitespace or newlines.
    fn parse_multiple_objects(&mut self) -> Result<()> {
        let mut remaining = self.buffer.trim();
        let mut consumed_bytes = 0;

        while !remaining.is_empty() {
            // Find the end of the next JSON object
            match self.find_json_object_end(remaining) {
                Some(end_pos) => {
                    let json_str = &remaining[..end_pos];

                    // Try to parse this JSON object
                    match serde_json::from_str(json_str) {
                        Ok(value) => {
                            self.parsed_objects.push(value);
                            consumed_bytes += end_pos;

                            // Move to the next part of the buffer
                            remaining = remaining[end_pos..].trim_start();

                            // Account for whitespace we trimmed
                            while consumed_bytes < self.buffer.len()
                                && self
                                    .buffer
                                    .chars()
                                    .nth(consumed_bytes)
                                    .is_some_and(|c| c.is_whitespace())
                            {
                                consumed_bytes += 1;
                            }
                        }
                        Err(_) => {
                            // This JSON object is incomplete, stop parsing
                            break;
                        }
                    }
                }
                None => {
                    // No complete JSON object found, try parsing the entire remaining buffer
                    match serde_json::from_str(remaining) {
                        Ok(value) => {
                            self.parsed_objects.push(value);
                            consumed_bytes = self.buffer.len();
                            break;
                        }
                        Err(_) => {
                            // Incomplete JSON, keep it in buffer
                            break;
                        }
                    }
                }
            }
        }

        // Remove consumed data from buffer
        if consumed_bytes > 0 {
            self.buffer.drain(..consumed_bytes);
        }

        Ok(())
    }

    /// Find the end position of a JSON object in the string.
    ///
    /// This uses a simple bracket/brace counting approach to find
    /// where a JSON object ends, handling nested structures.
    fn find_json_object_end(&self, s: &str) -> Option<usize> {
        let chars = s.char_indices();
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut started = false;

        for (i, ch) in chars {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => {
                    escape_next = true;
                }
                '"' => {
                    in_string = !in_string;
                }
                '{' | '[' if !in_string => {
                    depth += 1;
                    started = true;
                }
                '}' | ']' if !in_string => {
                    depth -= 1;
                    if started && depth == 0 {
                        return Some(i + 1);
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn append(&mut self, data: &str) {
        // Check if adding this data would exceed the limit
        if self.buffer.len() + data.len() > self.max_size {
            // Try to make room by parsing what we can first
            let _ = self.parse_multiple_objects();

            // If still too large after parsing, we'll let try_parse_and_clear handle the error
        }

        self.buffer.push_str(data);
    }

    /// Get the current buffer size for monitoring.
    fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Check if there are parsed objects waiting to be consumed.
    fn has_parsed_objects(&self) -> bool {
        !self.parsed_objects.is_empty()
    }

    /// Clear all buffered data and parsed objects.
    #[allow(dead_code)]
    fn clear(&mut self) {
        self.buffer.clear();
        self.parsed_objects.clear();
    }
}
