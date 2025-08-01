//! Streaming mode example for the Claude Code SDK.
//!
//! This example demonstrates interactive usage with bidirectional communication.
//! It shows how to maintain a persistent connection for multiple exchanges.

use claude_code_sdk::{
    ClaudeCodeOptions, ClaudeSDKClient, ContentBlock, Message, PermissionMode, PromptInput,
};
use std::path::PathBuf;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> claude_code_sdk::Result<()> {
    println!("Claude Code SDK - Streaming Mode Example");
    println!("========================================");

    // Create client with comprehensive options
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant specializing in Rust programming.")
        .permission_mode(PermissionMode::Default)
        .max_thinking_tokens(2000)
        .allowed_tools(vec!["file_editor".to_string(), "bash".to_string()])
        .cwd(PathBuf::from("."))
        .build();

    let mut client = ClaudeSDKClient::new(Some(options));

    // Connect to Claude
    println!("🔌 Connecting to Claude...");
    client.connect(None).await?;
    println!("✅ Connected!\n");

    // First exchange: Ask for a Rust function
    println!("📤 Sending first message...");
    client.query(
        PromptInput::from("Hello! Can you help me write a simple Rust function that calculates the factorial of a number?"), 
        None
    ).await?;

    // Receive and process first response
    {
        let response_stream = client.receive_response().await?;
        tokio::pin!(response_stream);

        println!("📥 Receiving response...\n");
        while let Some(message) = response_stream.next().await {
            match message? {
                Message::Assistant(msg) => {
                    println!("🤖 Claude:");
                    for content_block in &msg.content {
                        match content_block {
                            ContentBlock::Text(text_block) => {
                                println!("{}", text_block.text);
                            }
                            ContentBlock::ToolUse(tool_block) => {
                                println!(
                                    "🔧 Using tool: {} (id: {})",
                                    tool_block.name, tool_block.id
                                );
                            }
                            ContentBlock::ToolResult(result_block) => {
                                println!("📋 Tool result: {:?}", result_block.content);
                            }
                        }
                    }
                    println!();
                }
                Message::Result(result) => {
                    println!("✅ First exchange completed in {}ms\n", result.duration_ms);
                    break;
                }
                Message::System(sys) => {
                    println!("🔧 System: {}", sys.subtype);
                }
                _ => {}
            }
        }
    }

    // Second exchange: Ask for optimization
    println!("📤 Sending follow-up message...");
    client
        .query(
            PromptInput::from(
                "Great! Now can you make it more efficient and add error handling for edge cases?",
            ),
            None,
        )
        .await?;

    // Receive and process follow-up response
    {
        let response_stream = client.receive_response().await?;
        tokio::pin!(response_stream);

        println!("📥 Receiving follow-up response...\n");
        while let Some(message) = response_stream.next().await {
            match message? {
                Message::Assistant(msg) => {
                    println!("🤖 Claude:");
                    for content_block in &msg.content {
                        match content_block {
                            ContentBlock::Text(text_block) => {
                                println!("{}", text_block.text);
                            }
                            ContentBlock::ToolUse(tool_block) => {
                                println!(
                                    "🔧 Using tool: {} (id: {})",
                                    tool_block.name, tool_block.id
                                );
                            }
                            ContentBlock::ToolResult(result_block) => {
                                println!("📋 Tool result: {:?}", result_block.content);
                            }
                        }
                    }
                    println!();
                }
                Message::Result(result) => {
                    println!("✅ Second exchange completed in {}ms", result.duration_ms);
                    println!("   Total turns in session: {}", result.num_turns);
                    println!("   Session ID: {}", result.session_id);
                    break;
                }
                Message::System(sys) => {
                    println!("🔧 System: {}", sys.subtype);
                }
                _ => {}
            }
        }
    }

    // Third exchange: Demonstrate session continuity
    println!("\n📤 Sending a third message to show session continuity...");
    client.query(
        PromptInput::from("Can you also show me how to write unit tests for the factorial function you created?"), 
        None
    ).await?;

    // Receive response
    {
        let response_stream = client.receive_response().await?;
        tokio::pin!(response_stream);

        println!("📥 Receiving third response...\n");
        while let Some(message) = response_stream.next().await {
            match message? {
                Message::Assistant(msg) => {
                    println!("🤖 Claude:");
                    for content_block in &msg.content {
                        match content_block {
                            ContentBlock::Text(text_block) => {
                                println!("{}", text_block.text);
                            }
                            ContentBlock::ToolUse(tool_block) => {
                                println!(
                                    "🔧 Using tool: {} (id: {})",
                                    tool_block.name, tool_block.id
                                );
                            }
                            ContentBlock::ToolResult(result_block) => {
                                println!("📋 Tool result: {:?}", result_block.content);
                            }
                        }
                    }
                    println!();
                }
                Message::Result(result) => {
                    println!("✅ Third exchange completed in {}ms", result.duration_ms);
                    println!("   Total turns in session: {}", result.num_turns);
                    println!("   Session maintains context across multiple exchanges");
                    break;
                }
                Message::System(sys) => {
                    println!("🔧 System: {}", sys.subtype);
                }
                _ => {}
            }
        }
    }

    // Always disconnect explicitly for guaranteed cleanup
    println!("\n🔌 Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected!");

    println!("\n🎉 Streaming mode example completed successfully!");
    println!("💡 Key takeaways:");
    println!("   - Use ClaudeSDKClient for interactive sessions");
    println!("   - Always call disconnect() for guaranteed cleanup");
    println!("   - Process response streams message by message");
    println!("   - Use interrupt() to cancel long-running requests");

    Ok(())
}
