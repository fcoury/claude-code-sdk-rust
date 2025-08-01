//! # Claude Code SDK for Rust
//!
//! A high-performance, memory-safe Rust SDK for interacting with the Claude Code CLI.
//! This crate provides both one-shot queries and interactive bidirectional conversations
//! with Claude, leveraging Rust's type safety and async ecosystem.
//!
//! ## Features
//!
//! - **Type Safety**: Strongly-typed message and configuration structures with compile-time guarantees
//! - **Async Streams**: Native integration with Rust's async ecosystem using `tokio_stream::Stream`
//! - **Memory Safety**: Automatic resource management with explicit cleanup options
//! - **Error Handling**: Comprehensive error types with actionable error messages
//! - **Flexible Configuration**: Builder pattern for ergonomic configuration
//! - **CLI Integration**: Automatic CLI discovery with helpful installation guidance
//!
//! ## Quick Start
//!
//! For simple one-shot queries, use the [`query`] function:
//!
//! ```rust,no_run
//! use claude_code_sdk::{query, ClaudeCodeOptions, Message};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> claude_code_sdk::Result<()> {
//!     // Simple query with default options
//!     let stream = query("Hello, Claude!", None).await;
//!     tokio::pin!(stream);
//!     
//!     while let Some(message) = stream.next().await {
//!         match message? {
//!             Message::Assistant(msg) => {
//!                 println!("Claude: {:?}", msg.content);
//!             }
//!             Message::Result(result) => {
//!                 println!("Query completed in {}ms", result.duration_ms);
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Interactive Mode
//!
//! For bidirectional conversations, use [`ClaudeSDKClient`]:
//!
//! ```rust,no_run
//! use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions, PromptInput, Message};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> claude_code_sdk::Result<()> {
//!     let mut client = ClaudeSDKClient::new(None);
//!     client.connect(None).await?;
//!     
//!     // Send a message and receive responses
//!     client.query(PromptInput::from("Hello, Claude!"), None).await?;
//!     {
//!         let responses = client.receive_response().await?;
//!         tokio::pin!(responses);
//!         
//!         while let Some(message) = responses.next().await {
//!             match message? {
//!                 Message::Assistant(msg) => {
//!                     println!("Claude: {:?}", msg.content);
//!                 }
//!                 Message::Result(_) => break,
//!                 _ => {}
//!             }
//!         }
//!     }
//!     
//!     // Always disconnect explicitly for guaranteed cleanup
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! Use the builder pattern for advanced configuration:
//!
//! ```rust,no_run
//! use claude_code_sdk::{query, ClaudeCodeOptions, PermissionMode};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> claude_code_sdk::Result<()> {
//!     let options = ClaudeCodeOptions::builder()
//!         .system_prompt("You are a helpful coding assistant")
//!         .permission_mode(PermissionMode::AcceptEdits)
//!         .cwd(PathBuf::from("./my-project"))
//!         .allowed_tools(vec!["file_editor".to_string(), "bash".to_string()])
//!         .build();
//!     
//!     let stream = query("Help me refactor this code", Some(options)).await;
//!     // Process stream...
//!     # Ok(())
//! }
//! ```
//!
//! ## Error Handling
//!
//! All operations return [`Result<T>`] with detailed [`SdkError`] information:
//!
//! ```rust,no_run
//! use claude_code_sdk::{query, SdkError, Message};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() {
//!     let stream = query("Hello", None).await;
//!     tokio::pin!(stream);
//!     
//!     while let Some(message_result) = stream.next().await {
//!         match message_result {
//!             Ok(Message::Assistant(msg)) => {
//!                 println!("Claude: {:?}", msg.content);
//!             }
//!             Ok(Message::Result(_)) => {
//!                 println!("Query completed");
//!                 break;
//!             }
//!             Err(SdkError::CliNotFound(_)) => {
//!                 eprintln!("Please install the Claude Code CLI:");
//!                 eprintln!("npm install -g @anthropic-ai/claude-code");
//!                 break;
//!             }
//!             Err(e) => {
//!                 eprintln!("Error: {}", e);
//!                 if e.is_recoverable() {
//!                     eprintln!("This error might be recoverable by retrying");
//!                 }
//!                 break;
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! ```
//!
//! ## Requirements
//!
//! - **Node.js**: Required for running the Claude Code CLI
//! - **Claude Code CLI**: Install with `npm install -g @anthropic-ai/claude-code`
//! - **Tokio Runtime**: This crate requires a tokio async runtime
//!
//! ## Module Organization
//!
//! - [`client`]: Client implementations for one-shot and interactive queries
//! - [`errors`]: Error types and handling utilities
//! - [`types`]: Message types, configuration, and data structures
//! - [`transport`]: Low-level CLI communication and process management
//! - [`message_parser`]: JSON message parsing and validation

// Public modules - these contain implementation details but some types are re-exported
pub mod client;
pub mod errors;
pub mod message_parser;
pub mod transport;
pub mod types;

// Core public API re-exports
// These are the main types and functions users should interact with

/// Client for interactive bidirectional conversations with Claude.
///
/// See [`ClaudeSDKClient`] for detailed documentation.
pub use client::ClaudeSDKClient;

/// Result type alias for all SDK operations.
///
/// This is equivalent to `std::result::Result<T, SdkError>`.
pub use errors::{Result, SdkError};

/// Input type for prompts, supporting both text and async streams.
///
/// See [`PromptInput`] for detailed documentation.
pub use transport::PromptInput;

// Message and configuration types
pub use types::{
    AssistantMessage,
    // Configuration types
    ClaudeCodeOptions,
    ClaudeCodeOptionsBuilder,
    ContentBlock,
    McpServerConfig,
    // Core message types
    Message,
    // Content types
    MessageContent,
    PermissionMode,
    ResultMessage,
    SystemMessage,
    TextBlock,
    ToolResultBlock,
    ToolResultContent,
    ToolUseBlock,
    UserMessage,
};

// Internal client is not re-exported as it's only used by the query function

use tokio_stream::Stream;

/// Execute a one-shot query to Claude Code CLI.
///
/// This function provides a simple, ergonomic interface for sending a single prompt to Claude
/// and receiving a stream of response messages. The transport connection is automatically
/// managed and cleaned up after the query completes, making it ideal for simple interactions
/// that don't require session state.
///
/// # Arguments
///
/// * `prompt` - The prompt text to send to Claude. Accepts any type that implements `AsRef<str>`,
///   including `&str`, `String`, and other string-like types for maximum flexibility.
/// * `options` - Optional configuration for the query. If `None`, default options are used.
///   Use [`ClaudeCodeOptions::builder()`] to create custom configurations.
///
/// # Returns
///
/// Returns an async stream of [`Message`] results. The stream will yield various message
/// types in sequence:
/// - [`Message::System`]: Metadata and control information
/// - [`Message::Assistant`]: Claude's response content (may be multiple messages)
/// - [`Message::Result`]: Final result with completion metadata (always last)
///
/// Each item in the stream is a [`Result<Message>`], allowing for error handling at the
/// message level. The stream automatically terminates after the [`Message::Result`] is yielded.
///
/// # Errors
///
/// This function can return various errors through the stream items:
/// - [`SdkError::CliNotFound`]: Claude Code CLI is not installed or not in PATH
/// - [`SdkError::NodeJsNotFound`]: Node.js runtime is not available
/// - [`SdkError::Process`]: CLI process failed or returned non-zero exit code
/// - [`SdkError::JsonDecode`]: Failed to parse JSON from CLI output
/// - [`SdkError::MessageParse`]: Failed to parse message structure
/// - [`SdkError::Transport`]: Communication errors with the CLI process
/// - [`SdkError::InvalidWorkingDirectory`]: Specified working directory is invalid
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,no_run
/// use claude_code_sdk::{query, Message};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> claude_code_sdk::Result<()> {
///     let stream = query("What is Rust?", None).await;
///     tokio::pin!(stream);
///     
///     while let Some(message) = stream.next().await {
///         match message? {
///             Message::Assistant(msg) => {
///                 println!("Response: {:?}", msg.content);
///             }
///             Message::Result(result) => {
///                 println!("Query completed in {}ms", result.duration_ms);
///                 break;
///             }
///             _ => {} // Handle other message types as needed
///         }
///     }
///     
///     Ok(())
/// }
/// ```
///
/// ## With Custom Configuration
///
/// ```rust,no_run
/// use claude_code_sdk::{query, ClaudeCodeOptions, PermissionMode};
/// use tokio_stream::StreamExt;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> claude_code_sdk::Result<()> {
///     let options = ClaudeCodeOptions::builder()
///         .system_prompt("You are a helpful coding assistant")
///         .permission_mode(PermissionMode::AcceptEdits)
///         .cwd(PathBuf::from("./my-project"))
///         .allowed_tools(vec!["file_editor".to_string()])
///         .build();
///     
///     let stream = query("Help me refactor this code", Some(options)).await;
///     tokio::pin!(stream);
///     
///     while let Some(message) = stream.next().await {
///         // Process messages...
///         # break;
///     }
///     
///     Ok(())
/// }
/// ```
///
/// ## Error Handling
///
/// ```rust,no_run
/// use claude_code_sdk::{query, SdkError};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() {
///     let stream = query("Hello, Claude!", None).await;
///     tokio::pin!(stream);
///     
///     while let Some(message_result) = stream.next().await {
///         match message_result {
///             Ok(message) => {
///                 // Process successful message
///                 println!("Received: {:?}", message);
///             }
///             Err(SdkError::CliNotFound(_)) => {
///                 eprintln!("Please install Claude Code CLI:");
///                 eprintln!("npm install -g @anthropic-ai/claude-code");
///                 break;
///             }
///             Err(e) => {
///                 eprintln!("Error: {}", e);
///                 if e.is_recoverable() {
///                     eprintln!("This error might be recoverable");
///                 }
///                 break;
///             }
///         }
///     }
/// }
/// ```
///
/// ## Flexible String Types
///
/// ```rust,no_run
/// use claude_code_sdk::query;
///
/// #[tokio::main]
/// async fn main() -> claude_code_sdk::Result<()> {
///     // All of these work due to AsRef<str>
///     let _stream1 = query("string literal", None).await;
///     let _stream2 = query(String::from("owned string"), None).await;
///     let owned_string = "reference to string".to_string();
///     let _stream3 = query(&owned_string, None).await;
///     
///     Ok(())
/// }
/// ```
///
/// # Performance Notes
///
/// - The function creates a new CLI process for each query, which has startup overhead
/// - For multiple queries or interactive sessions, consider using [`ClaudeSDKClient`] instead
/// - The stream uses minimal buffering and processes messages as they arrive
/// - Resource cleanup is automatic and happens when the stream completes or is dropped
///
/// # Thread Safety
///
/// This function is thread-safe and can be called concurrently from multiple tasks.
/// Each call creates an independent CLI process and stream.
pub async fn query<S: AsRef<str>>(
    prompt: S,
    options: Option<ClaudeCodeOptions>,
) -> impl Stream<Item = Result<Message>> {
    let options = options.unwrap_or_default();
    let client = client::InternalClient::new();
    client
        .process_query(
            transport::PromptInput::Text(prompt.as_ref().to_string()),
            options,
        )
        .await
}
