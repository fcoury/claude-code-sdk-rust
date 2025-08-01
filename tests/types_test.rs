use claude_code_sdk::*;
use rstest::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Message Type Serialization/Deserialization Tests
// ============================================================================

#[test]
fn test_user_message_text_content() {
    let user_msg = UserMessage {
        content: MessageContent::Text("Hello, Claude!".to_string()),
    };
    let message = Message::User(user_msg);

    // Test serialization
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"user\""));
    assert!(json.contains("Hello, Claude!"));

    // Test deserialization
    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_user_message_block_content() {
    let user_msg = UserMessage {
        content: MessageContent::Blocks(vec![
            ContentBlock::Text(TextBlock {
                text: "Please calculate this:".to_string(),
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
        ]),
    };
    let message = Message::User(user_msg);

    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"user\""));
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("\"type\":\"tool_use\""));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_assistant_message_serialization() {
    let assistant_msg = AssistantMessage {
        content: vec![
            ContentBlock::Text(TextBlock {
                text: "Hello! How can I help you?".to_string(),
            }),
            ContentBlock::ToolUse(ToolUseBlock {
                id: "tool_456".to_string(),
                name: "file_reader".to_string(),
                input: {
                    let mut map = HashMap::new();
                    map.insert("path".to_string(), serde_json::json!("/tmp/test.txt"));
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
    assert!(json.contains("file_reader"));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_system_message_serialization() {
    let mut data = HashMap::new();
    data.insert("session_id".to_string(), serde_json::json!("session_123"));
    data.insert("timestamp".to_string(), serde_json::json!(1234567890));

    let system_msg = SystemMessage {
        subtype: "session_start".to_string(),
        data,
    };
    let message = Message::System(system_msg);

    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"system\""));
    assert!(json.contains("\"subtype\":\"session_start\""));
    assert!(json.contains("session_123"));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_result_message_serialization() {
    let mut usage = HashMap::new();
    usage.insert("input_tokens".to_string(), serde_json::json!(100));
    usage.insert("output_tokens".to_string(), serde_json::json!(50));

    let result_msg = ResultMessage {
        subtype: "query_complete".to_string(),
        duration_ms: 1500,
        duration_api_ms: 1200,
        is_error: false,
        num_turns: 3,
        session_id: "session_456".to_string(),
        total_cost_usd: Some(0.05),
        usage: Some(usage),
        result: Some("Task completed successfully".to_string()),
    };
    let message = Message::Result(result_msg);

    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"result\""));
    assert!(json.contains("\"subtype\":\"query_complete\""));
    assert!(json.contains("\"duration_ms\":1500"));
    assert!(json.contains("\"is_error\":false"));
    assert!(json.contains("\"total_cost_usd\":0.05"));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_result_message_minimal() {
    let result_msg = ResultMessage {
        subtype: "error".to_string(),
        duration_ms: 500,
        duration_api_ms: 400,
        is_error: true,
        num_turns: 1,
        session_id: "session_error".to_string(),
        total_cost_usd: None,
        usage: None,
        result: None,
    };
    let message = Message::Result(result_msg);

    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"result\""));
    assert!(json.contains("\"is_error\":true"));
    assert!(!json.contains("total_cost_usd"));
    assert!(!json.contains("usage"));
    // Note: "result" appears in the JSON structure itself, so we need to check for the field specifically
    assert!(!json.contains("\"result\":"));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

// ============================================================================
// Content Block Tests
// ============================================================================

#[test]
fn test_text_block_serialization() {
    let text_block = ContentBlock::Text(TextBlock {
        text: "This is a text block".to_string(),
    });

    let json = serde_json::to_string(&text_block).unwrap();
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("This is a text block"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(text_block, deserialized);
}

#[test]
fn test_tool_use_block_serialization() {
    let mut input = HashMap::new();
    input.insert("param1".to_string(), serde_json::json!("value1"));
    input.insert("param2".to_string(), serde_json::json!(42));
    input.insert("param3".to_string(), serde_json::json!(true));

    let tool_use_block = ContentBlock::ToolUse(ToolUseBlock {
        id: "tool_789".to_string(),
        name: "complex_tool".to_string(),
        input,
    });

    let json = serde_json::to_string(&tool_use_block).unwrap();
    assert!(json.contains("\"type\":\"tool_use\""));
    assert!(json.contains("\"id\":\"tool_789\""));
    assert!(json.contains("\"name\":\"complex_tool\""));
    assert!(json.contains("\"param1\":\"value1\""));
    assert!(json.contains("\"param2\":42"));
    assert!(json.contains("\"param3\":true"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(tool_use_block, deserialized);
}

#[test]
fn test_tool_result_block_text_content() {
    let tool_result_block = ContentBlock::ToolResult(ToolResultBlock {
        tool_use_id: "tool_789".to_string(),
        content: Some(ToolResultContent::Text("Operation completed".to_string())),
        is_error: Some(false),
    });

    let json = serde_json::to_string(&tool_result_block).unwrap();
    assert!(json.contains("\"type\":\"tool_result\""));
    assert!(json.contains("\"tool_use_id\":\"tool_789\""));
    assert!(json.contains("Operation completed"));
    assert!(json.contains("\"is_error\":false"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(tool_result_block, deserialized);
}

#[test]
fn test_tool_result_block_structured_content() {
    let mut structured_data = HashMap::new();
    structured_data.insert("status".to_string(), serde_json::json!("success"));
    structured_data.insert("data".to_string(), serde_json::json!({"key": "value"}));

    let tool_result_block = ContentBlock::ToolResult(ToolResultBlock {
        tool_use_id: "tool_structured".to_string(),
        content: Some(ToolResultContent::Structured(vec![structured_data])),
        is_error: None,
    });

    let json = serde_json::to_string(&tool_result_block).unwrap();
    assert!(json.contains("\"type\":\"tool_result\""));
    assert!(json.contains("\"tool_use_id\":\"tool_structured\""));
    assert!(json.contains("\"status\":\"success\""));
    assert!(!json.contains("is_error"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(tool_result_block, deserialized);
}

#[test]
fn test_tool_result_block_minimal() {
    let tool_result_block = ContentBlock::ToolResult(ToolResultBlock {
        tool_use_id: "tool_minimal".to_string(),
        content: None,
        is_error: None,
    });

    let json = serde_json::to_string(&tool_result_block).unwrap();
    assert!(json.contains("\"type\":\"tool_result\""));
    assert!(json.contains("\"tool_use_id\":\"tool_minimal\""));
    assert!(!json.contains("content"));
    assert!(!json.contains("is_error"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(tool_result_block, deserialized);
}

// ============================================================================
// Message Content Tests
// ============================================================================

#[test]
fn test_message_content_text_variant() {
    let content = MessageContent::Text("Simple text content".to_string());

    let json = serde_json::to_string(&content).unwrap();
    assert_eq!(json, "\"Simple text content\"");

    let deserialized: MessageContent = serde_json::from_str(&json).unwrap();
    assert_eq!(content, deserialized);
}

#[test]
fn test_message_content_blocks_variant() {
    let content = MessageContent::Blocks(vec![
        ContentBlock::Text(TextBlock {
            text: "First block".to_string(),
        }),
        ContentBlock::Text(TextBlock {
            text: "Second block".to_string(),
        }),
    ]);

    let json = serde_json::to_string(&content).unwrap();
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("First block"));
    assert!(json.contains("Second block"));

    let deserialized: MessageContent = serde_json::from_str(&json).unwrap();
    assert_eq!(content, deserialized);
}

// ============================================================================
// Permission Mode Tests
// ============================================================================

#[rstest]
#[case(PermissionMode::Default, "\"default\"")]
#[case(PermissionMode::AcceptEdits, "\"acceptEdits\"")]
#[case(PermissionMode::BypassPermissions, "\"bypassPermissions\"")]
fn test_permission_mode_serialization(#[case] mode: PermissionMode, #[case] expected_json: &str) {
    let json = serde_json::to_string(&mode).unwrap();
    assert_eq!(json, expected_json);

    let deserialized: PermissionMode = serde_json::from_str(&json).unwrap();
    assert_eq!(mode, deserialized);
}

// ============================================================================
// MCP Server Configuration Tests
// ============================================================================

#[test]
fn test_mcp_server_stdio_full() {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), "/usr/bin".to_string());
    env.insert("DEBUG".to_string(), "1".to_string());

    let config = McpServerConfig::Stdio {
        command: "python".to_string(),
        args: Some(vec![
            "-m".to_string(),
            "my_mcp_server".to_string(),
            "--verbose".to_string(),
        ]),
        env: Some(env),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"type\":\"stdio\""));
    assert!(json.contains("\"command\":\"python\""));
    assert!(json.contains("\"-m\""));
    assert!(json.contains("\"my_mcp_server\""));
    assert!(json.contains("\"--verbose\""));
    assert!(json.contains("\"PATH\":\"/usr/bin\""));
    assert!(json.contains("\"DEBUG\":\"1\""));

    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn test_mcp_server_stdio_minimal() {
    let config = McpServerConfig::Stdio {
        command: "node".to_string(),
        args: None,
        env: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"type\":\"stdio\""));
    assert!(json.contains("\"command\":\"node\""));
    assert!(!json.contains("args"));
    assert!(!json.contains("env"));

    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn test_mcp_server_sse_full() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let config = McpServerConfig::Sse {
        url: "https://api.example.com/sse".to_string(),
        headers: Some(headers),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"type\":\"sse\""));
    assert!(json.contains("\"url\":\"https://api.example.com/sse\""));
    assert!(json.contains("\"Authorization\":\"Bearer token123\""));
    assert!(json.contains("\"Content-Type\":\"application/json\""));

    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn test_mcp_server_sse_minimal() {
    let config = McpServerConfig::Sse {
        url: "https://simple.example.com".to_string(),
        headers: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"type\":\"sse\""));
    assert!(json.contains("\"url\":\"https://simple.example.com\""));
    assert!(!json.contains("headers"));

    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
fn test_mcp_server_http_full() {
    let mut headers = HashMap::new();
    headers.insert("X-API-Key".to_string(), "secret123".to_string());

    let config = McpServerConfig::Http {
        url: "https://api.example.com/mcp".to_string(),
        headers: Some(headers),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"type\":\"http\""));
    assert!(json.contains("\"url\":\"https://api.example.com/mcp\""));
    assert!(json.contains("\"X-API-Key\":\"secret123\""));

    let deserialized: McpServerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config, deserialized);
}

// ============================================================================
// ClaudeCodeOptions Builder Pattern Tests
// ============================================================================

#[test]
fn test_builder_pattern_comprehensive() {
    let mut mcp_servers = HashMap::new();
    mcp_servers.insert(
        "server1".to_string(),
        McpServerConfig::Stdio {
            command: "python".to_string(),
            args: Some(vec!["-m".to_string(), "server1".to_string()]),
            env: None,
        },
    );

    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant")
        .append_system_prompt("Be concise and accurate")
        .permission_mode(PermissionMode::AcceptEdits)
        .allowed_tools(vec!["calculator".to_string(), "file_editor".to_string()])
        .disallowed_tools(vec!["dangerous_tool".to_string()])
        .max_thinking_tokens(2000)
        .mcp_tools(vec!["tool1".to_string(), "tool2".to_string()])
        .mcp_server(
            "server1".to_string(),
            McpServerConfig::Stdio {
                command: "python".to_string(),
                args: Some(vec!["-m".to_string(), "server1".to_string()]),
                env: None,
            },
        )
        .continue_conversation(true)
        .resume("session_123".to_string())
        .max_turns(10)
        .model("claude-3-sonnet")
        .permission_prompt_tool_name("permission_tool")
        .cwd("/tmp/workspace")
        .settings("/path/to/settings.json")
        .build();

    assert_eq!(
        options.system_prompt,
        Some("You are a helpful assistant".to_string())
    );
    assert_eq!(
        options.append_system_prompt,
        Some("Be concise and accurate".to_string())
    );
    assert_eq!(options.permission_mode, Some(PermissionMode::AcceptEdits));
    assert_eq!(options.allowed_tools, vec!["calculator", "file_editor"]);
    assert_eq!(options.disallowed_tools, vec!["dangerous_tool"]);
    assert_eq!(options.max_thinking_tokens, 2000);
    assert_eq!(options.mcp_tools, vec!["tool1", "tool2"]);
    assert_eq!(options.mcp_servers.len(), 1);
    assert!(options.mcp_servers.contains_key("server1"));
    assert!(options.continue_conversation);
    assert_eq!(options.resume, Some("session_123".to_string()));
    assert_eq!(options.max_turns, Some(10));
    assert_eq!(options.model, Some("claude-3-sonnet".to_string()));
    assert_eq!(
        options.permission_prompt_tool_name,
        Some("permission_tool".to_string())
    );
    assert_eq!(options.cwd, Some(PathBuf::from("/tmp/workspace")));
    assert_eq!(options.settings, Some("/path/to/settings.json".to_string()));
}

#[test]
fn test_builder_pattern_defaults() {
    let options = ClaudeCodeOptions::builder().build();

    assert_eq!(options.system_prompt, None);
    assert_eq!(options.append_system_prompt, None);
    assert_eq!(options.permission_mode, None);
    assert!(options.allowed_tools.is_empty());
    assert!(options.disallowed_tools.is_empty());
    assert_eq!(options.max_thinking_tokens, 0);
    assert!(options.mcp_tools.is_empty());
    assert!(options.mcp_servers.is_empty());
    assert!(!options.continue_conversation);
    assert_eq!(options.resume, None);
    assert_eq!(options.max_turns, None);
    assert_eq!(options.model, None);
    assert_eq!(options.permission_prompt_tool_name, None);
    assert_eq!(options.cwd, None);
    assert_eq!(options.settings, None);
}

#[test]
fn test_builder_pattern_method_chaining() {
    // Test that builder methods can be chained in any order
    let options1 = ClaudeCodeOptions::builder()
        .system_prompt("Test")
        .max_thinking_tokens(1000)
        .permission_mode(PermissionMode::Default)
        .build();

    let options2 = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::Default)
        .max_thinking_tokens(1000)
        .system_prompt("Test")
        .build();

    assert_eq!(options1.system_prompt, options2.system_prompt);
    assert_eq!(options1.max_thinking_tokens, options2.max_thinking_tokens);
    assert_eq!(options1.permission_mode, options2.permission_mode);
}

#[test]
fn test_builder_pattern_string_conversions() {
    // Test that builder accepts various string types
    let string_owned = "owned string".to_string();
    let string_literal = "literal string";

    let options = ClaudeCodeOptions::builder()
        .system_prompt(string_owned)
        .model(string_literal)
        .resume("inline string".to_string())
        .build();

    assert_eq!(options.system_prompt, Some("owned string".to_string()));
    assert_eq!(options.model, Some("literal string".to_string()));
    assert_eq!(options.resume, Some("inline string".to_string()));
}

#[test]
fn test_builder_pattern_path_conversions() {
    // Test that builder accepts various path types
    let path_buf = PathBuf::from("/tmp/test");
    let path_str = "/tmp/test2";

    let options = ClaudeCodeOptions::builder().cwd(path_buf.clone()).build();

    assert_eq!(options.cwd, Some(path_buf));

    let options2 = ClaudeCodeOptions::builder().cwd(path_str).build();

    assert_eq!(options2.cwd, Some(PathBuf::from(path_str)));
}

// ============================================================================
// Edge Cases and Error Conditions
// ============================================================================

#[test]
fn test_empty_collections_serialization() {
    let assistant_msg = AssistantMessage {
        content: vec![], // Empty content blocks
    };
    let message = Message::Assistant(assistant_msg);

    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"type\":\"assistant\""));
    assert!(json.contains("\"content\":[]"));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(message, deserialized);
}

#[test]
fn test_large_input_data_serialization() {
    let large_text = "x".repeat(10000); // 10KB of text
    let mut large_input = HashMap::new();
    large_input.insert("large_data".to_string(), serde_json::json!(large_text));

    let tool_use_block = ContentBlock::ToolUse(ToolUseBlock {
        id: "large_tool".to_string(),
        name: "data_processor".to_string(),
        input: large_input,
    });

    let json = serde_json::to_string(&tool_use_block).unwrap();
    assert!(json.len() > 10000);
    assert!(json.contains("\"type\":\"tool_use\""));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(tool_use_block, deserialized);
}

#[test]
fn test_unicode_content_serialization() {
    let unicode_text = "Hello 世界! 🌍 Здравствуй мир! مرحبا بالعالم!";
    let text_block = ContentBlock::Text(TextBlock {
        text: unicode_text.to_string(),
    });

    let json = serde_json::to_string(&text_block).unwrap();
    assert!(json.contains("世界"));
    assert!(json.contains("🌍"));
    assert!(json.contains("мир"));
    assert!(json.contains("مرحبا"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(text_block, deserialized);
}

#[test]
fn test_nested_json_in_tool_input() {
    let mut nested_data = HashMap::new();
    nested_data.insert(
        "level1".to_string(),
        serde_json::json!({
            "level2": {
                "level3": ["item1", "item2", "item3"],
                "number": 42,
                "boolean": true,
                "null_value": null
            }
        }),
    );

    let tool_use_block = ContentBlock::ToolUse(ToolUseBlock {
        id: "nested_tool".to_string(),
        name: "json_processor".to_string(),
        input: nested_data,
    });

    let json = serde_json::to_string(&tool_use_block).unwrap();
    assert!(json.contains("\"level1\""));
    assert!(json.contains("\"level2\""));
    assert!(json.contains("\"level3\""));
    assert!(json.contains("\"item1\""));
    assert!(json.contains("\"number\":42"));
    assert!(json.contains("\"boolean\":true"));
    assert!(json.contains("\"null_value\":null"));

    let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();
    assert_eq!(tool_use_block, deserialized);
}
