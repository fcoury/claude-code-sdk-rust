//! Quick start example for the Claude Code SDK.
//!
//! This example demonstrates basic usage of the SDK for one-shot queries.

use claude_code_sdk::{query, ClaudeCodeOptions};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Claude Code SDK - Quick Start Example");
    println!("=====================================");

    // Create options (optional)
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant.")
        .build();

    // Execute a query
    let stream = query(
        "Hello, Claude! Can you help me understand Rust?",
        Some(options),
    )
    .await;
    tokio::pin!(stream);

    println!("Sending query to Claude...\n");

    // Process the response stream
    while let Some(message) = stream.next().await {
        match message? {
            claude_code_sdk::Message::Assistant(msg) => {
                println!("Claude: {:?}", msg.content);
            }
            claude_code_sdk::Message::Result(result) => {
                println!("\nQuery completed!");
                println!("Duration: {}ms", result.duration_ms);
                println!("API Duration: {}ms", result.duration_api_ms);
                println!("Turns: {}", result.num_turns);
                if let Some(cost) = result.total_cost_usd {
                    println!("Cost: ${:.4}", cost);
                }
                break;
            }
            claude_code_sdk::Message::System(sys) => {
                println!("System: {} - {:?}", sys.subtype, sys.data);
            }
            _ => {}
        }
    }

    Ok(())
}
