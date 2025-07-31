//! Streaming mode example for the Claude Code SDK.
//!
//! This example demonstrates interactive usage with bidirectional communication.

use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions, PermissionMode};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Claude Code SDK - Streaming Mode Example");
    println!("========================================");

    // Create client with options
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant.")
        .permission_mode(PermissionMode::Default)
        .max_thinking_tokens(1000)
        .build();

    let mut client = ClaudeSDKClient::new(Some(options));

    // Connect to Claude
    println!("Connecting to Claude...");
    client.connect(None).await?;
    println!("Connected!\n");

    // Send first message
    println!("Sending first message...");
    client.query("Hello! Can you help me write a simple Rust function?".into(), None).await?;

    // Receive response
    {
        let response_stream = client.receive_response().await?;
        tokio::pin!(response_stream);
        while let Some(message) = response_stream.next().await {
            match message? {
                claude_code_sdk::Message::Assistant(msg) => {
                    println!("Claude: {:?}", msg.content);
                }
                claude_code_sdk::Message::Result(result) => {
                    println!("\nFirst exchange completed in {}ms", result.duration_ms);
                    break;
                }
                _ => {}
            }
        }
    }

    // Send follow-up message
    println!("\nSending follow-up message...");
    client.query("Can you make it more efficient?".into(), None).await?;

    // Receive follow-up response
    {
        let response_stream = client.receive_response().await?;
        tokio::pin!(response_stream);
        while let Some(message) = response_stream.next().await {
            match message? {
                claude_code_sdk::Message::Assistant(msg) => {
                    println!("Claude: {:?}", msg.content);
                }
                claude_code_sdk::Message::Result(result) => {
                    println!("\nSecond exchange completed in {}ms", result.duration_ms);
                    break;
                }
                _ => {}
            }
        }
    }

    // Disconnect
    println!("\nDisconnecting...");
    client.disconnect().await?;
    println!("Disconnected!");

    Ok(())
}