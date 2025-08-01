//! Error handling example for the Claude Code SDK.
//!
//! This example demonstrates proper error handling patterns and recovery strategies.
//! It shows how to handle different types of errors that can occur when using the SDK.

use claude_code_sdk::{query, ClaudeCodeOptions, ClaudeSDKClient, Message, SdkError};
use std::path::PathBuf;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    println!("Claude Code SDK - Error Handling Example");
    println!("========================================");

    // Example 1: Handle CLI discovery and basic connectivity
    println!("1. Testing CLI discovery and basic connectivity...");
    match test_basic_query().await {
        Ok(_) => println!("   ✅ CLI found and working correctly"),
        Err(e) => {
            println!("   ❌ Error occurred: {e}");
            handle_error(&e);
            println!("   🔍 Error category: {}", e.category());
            println!("   🔄 Is recoverable: {}", e.is_recoverable());
        }
    }

    // Example 2: Handle invalid working directory
    println!("\n2. Testing invalid working directory error...");
    let options = ClaudeCodeOptions::builder()
        .cwd(PathBuf::from("/nonexistent/directory/that/does/not/exist"))
        .build();

    let stream = query("Hello", Some(options)).await;
    tokio::pin!(stream);

    match stream.next().await {
        Some(Ok(message)) => {
            println!("   ⚠️  Unexpected success: {message:?}");
        }
        Some(Err(e)) => {
            println!("   ✅ Expected error caught: {e}");
            handle_error(&e);
            demonstrate_error_properties(&e);
        }
        None => println!("   ❌ Stream ended unexpectedly"),
    }

    // Example 3: Handle configuration errors
    println!("\n3. Testing configuration validation...");
    test_configuration_errors().await;

    // Example 4: Handle interactive client errors
    println!("\n4. Testing interactive client error handling...");
    test_interactive_client_errors().await;

    // Example 5: Demonstrate error recovery patterns
    println!("\n5. Demonstrating error recovery patterns...");
    demonstrate_error_recovery().await;

    println!("\n🎉 Error handling examples completed!");
    println!("\n💡 Key takeaways:");
    println!("   - Always handle errors explicitly in production code");
    println!("   - Use error categories and recoverability info for smart retry logic");
    println!("   - Check error messages for actionable guidance");
    println!("   - Consider graceful degradation for non-critical failures");
}

async fn test_basic_query() -> claude_code_sdk::Result<()> {
    let stream = query("Hello, Claude! This is a test.", None).await;
    tokio::pin!(stream);

    while let Some(message) = stream.next().await {
        match message? {
            Message::Result(_) => break,
            Message::Assistant(_) => {
                // Successfully received response
            }
            _ => {}
        }
    }

    Ok(())
}

async fn test_configuration_errors() {
    // Test with invalid tool configuration
    let options = ClaudeCodeOptions::builder()
        .allowed_tools(vec!["nonexistent_tool".to_string()])
        .disallowed_tools(vec!["file_editor".to_string()]) // Conflicting config
        .build();

    let stream = query("Test configuration", Some(options)).await;
    tokio::pin!(stream);

    match stream.next().await {
        Some(Ok(_)) => println!("   ✅ Configuration accepted"),
        Some(Err(e)) => {
            println!("   ⚠️  Configuration error: {e}");
            handle_error(&e);
        }
        None => println!("   ❌ Stream ended unexpectedly"),
    }
}

async fn test_interactive_client_errors() {
    let mut client = ClaudeSDKClient::new(None);

    // Try to query without connecting first
    match client.query("Hello".into(), None).await {
        Ok(_) => println!("   ⚠️  Unexpected success - should fail when not connected"),
        Err(e) => {
            println!("   ✅ Expected error: {e}");
            handle_error(&e);
        }
    }

    // Try to receive messages without connecting
    match client.receive_messages().await {
        Ok(_) => println!("   ⚠️  Unexpected success - should fail when not connected"),
        Err(e) => {
            println!("   ✅ Expected error: {e}");
            handle_error(&e);
        }
    };
}

async fn demonstrate_error_recovery() {
    println!("   🔄 Attempting query with retry logic...");

    let max_retries = 3;
    let mut attempt = 0;

    loop {
        attempt += 1;
        println!("   📡 Attempt {attempt} of {max_retries}");

        match test_basic_query().await {
            Ok(_) => {
                println!("   ✅ Query succeeded on attempt {attempt}");
                break;
            }
            Err(e) => {
                println!("   ❌ Attempt {attempt} failed: {e}");

                if !e.is_recoverable() {
                    println!("   🛑 Error is not recoverable, stopping retries");
                    handle_error(&e);
                    break;
                }

                if attempt >= max_retries {
                    println!("   🛑 Max retries exceeded");
                    handle_error(&e);
                    break;
                }

                println!("   ⏳ Waiting before retry...");
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }
        }
    }
}

fn handle_error(error: &SdkError) {
    match error {
        SdkError::CliNotFound(_) => {
            println!("   💡 Solution: Install Claude Code CLI");
            println!("      Command: npm install -g @anthropic-ai/claude-code");
            println!("      Docs: https://github.com/anthropics/claude-code");
        }
        SdkError::NodeJsNotFound => {
            println!("   💡 Solution: Install Node.js runtime");
            println!("      Download: https://nodejs.org/");
            println!("      Minimum version: Node.js 18+");
        }
        SdkError::InvalidWorkingDirectory { path } => {
            println!("   💡 Solution: Check directory path");
            println!("      Path: {path}");
            println!("      Ensure the directory exists and is accessible");
        }
        SdkError::Process { exit_code, stderr } => {
            println!("   💡 CLI process failed");
            println!("      Exit code: {exit_code:?}");
            if !stderr.is_empty() {
                println!("      Error output: {stderr}");
            }
            println!("      Try updating the CLI: npm update -g @anthropic-ai/claude-code");
        }
        SdkError::JsonDecode(json_err) => {
            println!("   💡 JSON parsing failed");
            println!("      Error: {json_err}");
            println!("      This may indicate CLI version incompatibility");
        }
        SdkError::MessageParse { message, data } => {
            println!("   💡 Message parsing failed");
            println!("      Error: {message}");
            println!("      Data: {data}");
        }
        SdkError::Transport(msg) => {
            println!("   💡 Transport layer error");
            println!("      Details: {msg}");
            println!("      Check network connectivity and CLI status");
        }
        SdkError::BufferSizeExceeded { limit } => {
            println!("   💡 Buffer size exceeded");
            println!("      Limit: {limit} bytes");
            println!("      Try reducing message size or increasing buffer limit");
        }
        SdkError::Session(msg) => {
            println!("   💡 Session management error");
            println!("      Details: {msg}");
            println!("      Try reconnecting or creating a new client");
        }
        SdkError::IncompatibleCliVersion { found, expected } => {
            println!("   💡 CLI version incompatibility");
            println!("      Found: {found}");
            println!("      Expected: {expected}");
            println!("      Update CLI: npm update -g @anthropic-ai/claude-code");
        }
        SdkError::CliConnection(io_err) => {
            println!("   💡 CLI connection failed");
            println!("      Error: {io_err}");
            println!("      Check if CLI is properly installed and accessible");
        }
        SdkError::ControlTimeout { timeout_ms } => {
            println!("   💡 Control request timed out");
            println!("      Timeout: {timeout_ms}ms");
            println!("      The CLI may be unresponsive or overloaded");
        }
        SdkError::Configuration { message, .. } => {
            println!("   💡 Configuration error");
            println!("      Details: {message}");
            println!("      Check your ClaudeCodeOptions settings");
        }
        SdkError::Stream { message, context } => {
            println!("   💡 Stream processing error");
            println!("      Details: {message}");
            if let Some(ctx) = context {
                println!("      Context: {ctx}");
            }
        }
        SdkError::Interrupt {
            message,
            request_id,
        } => {
            println!("   💡 Interrupt handling error");
            println!("      Details: {message}");
            if let Some(id) = request_id {
                println!("      Request ID: {id}");
            }
        }
    }
}

fn demonstrate_error_properties(error: &SdkError) {
    println!("   📊 Error Analysis:");
    println!("      Category: {}", error.category());
    println!("      Recoverable: {}", error.is_recoverable());

    // Show structured error data for debugging
    let debug_data = error.debug_data();
    println!(
        "      Debug data: {}",
        serde_json::to_string_pretty(&debug_data).unwrap_or_else(|_| "N/A".to_string())
    );
}
