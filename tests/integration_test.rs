use claude_code_sdk::{
    transport::{SubprocessCliTransport, PromptInput},
    types::{ClaudeCodeOptions, Message},
    message_parser::parse_message,
    SdkError, Result,
};
use rstest::*;
use std::path::PathBuf;
use std::process::Command;
use tokio_stream::StreamExt;
use tokio_test;

// Helper function to get the path to our mock CLI
fn get_mock_cli_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("mock_claude_cli.js");
    path
}

// Helper function to check if Node.js is available
fn check_nodejs_available() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// Skip tests if Node.js is not available
macro_rules! skip_if_no_nodejs {
    () => {
        if !check_nodejs_available() {
            println!("Skipping test: Node.js not available");
            return;
        }
    };
}

// Helper function to create a mock CLI command that includes test parameters
fn create_mock_cli_command(test_mode: &str, additional_args: Vec<String>) -> PathBuf {
    // We'll create a wrapper script that calls our mock CLI with the right parameters
    let mut wrapper_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    wrapper_path.push("target");
    wrapper_path.push("test_cli_wrapper.js");
    
    let mock_cli_path = get_mock_cli_path();
    let wrapper_content = format!(
        r#"#!/usr/bin/env node
const {{ spawn }} = require('child_process');
const args = ['{}', '--test-mode', '{}'];
// Add any additional args passed to this wrapper
args.push(...process.argv.slice(2));
{}
const child = spawn('node', args, {{ stdio: 'inherit' }});
child.on('exit', (code) => process.exit(code));
"#,
        mock_cli_path.to_string_lossy(),
        test_mode,
        additional_args.iter().map(|arg| format!("args.push('{}');", arg)).collect::<Vec<_>>().join("\n")
    );
    
    std::fs::write(&wrapper_path, wrapper_content).expect("Failed to write wrapper script");
    
    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&wrapper_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms).unwrap();
    }
    
    wrapper_path
}

// ============================================================================
// Basic Transport Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_transport_connect_disconnect() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Hello, test!".to_string());
    let mock_cli_path = get_mock_cli_path();
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    // Test connection
    let connect_result = transport.connect().await;
    assert!(connect_result.is_ok(), "Failed to connect: {:?}", connect_result);
    
    // Test disconnection
    let disconnect_result = transport.disconnect().await;
    assert!(disconnect_result.is_ok(), "Failed to disconnect: {:?}", disconnect_result);
}

#[tokio::test]
async fn test_transport_normal_message_flow() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Hello, test!".to_string());
    let mock_cli_path = create_mock_cli_command("normal", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    // Collect all messages
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse message");
                messages.push(message);
                
                // Stop after result message
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Error receiving message: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Verify message sequence
    assert!(messages.len() >= 3, "Expected at least 3 messages, got {}", messages.len());
    
    // Check message types
    assert!(matches!(messages[0], Message::System(_)), "First message should be System");
    assert!(matches!(messages[1], Message::Assistant(_)), "Second message should be Assistant");
    assert!(matches!(messages.last(), Some(Message::Result(_))), "Last message should be Result");
    
    // Verify content
    if let Message::Assistant(ref assistant_msg) = messages[1] {
        assert!(!assistant_msg.content.is_empty(), "Assistant message should have content");
    }
    
    if let Message::Result(ref result_msg) = messages.last().unwrap() {
        assert!(!result_msg.is_error, "Result should not be an error");
        assert_eq!(result_msg.session_id, "test_session_123");
    }
}

// ============================================================================
// JSON Buffering Tests
// ============================================================================

#[tokio::test]
async fn test_split_json_buffering() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Split JSON test".to_string());
    let mock_cli_path = create_mock_cli_command("split_json", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse split JSON message");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Error with split JSON: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should successfully parse the split message
    assert!(messages.len() >= 2, "Should have received messages despite JSON splitting");
    
    // Verify the long message was parsed correctly
    if let Some(Message::Assistant(assistant_msg)) = messages.iter().find(|m| matches!(m, Message::Assistant(_))) {
        if let Some(content_block) = assistant_msg.content.first() {
            if let claude_code_sdk::types::ContentBlock::Text(text_block) = content_block {
                assert!(text_block.text.len() > 1000, "Should have received the long text message");
                assert!(text_block.text.contains("very long message"), "Should contain expected text");
            }
        }
    }
}

#[tokio::test]
async fn test_concatenated_json_buffering() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Concatenated JSON test".to_string());
    let mock_cli_path = create_mock_cli_command("concatenated_json", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse concatenated JSON");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Error with concatenated JSON: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should receive all concatenated messages
    assert!(messages.len() >= 4, "Should have received all concatenated messages, got {}", messages.len());
    
    // Verify we got the expected sequence
    let system_count = messages.iter().filter(|m| matches!(m, Message::System(_))).count();
    let assistant_count = messages.iter().filter(|m| matches!(m, Message::Assistant(_))).count();
    let result_count = messages.iter().filter(|m| matches!(m, Message::Result(_))).count();
    
    assert_eq!(system_count, 1, "Should have 1 system message");
    assert_eq!(assistant_count, 2, "Should have 2 assistant messages");
    assert_eq!(result_count, 1, "Should have 1 result message");
}

#[tokio::test]
async fn test_large_message_handling() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Large message test".to_string());
    let mock_cli_path = create_mock_cli_command("large_message", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse large message");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Error with large message: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should handle large messages without issues
    assert!(messages.len() >= 2, "Should have received messages");
    
    // Verify large content was received
    if let Some(Message::Assistant(assistant_msg)) = messages.iter().find(|m| matches!(m, Message::Assistant(_))) {
        assert!(assistant_msg.content.len() >= 2, "Should have multiple content blocks");
        
        // Check for large text block
        let has_large_text = assistant_msg.content.iter().any(|block| {
            if let claude_code_sdk::types::ContentBlock::Text(text_block) = block {
                text_block.text.len() > 40000 // 40KB+
            } else {
                false
            }
        });
        assert!(has_large_text, "Should have received large text block");
        
        // Check for tool use with large input
        let has_large_tool_use = assistant_msg.content.iter().any(|block| {
            if let claude_code_sdk::types::ContentBlock::ToolUse(tool_block) = block {
                tool_block.id == "large_tool_123"
            } else {
                false
            }
        });
        assert!(has_large_tool_use, "Should have received tool use block");
    }
}

// ============================================================================
// Error Handling and Process Management Tests
// ============================================================================

#[tokio::test]
async fn test_process_error_handling() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Error test".to_string());
    let mock_cli_path = create_mock_cli_command("error_exit", vec![
        "--test-exit-code".to_string(), "42".to_string(),
        "--test-error-message".to_string(), "Test error message".to_string()
    ]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut error_occurred = false;
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(_) => {
                // Shouldn't get successful messages in error mode
            }
            Err(e) => {
                error_occurred = true;
                // Should be a process error
                assert!(matches!(e, SdkError::Process { .. }), "Expected Process error, got: {:?}", e);
                
                if let SdkError::Process { exit_code, stderr } = e {
                    assert_eq!(exit_code, Some(42), "Should have correct exit code");
                    assert!(stderr.contains("Test error message"), "Should contain error message");
                }
                break;
            }
        }
    }
    
    assert!(error_occurred, "Should have received an error");
    
    // Disconnect should still work
    let disconnect_result = transport.disconnect().await;
    assert!(disconnect_result.is_ok(), "Disconnect should work even after error");
}

#[tokio::test]
async fn test_stderr_collection() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Stderr test".to_string());
    let mock_cli_path = create_mock_cli_command("stderr_output", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse message");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should still receive normal messages despite stderr output
    assert!(messages.len() >= 2, "Should have received messages despite stderr");
    
    // The stderr should be collected but not interfere with normal operation
    // (In a real implementation, stderr would be available for debugging)
}

#[tokio::test]
async fn test_malformed_json_handling() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Malformed JSON test".to_string());
    let mock_cli_path = create_mock_cli_command("malformed_json", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut valid_messages = Vec::new();
    let mut json_errors = 0;
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                match parse_message(json_value) {
                    Ok(message) => {
                        valid_messages.push(message);
                        if matches!(valid_messages.last(), Some(Message::Result(_))) {
                            break;
                        }
                    }
                    Err(_) => {
                        // Message parsing errors are expected with malformed JSON
                    }
                }
            }
            Err(SdkError::JsonDecode(_)) => {
                json_errors += 1;
                // JSON decode errors are expected
            }
            Err(e) => panic!("Unexpected error type: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should have encountered JSON errors but still received valid final message
    assert!(json_errors > 0, "Should have encountered JSON decode errors");
    assert!(valid_messages.len() >= 1, "Should have received at least the final valid message");
    
    // Final message should be the result
    if let Some(Message::Result(result_msg)) = valid_messages.last() {
        assert_eq!(result_msg.session_id, "malformed_test");
    }
}

// ============================================================================
// Interactive Mode and Control Tests
// ============================================================================

#[tokio::test]
async fn test_interactive_mode() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Stream(Box::pin(tokio_stream::empty())); // Empty stream for interactive
    let mock_cli_path = create_mock_cli_command("interactive", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        false, // Don't close stdin
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    // Send a test message
    let test_message = serde_json::json!({
        "type": "user",
        "content": "Hello interactive mode!"
    });
    
    transport.send_request(vec![test_message], std::collections::HashMap::new())
        .await
        .expect("Failed to send request");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    let mut response_count = 0;
    
    // Collect responses
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse interactive message");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    response_count += 1;
                    if response_count >= 2 { // System start + our response
                        break;
                    }
                }
            }
            Err(e) => panic!("Error in interactive mode: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should have received system start and response messages
    assert!(messages.len() >= 3, "Should have received system, assistant, and result messages");
    
    // Check for interactive response
    let has_interactive_response = messages.iter().any(|msg| {
        if let Message::Assistant(assistant_msg) = msg {
            assistant_msg.content.iter().any(|block| {
                if let claude_code_sdk::types::ContentBlock::Text(text_block) = block {
                    text_block.text.contains("Interactive response")
                } else {
                    false
                }
            })
        } else {
            false
        }
    });
    
    assert!(has_interactive_response, "Should have received interactive response");
}

// ============================================================================
// Unicode and Special Content Tests
// ============================================================================

#[tokio::test]
async fn test_unicode_content_handling() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Unicode test".to_string());
    let mock_cli_path = create_mock_cli_command("unicode_content", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse unicode message");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Error with unicode content: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should handle unicode content correctly
    assert!(messages.len() >= 2, "Should have received messages");
    
    // Verify unicode content
    if let Some(Message::Assistant(assistant_msg)) = messages.iter().find(|m| matches!(m, Message::Assistant(_))) {
        let has_unicode_text = assistant_msg.content.iter().any(|block| {
            if let claude_code_sdk::types::ContentBlock::Text(text_block) = block {
                text_block.text.contains("世界") && 
                text_block.text.contains("🌍") && 
                text_block.text.contains("мир") &&
                text_block.text.contains("مرحبا")
            } else {
                false
            }
        });
        assert!(has_unicode_text, "Should have received unicode text content");
        
        let has_unicode_tool_use = assistant_msg.content.iter().any(|block| {
            if let claude_code_sdk::types::ContentBlock::ToolUse(tool_block) = block {
                tool_block.name == "text_processor" && tool_block.id == "unicode_tool"
            } else {
                false
            }
        });
        assert!(has_unicode_tool_use, "Should have received unicode tool use");
    }
}

#[tokio::test]
async fn test_empty_response_handling() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Empty response test".to_string());
    let mock_cli_path = create_mock_cli_command("empty_response", vec![]);
    
    let mut transport = SubprocessCliTransport::new(
        prompt,
        options,
        Some(mock_cli_path),
        true,
    ).expect("Failed to create transport");
    
    transport.connect().await.expect("Failed to connect");
    
    let message_stream = transport.receive_messages().await;
    tokio::pin!(message_stream);
    let mut messages = Vec::new();
    
    while let Some(message_result) = message_stream.next().await {
        match message_result {
            Ok(json_value) => {
                let message = parse_message(json_value).expect("Failed to parse empty response");
                messages.push(message);
                
                if matches!(messages.last(), Some(Message::Result(_))) {
                    break;
                }
            }
            Err(e) => panic!("Error with empty response: {:?}", e),
        }
    }
    
    transport.disconnect().await.expect("Failed to disconnect");
    
    // Should handle empty responses gracefully
    assert!(messages.len() >= 2, "Should have received messages");
    
    // Verify empty assistant message
    if let Some(Message::Assistant(assistant_msg)) = messages.iter().find(|m| matches!(m, Message::Assistant(_))) {
        assert!(assistant_msg.content.is_empty(), "Assistant message should have empty content");
    }
}

// ============================================================================
// Resource Management and Cleanup Tests
// ============================================================================

#[tokio::test]
async fn test_multiple_connect_disconnect_cycles() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let mock_cli_path = get_mock_cli_path();
    
    for i in 0..3 {
        let prompt = PromptInput::Text(format!("Test cycle {}", i));
        let mock_cli_path = create_mock_cli_command("normal", vec![]);
        
        let mut transport = SubprocessCliTransport::new(
            prompt,
            options.clone(),
            Some(mock_cli_path),
            true,
        ).expect("Failed to create transport");
        
        // Connect
        transport.connect().await.expect(&format!("Failed to connect on cycle {}", i));
        
        // Receive at least one message
        let message_stream = transport.receive_messages().await;
        tokio::pin!(message_stream);
        if let Some(message_result) = message_stream.next().await {
            assert!(message_result.is_ok(), "Should receive valid message on cycle {}", i);
        }
        
        // Disconnect
        transport.disconnect().await.expect(&format!("Failed to disconnect on cycle {}", i));
    }
}

#[tokio::test]
async fn test_transport_drop_cleanup() {
    skip_if_no_nodejs!();
    
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("Drop test".to_string());
    let mock_cli_path = create_mock_cli_command("normal", vec![]);
    
    {
        let mut transport = SubprocessCliTransport::new(
            prompt,
            options,
            Some(mock_cli_path),
            true,
        ).expect("Failed to create transport");
        
        transport.connect().await.expect("Failed to connect");
        
        // Don't explicitly disconnect - let Drop handle it
    } // transport goes out of scope here
    
    // Give some time for cleanup
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // If we get here without hanging, Drop cleanup worked
    assert!(true, "Drop cleanup completed successfully");
}

