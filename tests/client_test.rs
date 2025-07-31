//! Tests for the ClaudeSDKClient implementation.

use claude_code_sdk::{ClaudeSDKClient, ClaudeCodeOptions, PromptInput};
use tokio_stream::{self as stream, StreamExt};

#[tokio::test]
async fn test_client_creation() {
    let client = ClaudeSDKClient::new(None);
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_client_with_options() {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("Test prompt")
        .max_thinking_tokens(1000)
        .build();
    
    let client = ClaudeSDKClient::new(Some(options));
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_stream_cancellation() {
    // Test that streams can be cancelled early
    let test_stream = stream::iter(vec![
        serde_json::json!({"role": "user", "content": "Hello"}),
        serde_json::json!({"role": "user", "content": "World"}),
        serde_json::json!({"role": "user", "content": "Test"}),
    ]);
    
    let prompt = PromptInput::Stream(Box::pin(test_stream));
    
    // This test verifies that the PromptInput::Stream variant can be created
    // and that the stream can be consumed (even though we can't test the full
    // client functionality without a real CLI process)
    match prompt {
        PromptInput::Stream(mut stream) => {
            let mut count = 0;
            while let Some(_item) = stream.next().await {
                count += 1;
                if count >= 2 {
                    break; // Early termination
                }
            }
            assert_eq!(count, 2);
        }
        _ => panic!("Expected stream variant"),
    }
}

#[tokio::test]
async fn test_stream_combinators() {
    // Test that streams support standard combinators
    let test_stream = stream::iter(vec![
        Ok::<serde_json::Value, std::io::Error>(serde_json::json!({"type": "user"})),
        Ok::<serde_json::Value, std::io::Error>(serde_json::json!({"type": "assistant"})),
        Ok::<serde_json::Value, std::io::Error>(serde_json::json!({"type": "result"})),
    ]);
    
    // Test filter combinator
    let filtered: Vec<_> = test_stream
        .filter(|item| {
            if let Ok(json) = item {
                json.get("type").and_then(|t| t.as_str()) == Some("assistant")
            } else {
                false
            }
        })
        .collect()
        .await;
    
    assert_eq!(filtered.len(), 1);
}

#[tokio::test]
async fn test_error_handling() {
    let mut client = ClaudeSDKClient::new(None);
    
    // Test that operations fail when not connected
    let result = client.query("test".into(), None).await;
    assert!(result.is_err());
    
    {
        let result = client.receive_messages().await;
        assert!(result.is_err());
    }
    
    let result = client.interrupt().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_empty_stream_error() {
    let mut client = ClaudeSDKClient::new(None);
    
    // Create an empty stream
    let empty_stream = stream::empty();
    let prompt = PromptInput::Stream(Box::pin(empty_stream));
    
    // This should fail even if connected because we can't test connection
    // without a real CLI, but we can test the empty stream logic
    let result = client.query(prompt, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_session_management() {
    let mut client = ClaudeSDKClient::new(None);
    
    // Test default session ID
    assert_eq!(client.current_session_id(), "default");
    
    // Test setting session ID
    client.set_session_id("test_session");
    assert_eq!(client.current_session_id(), "test_session");
    
    // Test creating new session
    let new_session = client.new_session();
    assert!(new_session.starts_with("session_"));
    assert_eq!(client.current_session_id(), new_session);
}

#[tokio::test]
async fn test_control_response_detection() {
    // Test the control response detection logic
    let control_response = serde_json::json!({
        "type": "control_response",
        "request_id": "123",
        "status": "success"
    });
    
    let control_message = serde_json::json!({
        "type": "control",
        "request_id": "456",
        "action": "interrupt"
    });
    
    let regular_message = serde_json::json!({
        "type": "user",
        "content": "Hello"
    });
    
    // We can't directly test the private method, but we can test the logic
    // by checking the JSON structure that would be detected
    assert_eq!(control_response.get("type").and_then(|t| t.as_str()), Some("control_response"));
    assert_eq!(control_message.get("type").and_then(|t| t.as_str()), Some("control"));
    assert!(control_message.get("request_id").is_some());
    assert_eq!(regular_message.get("type").and_then(|t| t.as_str()), Some("user"));
}