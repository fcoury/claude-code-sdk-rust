//! Client implementations for interacting with Claude Code.
//!
//! This module provides both internal client for one-shot queries and
//! the public interactive client for bidirectional conversations.

use crate::errors::{Result, SdkError};
use crate::message_parser::parse_message;
use crate::transport::{PromptInput, SubprocessCliTransport};
use crate::types::{ClaudeCodeOptions, Message};
use async_stream;
use std::collections::HashMap;

use tokio_stream::{Stream, StreamExt};

/// Internal client for processing one-shot queries.
pub struct InternalClient;

impl InternalClient {
    /// Create a new internal client.
    pub fn new() -> Self {
        Self
    }

    /// Process a query and return a stream of messages.
    pub async fn process_query(
        &self,
        prompt: PromptInput,
        options: ClaudeCodeOptions,
    ) -> impl Stream<Item = Result<Message>> {
        async_stream::stream! {
            // Create transport with close_stdin_after_prompt = true for one-shot queries
            let mut transport = match SubprocessCliTransport::new(
                prompt,
                options,
                None,
                true, // close_stdin_after_prompt
            ) {
                Ok(transport) => transport,
                Err(e) => {
                    yield Err(e);
                    return;
                }
            };

            // Connect to the CLI
            if let Err(e) = transport.connect().await {
                yield Err(e);
                return;
            }

            // Process messages from the transport
            {
                let message_stream = transport.receive_messages().await;
                tokio::pin!(message_stream);
                while let Some(data_result) = message_stream.next().await {
                    match data_result {
                        Ok(data) => {
                            match parse_message(data) {
                                Ok(message) => {
                                    let is_result = matches!(message, Message::Result(_));
                                    yield Ok(message);
                                    // Stop after result message for one-shot queries
                                    if is_result {
                                        break;
                                    }
                                }
                                Err(e) => yield Err(e),
                            }
                        }
                        Err(e) => {
                            yield Err(e);
                            break;
                        }
                    }
                }
            }

            // Clean up transport
            if let Err(e) = transport.disconnect().await {
                yield Err(e);
            }
        }
    }
}

impl Default for InternalClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Interactive client for bidirectional conversations with Claude.
///
/// [`ClaudeSDKClient`] provides a stateful interface for conducting multi-turn conversations
/// with Claude. Unlike the one-shot [`query`](crate::query) function, this client maintains
/// a persistent connection to the CLI process, allowing for efficient back-and-forth
/// communication without the overhead of process startup for each message.
///
/// # Features
///
/// - **Persistent Connection**: Maintains a single CLI process for multiple interactions
/// - **Session Management**: Tracks session IDs for conversation continuity
/// - **Stream-based Communication**: Async streams for real-time message processing
/// - **Interrupt Support**: Can send interrupt signals to cancel ongoing operations
/// - **Resource Management**: Automatic cleanup with explicit disconnect option
///
/// # Lifecycle
///
/// 1. **Create**: Use [`new`](Self::new) to create a client instance
/// 2. **Connect**: Call [`connect`](Self::connect) to establish CLI connection
/// 3. **Interact**: Use [`query`](Self::query) and [`receive_messages`](Self::receive_messages) for communication
/// 4. **Cleanup**: Call [`disconnect`](Self::disconnect) for guaranteed resource cleanup
///
/// # Examples
///
/// ## Basic Interactive Session
///
/// ```rust,no_run
/// use claude_code_sdk::{ClaudeSDKClient, PromptInput, Message};
/// use tokio_stream::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> claude_code_sdk::Result<()> {
///     let mut client = ClaudeSDKClient::new(None);
///     client.connect(None).await?;
///     
///     // Send first message
///     client.query(PromptInput::from("Hello, Claude!"), None).await?;
///     
///     // Receive response
///     let responses = client.receive_response().await?;
///     tokio::pin!(responses);
///     
///     while let Some(message) = responses.next().await {
///         match message? {
///             Message::Assistant(msg) => println!("Claude: {:?}", msg.content),
///             Message::Result(_) => break,
///             _ => {}
///         }
///     }
///     
///     // Send follow-up message
///     client.query(PromptInput::from("Can you explain that further?"), None).await?;
///     
///     // Process more responses...
///     
///     client.disconnect().await?;
///     Ok(())
/// }
/// ```
///
/// ## With Custom Configuration
///
/// ```rust,no_run
/// use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions, PermissionMode};
///
/// #[tokio::main]
/// async fn main() -> claude_code_sdk::Result<()> {
///     let options = ClaudeCodeOptions::builder()
///         .system_prompt("You are a helpful coding assistant")
///         .permission_mode(PermissionMode::AcceptEdits)
///         .build();
///     
///     let mut client = ClaudeSDKClient::new(Some(options));
///     client.connect(None).await?;
///     
///     // Use the configured client...
///     
///     client.disconnect().await?;
///     Ok(())
/// }
/// ```
pub struct ClaudeSDKClient {
    options: ClaudeCodeOptions,
    transport: Option<SubprocessCliTransport>,
    current_session_id: String,
}

impl ClaudeSDKClient {
    /// Create a new interactive client with optional configuration.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional configuration for the client. If `None`, default options are used.
    ///   Use [`ClaudeCodeOptions::builder()`] to create custom configurations.
    ///
    /// # Returns
    ///
    /// Returns a new [`ClaudeSDKClient`] instance in the disconnected state.
    /// Call [`connect`](Self::connect) to establish a connection to the CLI.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions};
    ///
    /// // With default options
    /// let client = ClaudeSDKClient::new(None);
    ///
    /// // With custom options
    /// let options = ClaudeCodeOptions::builder()
    ///     .system_prompt("You are a helpful assistant")
    ///     .build();
    /// let client = ClaudeSDKClient::new(Some(options));
    /// ```
    pub fn new(options: Option<ClaudeCodeOptions>) -> Self {
        Self {
            options: options.unwrap_or_default(),
            transport: None,
            current_session_id: "default".to_string(),
        }
    }

    /// Connect to the Claude CLI process.
    ///
    /// This method establishes a connection to the Claude Code CLI and prepares the client
    /// for interactive communication. The connection must be established before sending
    /// queries or receiving messages.
    ///
    /// # Arguments
    ///
    /// * `prompt` - Optional initial prompt to send during connection. If `None`, the client
    ///   starts in interactive mode ready to receive queries via [`query`](Self::query).
    ///   If provided, the prompt will be sent immediately after connection.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful connection, or an error if the connection fails.
    ///
    /// # Errors
    ///
    /// - [`SdkError::CliNotFound`]: Claude Code CLI is not installed or not in PATH
    /// - [`SdkError::NodeJsNotFound`]: Node.js runtime is not available
    /// - [`SdkError::CliConnection`]: Failed to start or connect to the CLI process
    /// - [`SdkError::InvalidWorkingDirectory`]: Specified working directory is invalid
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::{ClaudeSDKClient, PromptInput};
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     
    ///     // Connect without initial prompt
    ///     client.connect(None).await?;
    ///     
    ///     // Or connect with initial prompt
    ///     // client.connect(Some(PromptInput::from("Hello!"))).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(&mut self, prompt: Option<PromptInput>) -> Result<()> {
        let prompt = prompt.unwrap_or_else(|| {
            // Create a pending stream that never yields for interactive mode
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

    /// Send a query message to Claude in the interactive session.
    ///
    /// This method sends a message to Claude and returns immediately. To receive Claude's
    /// response, use [`receive_messages`](Self::receive_messages) or 
    /// [`receive_response`](Self::receive_response) after calling this method.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to send to Claude. Can be either:
    ///   - [`PromptInput::Text`]: A simple text message
    ///   - [`PromptInput::Stream`]: An async stream of JSON values for complex interactions
    /// * `session_id` - Optional session ID for this query. If `None`, uses the client's
    ///   current session ID (see [`current_session_id`](Self::current_session_id)).
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the message was sent successfully, or an error if sending failed.
    ///
    /// # Errors
    ///
    /// - [`SdkError::Transport`]: Client is not connected or communication failed
    /// - [`SdkError::Stream`]: Empty stream provided or stream processing failed
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::{ClaudeSDKClient, PromptInput};
    /// use tokio_stream::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     client.connect(None).await?;
    ///     
    ///     // Send a text message
    ///     client.query(PromptInput::from("Hello, Claude!"), None).await?;
    ///     
    ///     // Receive the response
    ///     let responses = client.receive_response().await?;
    ///     tokio::pin!(responses);
    ///     
    ///     while let Some(message) = responses.next().await {
    ///         // Process response messages...
    ///         # break;
    ///     }
    ///     
    ///     client.disconnect().await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// This method only sends the message. You must call one of the receive methods
    /// to get Claude's response. The client maintains the connection state between
    /// calls, allowing for multi-turn conversations.
    pub async fn query(&mut self, prompt: PromptInput, session_id: Option<String>) -> Result<()> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| SdkError::transport("Not connected"))?;

        let session_id = session_id.unwrap_or_else(|| self.current_session_id.clone());

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
                transport
                    .send_request(vec![message], HashMap::new())
                    .await
            }
            PromptInput::Stream(mut stream) => {
                // Collect stream items and convert them to messages
                let mut messages = Vec::new();
                
                while let Some(item) = stream.next().await {
                    // Each stream item should be a JSON value representing a message
                    // We'll wrap it in the expected format for the CLI
                    let message = serde_json::json!({
                        "type": "user",
                        "message": item,
                        "parent_tool_use_id": null,
                        "session_id": session_id
                    });
                    messages.push(message);
                }
                
                if messages.is_empty() {
                    return Err(SdkError::transport("Empty stream provided"));
                }
                
                transport.send_request(messages, HashMap::new()).await
            }
        }
    }

    /// Receive messages from Claude as an async stream.
    ///
    /// This method returns a stream that yields all messages from Claude, including
    /// system messages, assistant responses, and result messages. The stream continues
    /// indefinitely until the connection is closed or an error occurs.
    ///
    /// For most use cases, consider using [`receive_response`](Self::receive_response)
    /// instead, which automatically stops after receiving a result message.
    ///
    /// # Returns
    ///
    /// Returns a stream of [`Message`] results. Each item in the stream represents
    /// a message from Claude and may be:
    /// - [`Message::System`]: Metadata and control information
    /// - [`Message::Assistant`]: Claude's response content
    /// - [`Message::Result`]: Query completion metadata
    ///
    /// # Errors
    ///
    /// - [`SdkError::Transport`]: Client is not connected
    /// - Stream items may contain parsing or communication errors
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::{ClaudeSDKClient, PromptInput, Message};
    /// use tokio_stream::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     client.connect(None).await?;
    ///     
    ///     client.query(PromptInput::from("Hello!"), None).await?;
    ///     
    ///     let messages = client.receive_messages().await?;
    ///     tokio::pin!(messages);
    ///     
    ///     while let Some(message_result) = messages.next().await {
    ///         match message_result? {
    ///             Message::Assistant(msg) => {
    ///                 println!("Claude: {:?}", msg.content);
    ///             }
    ///             Message::Result(_) => {
    ///                 println!("Query completed");
    ///                 break; // Manually break on result
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    ///     
    ///     client.disconnect().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn receive_messages(&mut self) -> Result<impl Stream<Item = Result<Message>> + '_> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| SdkError::transport("Not connected"))?;

        let message_stream = transport.receive_messages().await;
        Ok(message_stream.map(|data_result| data_result.and_then(parse_message)))
    }

    /// Receive messages until a result message is encountered.
    ///
    /// This is a convenience method that wraps [`receive_messages`](Self::receive_messages)
    /// and automatically terminates the stream after receiving a [`Message::Result`].
    /// This is the most common pattern for processing Claude's responses to a single query.
    ///
    /// # Returns
    ///
    /// Returns a stream of [`Message`] results that automatically terminates after
    /// yielding a [`Message::Result`]. The result message is included in the stream.
    ///
    /// # Errors
    ///
    /// - [`SdkError::Transport`]: Client is not connected
    /// - Stream items may contain parsing or communication errors
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::{ClaudeSDKClient, PromptInput, Message};
    /// use tokio_stream::StreamExt;
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     client.connect(None).await?;
    ///     
    ///     client.query(PromptInput::from("Hello!"), None).await?;
    ///     
    ///     let responses = client.receive_response().await?;
    ///     tokio::pin!(responses);
    ///     
    ///     while let Some(message) = responses.next().await {
    ///         match message? {
    ///             Message::Assistant(msg) => {
    ///                 println!("Claude: {:?}", msg.content);
    ///             }
    ///             Message::Result(result) => {
    ///                 println!("Completed in {}ms", result.duration_ms);
    ///                 // Stream automatically ends here
    ///             }
    ///             _ => {}
    ///         }
    ///     }
    ///     
    ///     client.disconnect().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn receive_response(&mut self) -> Result<impl Stream<Item = Result<Message>> + '_> {
        let messages = self.receive_messages().await?;
        Ok(async_stream::stream! {
            tokio::pin!(messages);
            while let Some(message_result) = messages.next().await {
                match message_result {
                    Ok(message) => {
                        let is_result = matches!(message, Message::Result(_));
                        yield Ok(message);
                        if is_result {
                            break;
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        })
    }

    /// Send an interrupt signal to Claude.
    ///
    /// This method sends a control signal to interrupt Claude's current processing.
    /// This can be useful to cancel long-running operations or stop Claude from
    /// continuing with a response.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the interrupt signal was sent successfully.
    ///
    /// # Errors
    ///
    /// - [`SdkError::Transport`]: Client is not connected
    /// - [`SdkError::Interrupt`]: Failed to send interrupt signal
    /// - [`SdkError::ControlTimeout`]: Interrupt acknowledgment timed out
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::ClaudeSDKClient;
    /// use tokio::time::{sleep, Duration};
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     client.connect(None).await?;
    ///     
    ///     // Start a potentially long-running query
    ///     client.query("Write a very long story".into(), None).await?;
    ///     
    ///     // Wait a bit, then interrupt
    ///     sleep(Duration::from_secs(2)).await;
    ///     client.interrupt().await?;
    ///     
    ///     client.disconnect().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn interrupt(&mut self) -> Result<()> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| SdkError::transport("Not connected"))?;
        transport.interrupt().await
    }

    /// Disconnect from the Claude CLI and clean up resources.
    ///
    /// This method gracefully terminates the CLI process and cleans up all associated
    /// resources. It should be called when you're done with the client to ensure
    /// proper cleanup, although the [`Drop`] implementation provides best-effort
    /// cleanup as a fallback.
    ///
    /// After calling this method, the client returns to the disconnected state and
    /// [`connect`](Self::connect) must be called again before sending queries.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if disconnection was successful, or an error if cleanup failed.
    /// Even if an error is returned, the client is considered disconnected.
    ///
    /// # Errors
    ///
    /// - [`SdkError::Process`]: Failed to terminate the CLI process gracefully
    /// - [`SdkError::Transport`]: Error during resource cleanup
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::ClaudeSDKClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     client.connect(None).await?;
    ///     
    ///     // Use the client...
    ///     
    ///     // Always disconnect explicitly for guaranteed cleanup
    ///     client.disconnect().await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// While the [`Drop`] implementation provides automatic cleanup, it cannot
    /// guarantee completion if the program exits immediately. For guaranteed
    /// resource cleanup, always call this method explicitly.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut transport) = self.transport.take() {
            transport.disconnect().await?;
        }
        Ok(())
    }

    /// Check if the client is currently connected to the CLI.
    ///
    /// # Returns
    ///
    /// Returns `true` if the client has an active connection to the Claude CLI,
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use claude_code_sdk::ClaudeSDKClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> claude_code_sdk::Result<()> {
    ///     let mut client = ClaudeSDKClient::new(None);
    ///     assert!(!client.is_connected());
    ///     
    ///     client.connect(None).await?;
    ///     assert!(client.is_connected());
    ///     
    ///     client.disconnect().await?;
    ///     assert!(!client.is_connected());
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
    }

    /// Get the current session ID.
    ///
    /// Returns the session ID that will be used for queries when no explicit
    /// session ID is provided to [`query`](Self::query).
    ///
    /// # Returns
    ///
    /// Returns a string slice containing the current session ID.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use claude_code_sdk::ClaudeSDKClient;
    ///
    /// let client = ClaudeSDKClient::new(None);
    /// assert_eq!(client.current_session_id(), "default");
    /// ```
    pub fn current_session_id(&self) -> &str {
        &self.current_session_id
    }

    /// Set the current session ID for future queries.
    ///
    /// This session ID will be used as the default for all subsequent calls to
    /// [`query`](Self::query) when no explicit session ID is provided.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The new session ID to use. Accepts any type that can be
    ///   converted to a `String`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use claude_code_sdk::ClaudeSDKClient;
    ///
    /// let mut client = ClaudeSDKClient::new(None);
    /// assert_eq!(client.current_session_id(), "default");
    ///
    /// client.set_session_id("my-session");
    /// assert_eq!(client.current_session_id(), "my-session");
    ///
    /// client.set_session_id(String::from("another-session"));
    /// assert_eq!(client.current_session_id(), "another-session");
    /// ```
    pub fn set_session_id<S: Into<String>>(&mut self, session_id: S) {
        self.current_session_id = session_id.into();
    }

    /// Create a new session with a generated unique ID.
    ///
    /// This method generates a new session ID based on the current timestamp and
    /// sets it as the current session ID for the client. This is useful for
    /// starting fresh conversations or organizing different interaction contexts.
    ///
    /// # Returns
    ///
    /// Returns the newly generated session ID as a `String`. The ID format is
    /// `"session_{timestamp}"` where timestamp is milliseconds since Unix epoch.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use claude_code_sdk::ClaudeSDKClient;
    ///
    /// let mut client = ClaudeSDKClient::new(None);
    /// assert_eq!(client.current_session_id(), "default");
    ///
    /// let new_id = client.new_session();
    /// assert!(new_id.starts_with("session_"));
    /// assert_eq!(client.current_session_id(), new_id);
    ///
    /// // Each call generates a unique ID
    /// let another_id = client.new_session();
    /// assert_ne!(new_id, another_id);
    /// ```
    ///
    /// # Use Cases
    ///
    /// - Starting a new conversation context
    /// - Organizing different types of interactions
    /// - Debugging and logging with unique identifiers
    /// - Implementing conversation history management
    pub fn new_session(&mut self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        
        let session_id = format!("session_{}", timestamp);
        self.current_session_id = session_id.clone();
        session_id
    }
}

impl Drop for ClaudeSDKClient {
    fn drop(&mut self) {
        if self.transport.is_some() {
            // Best-effort cleanup: spawn a task to handle async cleanup
            // Note: This is not guaranteed to complete if the main program exits immediately
            // Users should call disconnect() explicitly for guaranteed cleanup
            tokio::spawn(async move {
                // The transport's Drop implementation will handle process cleanup
            });
        }
    }
}