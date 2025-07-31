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
pub struct ClaudeSDKClient {
    options: ClaudeCodeOptions,
    transport: Option<SubprocessCliTransport>,
}

impl ClaudeSDKClient {
    /// Create a new interactive client.
    pub fn new(options: Option<ClaudeCodeOptions>) -> Self {
        Self {
            options: options.unwrap_or_default(),
            transport: None,
        }
    }

    /// Connect to the Claude CLI.
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

    /// Send a query message to Claude.
    pub async fn query(&mut self, prompt: PromptInput, session_id: Option<String>) -> Result<()> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| SdkError::transport("Not connected"))?;

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
                transport
                    .send_request(vec![message], HashMap::new())
                    .await
            }
            PromptInput::Stream(_stream) => {
                // TODO: Implement stream handling for async iterables
                // This would involve converting the stream items and sending them to transport
                Err(SdkError::transport(
                    "Stream-based prompts not yet implemented for interactive client",
                ))
            }
        }
    }

    /// Receive messages from Claude as a stream.
    pub async fn receive_messages(&mut self) -> Result<impl Stream<Item = Result<Message>> + '_> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| SdkError::transport("Not connected"))?;

        let message_stream = transport.receive_messages().await;
        Ok(message_stream.map(|data_result| data_result.and_then(parse_message)))
    }

    /// Receive messages until a result message is encountered.
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
    pub async fn interrupt(&mut self) -> Result<()> {
        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| SdkError::transport("Not connected"))?;
        transport.interrupt().await
    }

    /// Disconnect from the Claude CLI.
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut transport) = self.transport.take() {
            transport.disconnect().await?;
        }
        Ok(())
    }

    /// Check if the client is currently connected.
    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
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