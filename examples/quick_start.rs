//! Quick start example for the Claude Code SDK.
//!
//! This example demonstrates basic usage of the SDK for one-shot queries.
//! It shows how to send a simple prompt to Claude and process the response stream.

use claude_code_sdk::{query, ClaudeCodeOptions, Message, ContentBlock};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> claude_code_sdk::Result<()> {
    println!("Claude Code SDK - Quick Start Example");
    println!("=====================================");

    // Create options (optional) - demonstrates builder pattern
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant that explains programming concepts clearly.")
        .max_thinking_tokens(1000)
        .build();

    // Execute a query - demonstrates the main query function
    let stream = query(
        "Hello, Claude! Can you help me understand what makes Rust special as a programming language?",
        Some(options),
    )
    .await;
    tokio::pin!(stream);

    println!("Sending query to Claude...\n");

    // Process the response stream
    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                println!("Claude:");
                for content_block in &msg.content {
                    match content_block {
                        ContentBlock::Text(text_block) => {
                            println!("{}", text_block.text);
                        }
                        ContentBlock::ToolUse(tool_block) => {
                            println!("🔧 Using tool: {} (id: {})", tool_block.name, tool_block.id);
                            println!("   Input: {:?}", tool_block.input);
                        }
                        ContentBlock::ToolResult(result_block) => {
                            println!("📋 Tool result for: {}", result_block.tool_use_id);
                            if let Some(content) = &result_block.content {
                                println!("   Content: {:?}", content);
                            }
                        }
                    }
                }
                println!();
            }
            Message::Result(result) => {
                println!("✅ Query completed!");
                println!("   Duration: {}ms", result.duration_ms);
                println!("   API Duration: {}ms", result.duration_api_ms);
                println!("   Turns: {}", result.num_turns);
                println!("   Session ID: {}", result.session_id);
                if let Some(cost) = result.total_cost_usd {
                    println!("   Cost: ${:.4}", cost);
                }
                if result.is_error {
                    println!("   ⚠️  Query completed with errors");
                }
                break;
            }
            Message::System(sys) => {
                println!("🔧 System: {} - {:?}", sys.subtype, sys.data);
            }
            Message::User(user) => {
                println!("👤 User: {:?}", user.content);
            }
        }
    }

    println!("\n🎉 Example completed successfully!");
    Ok(())
}
