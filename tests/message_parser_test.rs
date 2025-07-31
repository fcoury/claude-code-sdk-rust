use claude_code_sdk::message_parser::parse_message;
use claude_code_sdk::types::*;
use claude_code_sdk::errors::SdkError;
use serde_json::json;

#[test]
fn test_parse_user_message_with_text_content() {
    let json_data = json!({
        "type": "user",
        "content": "Hello, Claude!"
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::User(user_msg) => {
            match user_msg.content {
                MessageContent::Text(text) => {
                    assert_eq!(text, "Hello, Claude!");
                }
                _ => panic!("Expected text content"),
            }
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_user_message_with_block_content() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "text",
                "text": "Hello, Claude!"
            }
        ]
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::User(user_msg) => {
            match user_msg.content {
                MessageContent::Blocks(blocks) => {
                    assert_eq!(blocks.len(), 1);
                    match &blocks[0] {
                        ContentBlock::Text(text_block) => {
                            assert_eq!(text_block.text, "Hello, Claude!");
                        }
                        _ => panic!("Expected text block"),
                    }
                }
                _ => panic!("Expected block content"),
            }
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_assistant_message() {
    let json_data = json!({
        "type": "assistant",
        "content": [
            {
                "type": "text",
                "text": "Hello! How can I help you?"
            },
            {
                "type": "tool_use",
                "id": "tool_123",
                "name": "calculator",
                "input": {
                    "expression": "2 + 2"
                }
            }
        ]
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::Assistant(assistant_msg) => {
            assert_eq!(assistant_msg.content.len(), 2);
            
            // Check text block
            match &assistant_msg.content[0] {
                ContentBlock::Text(text_block) => {
                    assert_eq!(text_block.text, "Hello! How can I help you?");
                }
                _ => panic!("Expected text block"),
            }
            
            // Check tool use block
            match &assistant_msg.content[1] {
                ContentBlock::ToolUse(tool_block) => {
                    assert_eq!(tool_block.id, "tool_123");
                    assert_eq!(tool_block.name, "calculator");
                    assert_eq!(tool_block.input.get("expression").unwrap(), "2 + 2");
                }
                _ => panic!("Expected tool use block"),
            }
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_system_message() {
    let json_data = json!({
        "type": "system",
        "subtype": "session_start",
        "data": {
            "session_id": "session_123",
            "timestamp": "2024-01-01T00:00:00Z"
        }
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::System(system_msg) => {
            assert_eq!(system_msg.subtype, "session_start");
            assert_eq!(system_msg.data.get("session_id").unwrap(), "session_123");
            assert_eq!(system_msg.data.get("timestamp").unwrap(), "2024-01-01T00:00:00Z");
        }
        _ => panic!("Expected system message"),
    }
}

#[test]
fn test_parse_result_message() {
    let json_data = json!({
        "type": "result",
        "subtype": "query_complete",
        "duration_ms": 1500,
        "duration_api_ms": 1200,
        "is_error": false,
        "num_turns": 3,
        "session_id": "session_123",
        "total_cost_usd": 0.05,
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50
        },
        "result": "Task completed successfully"
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::Result(result_msg) => {
            assert_eq!(result_msg.subtype, "query_complete");
            assert_eq!(result_msg.duration_ms, 1500);
            assert_eq!(result_msg.duration_api_ms, 1200);
            assert_eq!(result_msg.is_error, false);
            assert_eq!(result_msg.num_turns, 3);
            assert_eq!(result_msg.session_id, "session_123");
            assert_eq!(result_msg.total_cost_usd, Some(0.05));
            assert!(result_msg.usage.is_some());
            assert_eq!(result_msg.result, Some("Task completed successfully".to_string()));
        }
        _ => panic!("Expected result message"),
    }
}

#[test]
fn test_parse_tool_result_block_with_text_content() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "tool_123",
                "content": "The result is 4",
                "is_error": false
            }
        ]
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::User(user_msg) => {
            match user_msg.content {
                MessageContent::Blocks(blocks) => {
                    match &blocks[0] {
                        ContentBlock::ToolResult(tool_result) => {
                            assert_eq!(tool_result.tool_use_id, "tool_123");
                            assert_eq!(tool_result.is_error, Some(false));
                            match &tool_result.content {
                                Some(ToolResultContent::Text(text)) => {
                                    assert_eq!(text, "The result is 4");
                                }
                                _ => panic!("Expected text content"),
                            }
                        }
                        _ => panic!("Expected tool result block"),
                    }
                }
                _ => panic!("Expected block content"),
            }
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_tool_result_block_with_structured_content() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "tool_123",
                "content": [
                    {
                        "type": "file",
                        "name": "test.txt"
                    },
                    {
                        "type": "directory",
                        "name": "src"
                    }
                ],
                "is_error": false
            }
        ]
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::User(user_msg) => {
            match user_msg.content {
                MessageContent::Blocks(blocks) => {
                    match &blocks[0] {
                        ContentBlock::ToolResult(tool_result) => {
                            assert_eq!(tool_result.tool_use_id, "tool_123");
                            match &tool_result.content {
                                Some(ToolResultContent::Structured(structured)) => {
                                    assert_eq!(structured.len(), 2);
                                    assert_eq!(structured[0].get("type").unwrap(), "file");
                                    assert_eq!(structured[0].get("name").unwrap(), "test.txt");
                                    assert_eq!(structured[1].get("type").unwrap(), "directory");
                                    assert_eq!(structured[1].get("name").unwrap(), "src");
                                }
                                _ => panic!("Expected structured content"),
                            }
                        }
                        _ => panic!("Expected tool result block"),
                    }
                }
                _ => panic!("Expected block content"),
            }
        }
        _ => panic!("Expected user message"),
    }
}

#[test]
fn test_parse_message_missing_type_field() {
    let json_data = json!({
        "content": "Hello, Claude!"
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing or invalid 'type' field"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_message_unknown_type() {
    let json_data = json!({
        "type": "unknown_type",
        "content": "Hello, Claude!"
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Unknown message type: unknown_type"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_message_not_json_object() {
    let json_data = json!("not an object");

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Expected JSON object"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_user_message_missing_content() {
    let json_data = json!({
        "type": "user"
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing 'content' field"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_assistant_message_content_not_array() {
    let json_data = json!({
        "type": "assistant",
        "content": "should be array"
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Assistant content must be an array"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_system_message_missing_subtype() {
    let json_data = json!({
        "type": "system",
        "data": {}
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing 'subtype' field"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_result_message_missing_required_fields() {
    let json_data = json!({
        "type": "result",
        "subtype": "query_complete"
        // Missing required fields
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_content_block_unknown_type() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "unknown_block_type",
                "data": "test"
            }
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Unknown content block type: unknown_block_type"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_text_block_missing_text_field() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "text"
                // Missing text field
            }
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing 'text' field in text block"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_tool_use_block_missing_fields() {
    let json_data = json!({
        "type": "assistant",
        "content": [
            {
                "type": "tool_use",
                "id": "tool_123"
                // Missing name field
            }
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing 'name' field in tool_use block"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_tool_result_block_missing_tool_use_id() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "tool_result",
                "content": "result"
                // Missing tool_use_id
            }
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Missing 'tool_use_id' field in tool_result block"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_tool_result_content_invalid_structured() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "tool_123",
                "content": [
                    "not an object" // Should be object in structured content
                ]
            }
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Structured tool result content must be array of objects"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_message_content_invalid_type() {
    let json_data = json!({
        "type": "user",
        "content": 123 // Should be string or array
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Content must be string or array"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_content_block_not_object() {
    let json_data = json!({
        "type": "user",
        "content": [
            "not an object" // Content blocks must be objects
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Content block must be an object"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_tool_result_content_invalid_type() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "tool_123",
                "content": 123 // Should be string or array
            }
        ]
    });

    let result = parse_message(json_data);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        SdkError::MessageParse { message, .. } => {
            assert!(message.contains("Tool result content must be string or array"));
        }
        _ => panic!("Expected MessageParse error"),
    }
}

#[test]
fn test_parse_system_message_with_empty_data() {
    let json_data = json!({
        "type": "system",
        "subtype": "session_start"
        // No data field - should default to empty HashMap
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::System(system_msg) => {
            assert_eq!(system_msg.subtype, "session_start");
            assert!(system_msg.data.is_empty());
        }
        _ => panic!("Expected system message"),
    }
}

#[test]
fn test_parse_result_message_with_optional_fields_none() {
    let json_data = json!({
        "type": "result",
        "subtype": "query_complete",
        "duration_ms": 1500,
        "duration_api_ms": 1200,
        "is_error": false,
        "num_turns": 3,
        "session_id": "session_123"
        // Optional fields omitted
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::Result(result_msg) => {
            assert_eq!(result_msg.subtype, "query_complete");
            assert_eq!(result_msg.duration_ms, 1500);
            assert_eq!(result_msg.duration_api_ms, 1200);
            assert_eq!(result_msg.is_error, false);
            assert_eq!(result_msg.num_turns, 3);
            assert_eq!(result_msg.session_id, "session_123");
            assert_eq!(result_msg.total_cost_usd, None);
            assert_eq!(result_msg.usage, None);
            assert_eq!(result_msg.result, None);
        }
        _ => panic!("Expected result message"),
    }
}

#[test]
fn test_parse_tool_use_block_with_empty_input() {
    let json_data = json!({
        "type": "assistant",
        "content": [
            {
                "type": "tool_use",
                "id": "tool_123",
                "name": "calculator"
                // No input field - should default to empty HashMap
            }
        ]
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::Assistant(assistant_msg) => {
            match &assistant_msg.content[0] {
                ContentBlock::ToolUse(tool_block) => {
                    assert_eq!(tool_block.id, "tool_123");
                    assert_eq!(tool_block.name, "calculator");
                    assert!(tool_block.input.is_empty());
                }
                _ => panic!("Expected tool use block"),
            }
        }
        _ => panic!("Expected assistant message"),
    }
}

#[test]
fn test_parse_tool_result_block_with_optional_fields_none() {
    let json_data = json!({
        "type": "user",
        "content": [
            {
                "type": "tool_result",
                "tool_use_id": "tool_123"
                // Optional content and is_error fields omitted
            }
        ]
    });

    let result = parse_message(json_data).unwrap();
    
    match result {
        Message::User(user_msg) => {
            match user_msg.content {
                MessageContent::Blocks(blocks) => {
                    match &blocks[0] {
                        ContentBlock::ToolResult(tool_result) => {
                            assert_eq!(tool_result.tool_use_id, "tool_123");
                            assert_eq!(tool_result.content, None);
                            assert_eq!(tool_result.is_error, None);
                        }
                        _ => panic!("Expected tool result block"),
                    }
                }
                _ => panic!("Expected block content"),
            }
        }
        _ => panic!("Expected user message"),
    }
}