use claude_code_sdk::*;
use std::collections::HashMap;

#[test]
fn test_message_serialization() {
    // Test UserMessage serialization
    let user_msg = UserMessage {
        content: MessageContent::Text("Hello, Claude!".to_string()),
    };
    let message = Message::User(user_msg);
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"user\""));
    assert!(json.contains("Hello, Claude!"));

    // Test deserialization
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_content_blocks() {
    // Test AssistantMessage with content blocks
    let assistant_msg = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "Hello! How can I help you?".to_string(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool_123".to_string(),
                name: "calculator".to_string(),
                input: {
                    let mut map = HashMap::new();
                    map.insert("expression".to_string(), serde_json::json!("2 + 2"));
                    map
                },
            }),
        ],
    };
    let message = Message::Assistant(assistant_msg);
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"assistant\""));
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("\"type\":\"tool_use\""));
}

#[test]
fn test_builder_pattern() {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec!["calculator".to_string(), "file_editor".to_string()])
        .max_thinking_tokens(1000)
        .build();
    
    assert_eq!(options.system_prompt, Some("You are a helpful assistant".to_string()));
    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
    assert_eq!(options.allowed_tools, vec!["calculator", "file_editor"]);
    assert_eq!(options.max_thinking_tokens, 1000);
}

#[test]
fn test_mcp_server_config() {
    let mcp_config = McpServerConfig::Stdio {
        command: "python".to_string(),
        args: Some(vec!["-m".to_string(), "my_mcp_server".to_string()]),
        env: None,
    };
    let json = serde_json::to_string(&mcp_config).unwrap();
    assert!(json.contains("\"type\":\"stdio\""));
    assert!(json.contains("\"command\":\"python\""));
}