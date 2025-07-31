//! Error handling example for the Claude Code SDK.
//!
//! This example demonstrates proper error handling patterns.

use claude_code_sdk::{query, ClaudeCodeOptions, SdkError};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    println!("Claude Code SDK - Error Handling Example");
    println!("========================================");

    // Example 1: Handle CLI not found error
    println!("1. Testing CLI discovery...");
    match test_basic_query().await {
        Ok(_) => println!("   ✓ CLI found and working"),
        Err(e) => {
            println!("   ✗ Error occurred: {}", e);
            handle_error(&e);
        }
    }

    // Example 2: Handle invalid working directory
    println!("\n2. Testing invalid working directory...");
    let options = ClaudeCodeOptions::builder()
        .cwd("/nonexistent/directory")
        .build();

    let stream = query("Hello", Some(options)).await;
    tokio::pin!(stream);
    match stream.next().await {
        Some(Ok(_)) => println!("   ✓ Unexpected success"),
        Some(Err(e)) => {
            println!("   ✗ Expected error: {}", e);
            handle_error(&e);
        }
        None => println!("   ✗ Stream ended unexpectedly"),
    }

    println!("\nError handling examples completed!");
}

async fn test_basic_query() -> Result<(), SdkError> {
    let stream = query("Hello, Claude!", None).await;
    tokio::pin!(stream);
    
    while let Some(message) = stream.next().await {
        match message? {
            claude_code_sdk::Message::Result(_) => break,
            _ => {}
        }
    }
    
    Ok(())
}

fn handle_error(error: &SdkError) {
    match error {
        SdkError::CliNotFound(_) => {
            println!("   💡 Install Claude Code CLI with: npm install -g @anthropic-ai/claude-code");
        }
        SdkError::NodeJsNotFound => {
            println!("   💡 Install Node.js from: https://nodejs.org/");
        }
        SdkError::InvalidWorkingDirectory { path } => {
            println!("   💡 Check that directory exists: {}", path);
        }
        SdkError::Process { exit_code, stderr } => {
            println!("   💡 CLI process failed with code {:?}", exit_code);
            if !stderr.is_empty() {
                println!("   📝 Error output: {}", stderr);
            }
        }
        SdkError::Transport(msg) => {
            println!("   💡 Transport error: {}", msg);
        }
        _ => {
            println!("   💡 Other error occurred");
        }
    }
}