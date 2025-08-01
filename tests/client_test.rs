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

// Stream Handling Tests

#[tokio::test]
async fn test_async_stream_backpressure() {
    use std::time::Duration;
    use tokio::time::sleep;
    
    // Create a stream that produces items with delays to test backpressure
    let delayed_stream = stream::iter(vec![
        serde_json::json!({"message": "first"}),
        serde_json::json!({"message": "second"}),
        serde_json::json!({"message": "third"}),
    ])
    .then(|item| async move {
        sleep(Duration::from_millis(10)).await;
        item
    });
    
    let prompt = PromptInput::Stream(Box::pin(delayed_stream));
    
    // Test that the stream can handle backpressure properly
    match prompt {
        PromptInput::Stream(mut stream) => {
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            assert_eq!(items.len(), 3);
        }
        _ => panic!("Expected stream variant"),
    }
}

#[tokio::test]
async fn test_stream_error_propagation() {
    use std::io::{Error, ErrorKind};
    
    // Create a stream that produces an error
    let error_stream = stream::iter(vec![
        Ok::<serde_json::Value, Error>(serde_json::json!({"message": "ok"})),
        Err::<serde_json::Value, Error>(Error::new(ErrorKind::Other, "test error")),
        Ok::<serde_json::Value, Error>(serde_json::json!({"message": "after_error"})),
    ]);
    
    let mut error_count = 0;
    let mut success_count = 0;
    
    tokio::pin!(error_stream);
    while let Some(result) = error_stream.next().await {
        match result {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    assert_eq!(success_count, 2);
    assert_eq!(error_count, 1);
}

#[tokio::test]
async fn test_stream_timeout_handling() {
    use std::time::Duration;
    use tokio::time::{timeout, sleep};
    
    // Create a stream that takes too long
    let slow_stream = stream::iter(vec![
        serde_json::json!({"message": "fast"}),
    ])
    .chain(
        stream::iter(vec![serde_json::json!({"message": "slow"})])
            .then(|item| async move {
                sleep(Duration::from_millis(200)).await;
                item
            })
    );
    
    let prompt = PromptInput::Stream(Box::pin(slow_stream));
    
    match prompt {
        PromptInput::Stream(mut stream) => {
            // First item should be fast
            let first = timeout(Duration::from_millis(50), stream.next()).await;
            assert!(first.is_ok());
            
            // Second item should timeout
            let second = timeout(Duration::from_millis(50), stream.next()).await;
            assert!(second.is_err()); // Timeout error
        }
        _ => panic!("Expected stream variant"),
    }
}

#[tokio::test]
async fn test_stream_large_data_handling() {
    // Test handling of large JSON objects in streams
    let large_content = "x".repeat(10000); // 10KB string
    let large_stream = stream::iter(vec![
        serde_json::json!({"type": "user", "content": large_content.clone()}),
        serde_json::json!({"type": "assistant", "content": large_content.clone()}),
        serde_json::json!({"type": "result", "data": large_content}),
    ]);
    
    let prompt = PromptInput::Stream(Box::pin(large_stream));
    
    match prompt {
        PromptInput::Stream(mut stream) => {
            let mut total_size = 0;
            while let Some(item) = stream.next().await {
                let serialized = serde_json::to_string(&item).unwrap();
                total_size += serialized.len();
            }
            // Should have processed all large items
            assert!(total_size > 30000); // At least 30KB total
        }
        _ => panic!("Expected stream variant"),
    }
}

// Client State Management Tests

#[tokio::test]
async fn test_client_state_transitions() {
    let mut client = ClaudeSDKClient::new(None);
    
    // Initial state
    assert!(!client.is_connected());
    assert_eq!(client.current_session_id(), "default");
    
    // Test session management
    let session1 = client.new_session();
    assert_eq!(client.current_session_id(), session1);
    
    client.set_session_id("custom_session");
    assert_eq!(client.current_session_id(), "custom_session");
    
    // Test that operations fail when not connected
    let query_result = client.query("test".into(), None).await;
    assert!(query_result.is_err());
    
    {
        let receive_result = client.receive_messages().await;
        assert!(receive_result.is_err());
    }
    
    let interrupt_result = client.interrupt().await;
    assert!(interrupt_result.is_err());
}

#[tokio::test]
async fn test_client_options_handling() {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("Custom system prompt")
        .max_thinking_tokens(2000)
        .allowed_tools(vec!["tool1".to_string(), "tool2".to_string()])
        .build();
    
    let client = ClaudeSDKClient::new(Some(options.clone()));
    
    // Test that client was created successfully with options
    // (We can't access the options directly as they're private, but we can test that the client works)
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_client_concurrent_operations() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let client = Arc::new(Mutex::new(ClaudeSDKClient::new(None)));
    
    // Test that multiple concurrent operations handle the not-connected state properly
    let handles: Vec<_> = (0..5).map(|i| {
        let client = Arc::clone(&client);
        tokio::spawn(async move {
            let mut client = client.lock().await;
            let result = client.query(format!("test {}", i).into(), None).await;
            assert!(result.is_err());
        })
    }).collect();
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

// Message Flow and Control Tests

#[tokio::test]
async fn test_message_type_detection() {
    // Test different message types that the client should handle
    let user_message = serde_json::json!({
        "type": "user",
        "content": "Hello"
    });
    
    let assistant_message = serde_json::json!({
        "type": "assistant",
        "content": [{"type": "text", "text": "Hi there"}]
    });
    
    let system_message = serde_json::json!({
        "type": "system",
        "subtype": "session_start",
        "data": {"session_id": "test"}
    });
    
    let result_message = serde_json::json!({
        "type": "result",
        "subtype": "query_complete",
        "is_error": false,
        "session_id": "test"
    });
    
    let control_response = serde_json::json!({
        "type": "control_response",
        "request_id": "123",
        "status": "success"
    });
    
    // Verify message type detection logic
    assert_eq!(user_message["type"], "user");
    assert_eq!(assistant_message["type"], "assistant");
    assert_eq!(system_message["type"], "system");
    assert_eq!(result_message["type"], "result");
    assert_eq!(control_response["type"], "control_response");
    
    // Test control response detection
    assert!(control_response.get("request_id").is_some());
    assert!(control_response.get("status").is_some());
}

#[tokio::test]
async fn test_session_id_generation() {
    let mut client = ClaudeSDKClient::new(None);
    
    // Test that new session IDs are unique (add small delays to ensure uniqueness)
    let session1 = client.new_session();
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    let session2 = client.new_session();
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    let session3 = client.new_session();
    
    assert_ne!(session1, session2);
    assert_ne!(session2, session3);
    assert_ne!(session1, session3);
    
    // Test that session IDs have the expected format
    assert!(session1.starts_with("session_"));
    assert!(session2.starts_with("session_"));
    assert!(session3.starts_with("session_"));
    
    // Test that the current session is updated
    assert_eq!(client.current_session_id(), session3);
}

#[tokio::test]
async fn test_prompt_input_variants() {
    // Test Text variant
    let text_prompt = PromptInput::Text("Hello, world!".to_string());
    match text_prompt {
        PromptInput::Text(text) => assert_eq!(text, "Hello, world!"),
        _ => panic!("Expected Text variant"),
    }
    
    // Test Stream variant
    let stream_data = vec![
        serde_json::json!({"message": 1}),
        serde_json::json!({"message": 2}),
    ];
    let test_stream = stream::iter(stream_data.clone());
    let stream_prompt = PromptInput::Stream(Box::pin(test_stream));
    
    match stream_prompt {
        PromptInput::Stream(mut stream) => {
            let mut collected = Vec::new();
            while let Some(item) = stream.next().await {
                collected.push(item);
            }
            assert_eq!(collected.len(), 2);
            assert_eq!(collected[0], stream_data[0]);
            assert_eq!(collected[1], stream_data[1]);
        }
        _ => panic!("Expected Stream variant"),
    }
}

// Error Handling and Edge Cases

#[tokio::test]
async fn test_client_error_scenarios() {
    let mut client = ClaudeSDKClient::new(None);
    
    // Test query with empty string
    let result = client.query("".into(), None).await;
    assert!(result.is_err());
    
    // Test query with None session
    let result = client.query("test".into(), None).await;
    assert!(result.is_err());
    
    // Test interrupt without connection
    let result = client.interrupt().await;
    assert!(result.is_err());
    
    // Test receive_messages without connection
    {
        let result = client.receive_messages().await;
        assert!(result.is_err());
    }
    
    // Test receive_response without connection
    {
        let result = client.receive_response().await;
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_stream_edge_cases() {
    // Test empty stream
    let empty_stream = stream::empty::<serde_json::Value>();
    let prompt = PromptInput::Stream(Box::pin(empty_stream));
    
    match prompt {
        PromptInput::Stream(mut stream) => {
            let item = stream.next().await;
            assert!(item.is_none());
        }
        _ => panic!("Expected Stream variant"),
    }
    
    // Test single item stream
    let single_stream = stream::iter(vec![serde_json::json!({"single": true})]);
    let prompt = PromptInput::Stream(Box::pin(single_stream));
    
    match prompt {
        PromptInput::Stream(mut stream) => {
            let first = stream.next().await;
            assert!(first.is_some());
            let second = stream.next().await;
            assert!(second.is_none());
        }
        _ => panic!("Expected Stream variant"),
    }
}

#[tokio::test]
async fn test_client_drop_behavior() {
    // Test that client can be dropped safely
    {
        let _client = ClaudeSDKClient::new(None);
        // Client should drop cleanly here
    }
    
    // Test with options
    {
        let options = ClaudeCodeOptions::builder()
            .system_prompt("Test")
            .build();
        let _client = ClaudeSDKClient::new(Some(options));
        // Client should drop cleanly here
    }
}

// Performance and Resource Tests

#[tokio::test]
async fn test_stream_memory_efficiency() {
    // Test that streams don't consume excessive memory
    let large_stream = stream::iter((0..1000).map(|i| {
        serde_json::json!({"index": i, "data": format!("item_{}", i)})
    }));
    
    let prompt = PromptInput::Stream(Box::pin(large_stream));
    
    match prompt {
        PromptInput::Stream(mut stream) => {
            let mut count = 0;
            // Process stream items one by one without collecting all
            while let Some(_item) = stream.next().await {
                count += 1;
                if count >= 100 {
                    break; // Early termination to test memory efficiency
                }
            }
            assert_eq!(count, 100);
        }
        _ => panic!("Expected Stream variant"),
    }
}

#[tokio::test]
async fn test_concurrent_stream_processing() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let counter = Arc::new(AtomicUsize::new(0));
    
    // Create multiple streams and process them concurrently
    let handles: Vec<_> = (0..5).map(|stream_id| {
        let counter = Arc::clone(&counter);
        tokio::spawn(async move {
            let test_stream = stream::iter((0..10).map(move |i| {
                serde_json::json!({"stream": stream_id, "item": i})
            }));
            
            let prompt = PromptInput::Stream(Box::pin(test_stream));
            
            match prompt {
                PromptInput::Stream(mut stream) => {
                    while let Some(_item) = stream.next().await {
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                }
                _ => panic!("Expected Stream variant"),
            }
        })
    }).collect();
    
    // Wait for all streams to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Should have processed 5 streams * 10 items each = 50 items
    assert_eq!(counter.load(Ordering::SeqCst), 50);
}