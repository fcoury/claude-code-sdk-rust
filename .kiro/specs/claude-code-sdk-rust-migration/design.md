# Design Document

## Overview

The Claude Code SDK for Rust will provide a high-performance, memory-safe implementation that maintains full API compatibility with the Python SDK. The design follows Rust idioms while preserving the familiar patterns developers expect from the Python version. The architecture consists of four main layers: types, transport, client, and public API, with comprehensive error handling throughout.

The SDK supports two primary usage patterns:
1. **One-shot queries** via the `query()` function for simple, stateless interactions
2. **Interactive sessions** via the `ClaudeSDKClient` for bidirectional, stateful conversations

## Architecture

### Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Public API Layer                         │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │   query()       │    │    ClaudeSDKClient              │ │
│  │   function      │    │    (Interactive)                │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                   Client Layer                              │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              InternalClient                             │ │
│  │         (Connection Management)                         │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                  Transport Layer                            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │           SubprocessCliTransport                        │ │
│  │    (Process Management & I/O Handling)                 │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                    Foundation Layer                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │   Types     │ │   Errors    │ │   Message Parser        │ │
│  │  (Serde)    │ │ (thiserror) │ │   (JSON Handling)       │ │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Async Architecture

The SDK leverages Rust's async ecosystem with tokio as the runtime:

- **Streams**: All message flows use `tokio_stream::Stream` for consistent async iteration
- **Process Management**: `tokio::process::Command` for subprocess handling
- **I/O**: `tokio::io` for non-blocking stdin/stdout/stderr operations
- **Concurrency**: Task spawning for concurrent stderr monitoring and message streaming

## Components and Interfaces

### 1. Type System (`src/types.rs`)

#### Core Message Types

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: MessageContent, // Preserves Python API compatibility - can be string or blocks
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMessage {
    pub subtype: String,
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultMessage {
    pub subtype: String,
    pub duration_ms: i64,
    pub duration_api_ms: i64,
    pub is_error: bool,
    pub num_turns: i32,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Result(ResultMessage),
}
```

#### Content Block System

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Structured(Vec<HashMap<String, serde_json::Value>>),
}
```

#### Configuration with Builder Pattern

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PermissionMode {
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "acceptEdits")]
    AcceptEdits,
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServerConfig {
    Stdio {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        args: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    Sse {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
    Http {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptions {
    pub allowed_tools: Vec<String>,
    pub max_thinking_tokens: i32,
    pub system_prompt: Option<String>,
    pub append_system_prompt: Option<String>,
    pub mcp_tools: Vec<String>,
    pub mcp_servers: HashMap<String, McpServerConfig>,
    pub permission_mode: Option<PermissionMode>,
    pub continue_conversation: bool,
    pub resume: Option<String>,
    pub max_turns: Option<i32>,
    pub disallowed_tools: Vec<String>,
    pub model: Option<String>,
    pub permission_prompt_tool_name: Option<String>,
    pub cwd: Option<PathBuf>,
    pub settings: Option<String>,
}

impl ClaudeCodeOptions {
    pub fn builder() -> ClaudeCodeOptionsBuilder {
        ClaudeCodeOptionsBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClaudeCodeOptionsBuilder {
    inner: ClaudeCodeOptions,
}

impl ClaudeCodeOptionsBuilder {
    pub fn allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.inner.allowed_tools = tools;
        self
    }
    
    pub fn system_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.inner.system_prompt = Some(prompt.into());
        self
    }
    
    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.inner.permission_mode = Some(mode);
        self
    }
    
    pub fn cwd<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.inner.cwd = Some(path.into());
        self
    }
    
    // ... additional builder methods
    
    pub fn build(self) -> ClaudeCodeOptions {
        self.inner
    }
}
```

### 2. Error System (`src/errors.rs`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SdkError {
    #[error("Claude Code CLI not found. Please ensure it is installed and in your PATH.\n\nInstall with: npm install -g @anthropic-ai/claude-code")]
    CliNotFound(#[from] which::Error),

    #[error("Failed to connect to or start the Claude Code CLI process: {0}")]
    CliConnection(#[from] std::io::Error),

    #[error("The CLI process failed with exit code {exit_code:?}: {stderr}")]
    Process {
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Failed to decode JSON from CLI output: {0}")]
    JsonDecode(#[from] serde_json::Error),

    #[error("Failed to parse message from data: {message}")]
    MessageParse {
        message: String,
        data: serde_json::Value,
    },

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Invalid working directory: {path}")]
    InvalidWorkingDirectory { path: String },

    #[error("Buffer size exceeded maximum limit of {limit} bytes")]
    BufferSizeExceeded { limit: usize },
}

pub type Result<T> = std::result::Result<T, SdkError>;
```

### 3. Transport Layer (`src/transport.rs`)

#### CLI Discovery and Process Management

```rust
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
    pub fn new(
        prompt: PromptInput,
        options: ClaudeCodeOptions,
        cli_path: Option<PathBuf>,
        close_stdin_after_prompt: bool,
    ) -> Result<Self> {
        let cli_path = cli_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| Self::find_cli())?;
            
        Ok(Self {
            prompt,
            options,
            cli_path,
            process: None,
            stdout_stream: None,
            stderr_stream: None,
            stdin_stream: None,
            is_streaming: matches!(prompt, PromptInput::Stream(_)),
            close_stdin_after_prompt,
            request_counter: AtomicU64::new(0),
            pending_control_responses: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn find_cli() -> Result<String> {
        // Use which crate to find claude binary
        // Check standard locations: ~/.npm-global/bin, /usr/local/bin, etc.
        // Provide helpful error messages for Node.js not installed
    }

    fn build_command(&self) -> Vec<String> {
        // Build CLI command with all options
        // Handle streaming vs string mode
        // Convert ClaudeCodeOptions to CLI arguments
    }

    pub async fn connect(&mut self) -> Result<()> {
        // Spawn subprocess using tokio::process::Command
        // Set up stdin/stdout/stderr streams
        // Handle streaming mode initialization
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        // Gracefully terminate subprocess
        // Clean up resources
        // Handle timeout and force kill if necessary
    }

    pub async fn receive_messages(&mut self) -> impl Stream<Item = Result<serde_json::Value>> {
        // Return async stream of parsed JSON messages
        // Handle buffering for split/concatenated JSON
        // Manage stderr collection concurrently
    }

    pub async fn send_request(&mut self, messages: Vec<serde_json::Value>, options: HashMap<String, serde_json::Value>) -> Result<()> {
        // Send messages to stdin in streaming mode
        // Handle message formatting and serialization
    }

    pub async fn interrupt(&mut self) -> Result<()> {
        // Send interrupt control request
        // Wait for acknowledgment response
    }
}

#[derive(Debug)]
pub enum PromptInput {
    Text(String),
    Stream(Pin<Box<dyn Stream<Item = serde_json::Value> + Send>>),
}
```

#### JSON Buffering Strategy

The transport implements sophisticated JSON buffering to handle the CLI's output patterns:

```rust
const MAX_BUFFER_SIZE: usize = 1024 * 1024; // 1MB limit

struct JsonBuffer {
    buffer: String,
    max_size: usize,
}

impl JsonBuffer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            max_size: MAX_BUFFER_SIZE,
        }
    }

    fn try_parse_and_clear(&mut self) -> Result<Option<serde_json::Value>> {
        if self.buffer.len() > self.max_size {
            return Err(SdkError::BufferSizeExceeded { 
                limit: self.max_size 
            });
        }

        match serde_json::from_str(&self.buffer) {
            Ok(value) => {
                self.buffer.clear();
                Ok(Some(value))
            }
            Err(_) => Ok(None), // Not yet complete JSON
        }
    }

    fn append(&mut self, data: &str) {
        self.buffer.push_str(data);
    }
}
```

### 4. Message Parser (`src/message_parser.rs`)

```rust
pub fn parse_message(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object()
        .ok_or_else(|| SdkError::MessageParse {
            message: "Expected JSON object".to_string(),
            data: data.clone(),
        })?;

    let message_type = obj.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::MessageParse {
            message: "Missing or invalid 'type' field".to_string(),
            data: data.clone(),
        })?;

    match message_type {
        "user" => parse_user_message(obj, &data),
        "assistant" => parse_assistant_message(obj, &data),
        "system" => parse_system_message(obj, &data),
        "result" => parse_result_message(obj, &data),
        _ => Err(SdkError::MessageParse {
            message: format!("Unknown message type: {}", message_type),
            data,
        }),
    }
}

fn parse_user_message(obj: &serde_json::Map<String, serde_json::Value>, data: &serde_json::Value) -> Result<Message> {
    // Parse user message with content handling
}

fn parse_assistant_message(obj: &serde_json::Map<String, serde_json::Value>, data: &serde_json::Value) -> Result<Message> {
    // Parse assistant message with content blocks
}

// Additional parsing functions...
```

### 5. Client Layer (`src/client.rs`)

#### Internal Client

```rust
pub struct InternalClient;

impl InternalClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn process_query(
        &self,
        prompt: PromptInput,
        options: ClaudeCodeOptions,
    ) -> impl Stream<Item = Result<Message>> {
        async_stream::stream! {
            let mut transport = SubprocessCliTransport::new(
                prompt,
                options,
                None,
                true, // close_stdin_after_prompt for one-shot queries
            )?;

            transport.connect().await?;

            let mut message_stream = transport.receive_messages().await;
            while let Some(data_result) = message_stream.next().await {
                match data_result {
                    Ok(data) => {
                        match parse_message(data) {
                            Ok(message) => yield Ok(message),
                            Err(e) => yield Err(e),
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }

            transport.disconnect().await?;
        }
    }
}
```

#### Interactive Client

```rust
pub struct ClaudeSDKClient {
    options: ClaudeCodeOptions,
    transport: Option<SubprocessCliTransport>,
}

impl ClaudeSDKClient {
    pub fn new(options: Option<ClaudeCodeOptions>) -> Self {
        Self {
            options: options.unwrap_or_default(),
            transport: None,
        }
    }

    pub async fn connect(&mut self, prompt: Option<PromptInput>) -> Result<()> {
        let prompt = prompt.unwrap_or_else(|| {
            // Create pending stream that never yields for interactive mode
            PromptInput::Stream(Box::pin(tokio_stream::pending()))
        });

        let mut transport = SubprocessCliTransport::new(
            prompt,
            self.options.clone(),
            None,
            false, // Keep stdin open for interactive mode
        )?;

        transport.connect().await?;
        self.transport = Some(transport);
        Ok(())
    }

    pub async fn query(&mut self, prompt: PromptInput, session_id: Option<String>) -> Result<()> {
        let transport = self.transport.as_mut()
            .ok_or_else(|| SdkError::Transport("Not connected".to_string()))?;

        let session_id = session_id.unwrap_or_else(|| "default".to_string());
        
        match prompt {
            PromptInput::Text(text) => {
                let message = serde_json::json!({
                    "type": "user",
                    "message": {
                        "role": "user",
                        "content": text
                    },
                    "parent_tool_use_id": null,
                    "session_id": session_id
                });
                transport.send_request(vec![message], HashMap::new()).await
            }
            PromptInput::Stream(stream) => {
                // Handle async iterable prompts
                // Convert stream items and send to transport
                todo!("Implement stream handling")
            }
        }
    }

    pub async fn receive_messages(&mut self) -> Result<impl Stream<Item = Result<Message>>> {
        let transport = self.transport.as_mut()
            .ok_or_else(|| SdkError::Transport("Not connected".to_string()))?;

        let message_stream = transport.receive_messages().await;
        Ok(message_stream.map(|data_result| {
            data_result.and_then(parse_message)
        }))
    }

    pub async fn receive_response(&mut self) -> Result<impl Stream<Item = Result<Message>>> {
        let mut messages = self.receive_messages().await?;
        Ok(async_stream::stream! {
            while let Some(message_result) = messages.next().await {
                match message_result {
                    Ok(message) => {
                        let is_result = matches!(message, Message::Result(_));
                        yield Ok(message);
                        if is_result {
                            break;
                        }
                    }
                    Err(e) => yield Err(e),
                }
            }
        })
    }

    pub async fn interrupt(&mut self) -> Result<()> {
        let transport = self.transport.as_mut()
            .ok_or_else(|| SdkError::Transport("Not connected".to_string()))?;
        transport.interrupt().await
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut transport) = self.transport.take() {
            transport.disconnect().await?;
        }
        Ok(())
    }
}

impl Drop for ClaudeSDKClient {
    fn drop(&mut self) {
        if self.transport.is_some() {
            // Best-effort cleanup: spawn a task to handle async cleanup
            // Note: This is not guaranteed to complete if the main program exits immediately
            // Users should call disconnect() explicitly for guaranteed cleanup
            tokio::spawn(async move {
                // Cleanup logic - terminate child process if still running
            });
        }
    }
}
```

### 6. Public API (`src/lib.rs`)

```rust
pub async fn query<S: AsRef<str>>(
    prompt: S,
    options: Option<ClaudeCodeOptions>,
) -> impl Stream<Item = Result<Message>> {
    let options = options.unwrap_or_default();
    let client = InternalClient::new();
    client.process_query(PromptInput::Text(prompt.as_ref().to_string()), options).await
}

// Re-exports
pub use client::ClaudeSDKClient;
pub use errors::{SdkError, Result};
pub use types::*;
```

## Data Models

### Message Flow Architecture

```
User Input → PromptInput → Transport → JSON Stream → Message Parser → Typed Messages
     ↓              ↓           ↓            ↓              ↓              ↓
   String      Text/Stream   CLI Process   Raw JSON    parse_message()  Message Enum
```

### State Management

The SDK maintains state at different levels:

1. **Transport State**: Process handles, streams, connection status
2. **Client State**: Transport instance, configuration, session management
3. **Message State**: Parsing buffers, control request tracking

### Concurrency Model

- **Single-threaded async**: All operations use async/await without thread spawning
- **Concurrent I/O**: Stdout and stderr are read concurrently using tokio tasks
- **Stream-based**: All message flows use async streams for backpressure handling
- **Resource cleanup**: Automatic cleanup via Drop trait and async context managers

## Error Handling

### Error Propagation Strategy

1. **Transport Errors**: I/O failures, process termination, JSON parsing
2. **Client Errors**: Connection state, invalid operations
3. **Message Errors**: Parsing failures, unknown message types
4. **Configuration Errors**: Invalid paths, missing CLI, bad options

### Error Recovery

- **Automatic retry**: Not implemented (matches Python SDK behavior)
- **Graceful degradation**: Capture stderr for debugging
- **Resource cleanup**: Ensure processes are terminated on errors via Drop trait (best-effort) and explicit disconnect() calls (guaranteed)
- **User guidance**: Provide actionable error messages with installation instructions

## Testing Strategy

### Unit Tests

- **Type serialization/deserialization**: Verify serde implementations
- **Builder pattern**: Test ClaudeCodeOptions builder methods
- **Error handling**: Test all error variants and messages
- **Message parsing**: Test parse_message with various inputs

### Integration Tests

- **Mock CLI**: Create test harness that simulates claude-code CLI
- **Buffering scenarios**: Test JSON splitting, concatenation, large messages
- **Process lifecycle**: Test connect, disconnect, error handling
- **Stream handling**: Test async stream behavior and cancellation

### Property-Based Tests

- **JSON roundtrip**: Generate random messages and verify serialization
- **Buffer handling**: Test with various buffer sizes and split patterns
- **Error conditions**: Generate invalid inputs and verify error handling

### Performance Tests

- **Memory usage**: Verify no memory leaks in long-running sessions
- **Throughput**: Compare performance with Python SDK
- **Latency**: Measure message parsing and transport overhead

The testing strategy will use `rstest` for parameterized tests and `tokio-test` for async test utilities, closely mirroring the Python test suite structure and coverage.

## Design Refinements and Considerations

### API Ergonomics

The design prioritizes both Python API compatibility and Rust ergonomics:

1. **Flexible String Handling**: The `query()` function accepts `AsRef<str>` to work with `&str`, `String`, and other string types
2. **Pending Streams**: Uses `tokio_stream::pending()` for cleaner empty stream creation in interactive mode
3. **Builder Pattern**: Provides ergonomic configuration while maintaining type safety

### Resource Management Strategy

The design implements a two-tier cleanup approach:

1. **Primary**: Explicit `disconnect()` calls provide guaranteed resource cleanup
2. **Fallback**: `Drop` trait implementation provides best-effort cleanup with documented limitations
3. **Documentation**: Clear guidance on when to use each approach

### Type System Design Decisions

1. **Message Structure**: Maintains nested structure (`Message::User(UserMessage)`) for clear separation of concerns and Python compatibility
2. **Content Flexibility**: Preserves `MessageContent` enum to handle both string and block-based content as per Python API
3. **Serde Integration**: Uses tagged enums and proper serialization attributes for robust JSON handling

### Performance Considerations

1. **Stream-based Architecture**: Consistent use of async streams for memory efficiency and backpressure handling
2. **Zero-copy Where Possible**: Minimizes string allocations in JSON parsing paths
3. **Concurrent I/O**: Separate tasks for stdout/stderr to prevent blocking
4. **Buffer Management**: Configurable buffer limits to prevent memory exhaustion

### Error Handling Philosophy

The error system balances comprehensive coverage with usability:

1. **Actionable Messages**: All errors include context and suggested remediation
2. **Structured Data**: Parse errors include original data for debugging
3. **Installation Guidance**: CLI discovery errors provide step-by-step installation instructions
4. **Graceful Degradation**: Capture and report stderr even when processes succeed