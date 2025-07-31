//! # Claude Code SDK for Rust
//!
//! This crate provides a Rust SDK for interacting with the Claude Code CLI.
//! It supports both one-shot queries and interactive bidirectional conversations.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use claude_code_sdk::{query, ClaudeCodeOptions};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut stream = query("Hello, Claude!", None).await;
//!     
//!     while let Some(message) = stream.next().await {
//!         match message? {
//!             claude_code_sdk::Message::Assistant(msg) => {
//!                 println!("Claude: {:?}", msg.content);
//!             }
//!             claude_code_sdk::Message::Result(result) => {
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
//! ```rust,no_run
//! use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = ClaudeSDKClient::new(None);
//!     client.connect(None).await?;
//!     
//!     // Send a message and receive responses
//!     client.query("Hello, Claude!".into(), None).await?;
//!     let mut responses = client.receive_response().await?;
//!     
//!     while let Some(message) = responses.next().await {
//!         // Handle messages...
//!     }
//!     
//!     client.disconnect().await?;
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod errors;
pub mod message_parser;
pub mod transport;
pub mod types;

// Re-export public API
pub use client::{ClaudeSDKClient, InternalClient};
pub use errors::{Result, SdkError};
pub use types::*;

use tokio_stream::Stream;
use transport::PromptInput;

/// Execute a one-shot query to Claude Code CLI.
///
/// This function provides a simple interface for sending a single prompt to Claude
/// and receiving a stream of response messages. The transport connection is automatically
/// managed and cleaned up after the query completes.
///
/// # Arguments
///
/// * `prompt` - The prompt text to send to Claude
/// * `options` - Optional configuration for the query
///
/// # Returns
///
/// Returns an async stream of `Message` results. The stream will yield various message
/// types including `AssistantMessage` for Claude's responses and `ResultMessage` when
/// the query completes.
///
/// # Example
///
/// ```rust,no_run
/// use claude_code_sdk::{query, ClaudeCodeOptions};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> claude_code_sdk::Result<()> {
///     let mut stream = query("What is Rust?", None).await;
///     
///     while let Some(message) = stream.next().await {
///         match message? {
///             claude_code_sdk::Message::Assistant(msg) => {
///                 println!("Response: {:?}", msg.content);
///             }
///             claude_code_sdk::Message::Result(_) => break,
///             _ => {}
///         }
///     }
///     
///     Ok(())
/// }
/// ```
pub async fn query<S: AsRef<str>>(
    prompt: S,
    options: Option<ClaudeCodeOptions>,
) -> impl Stream<Item = Result<Message>> {
    let options = options.unwrap_or_default();
    let client = client::InternalClient::new();
    client
        .process_query(
            PromptInput::Text(prompt.as_ref().to_string()),
            options,
        )
        .await
}