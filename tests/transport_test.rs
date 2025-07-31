use claude_code_sdk::types::{ClaudeCodeOptions, PermissionMode, McpServerConfig};
use claude_code_sdk::transport::{SubprocessCliTransport, PromptInput};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_cli_discovery() {
    // This test will pass if Node.js is available and fail with helpful error if not
    let result = SubprocessCliTransport::find_cli();
    
    match result {
        Ok(path) => {
            println!("Found CLI at: {}", path);
            assert!(!path.is_empty());
        }
        Err(e) => {
            // Should provide helpful error messages
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("Claude Code CLI not found") || error_msg.contains("Node.js"));
            println!("Expected error: {}", error_msg);
        }
    }
}

#[test]
fn test_command_building() {
    let mut options = ClaudeCodeOptions::default();
    options.allowed_tools = vec!["file_editor".to_string(), "bash".to_string()];
    options.max_thinking_tokens = 1000;
    options.system_prompt = Some("You are a helpful assistant".to_string());
    options.permission_mode = Some(PermissionMode::AcceptEdits);
    options.continue_conversation = true;
    options.max_turns = Some(10);
    options.model = Some("claude-3-5-sonnet-20241022".to_string());
    options.cwd = Some(PathBuf::from("/tmp"));
    
    // Add MCP server config
    let mut mcp_servers = HashMap::new();
    mcp_servers.insert(
        "test_server".to_string(),
        McpServerConfig::Stdio {
            command: "python".to_string(),
            args: Some(vec!["-m".to_string(), "test_server".to_string()]),
            env: Some({
                let mut env = HashMap::new();
                env.insert("DEBUG".to_string(), "1".to_string());
                env
            }),
        }
    );
    options.mcp_servers = mcp_servers;
    
    let prompt = PromptInput::Text("Hello, world!".to_string());
    
    // Test with streaming mode
    let transport_result = SubprocessCliTransport::new(
        prompt,
        options.clone(),
        Some(PathBuf::from("/usr/bin/claude")), // Custom CLI path
        false, // Don't close stdin after prompt
    );
    
    match transport_result {
        Ok(transport) => {
            let args = transport.build_command();
            
            // Verify basic structure
            assert_eq!(args[0], "code");
            
            // Check for streaming flag (should be present for Stream input)
            // Note: This transport was created with Text input, so no streaming flag
            
            // Check for allowed tools
            assert!(args.contains(&"--allowed-tools".to_string()));
            let tools_index = args.iter().position(|x| x == "--allowed-tools").unwrap();
            assert_eq!(args[tools_index + 1], "file_editor,bash");
            
            // Check for max thinking tokens
            assert!(args.contains(&"--max-thinking-tokens".to_string()));
            let tokens_index = args.iter().position(|x| x == "--max-thinking-tokens").unwrap();
            assert_eq!(args[tokens_index + 1], "1000");
            
            // Check for system prompt
            assert!(args.contains(&"--system-prompt".to_string()));
            let prompt_index = args.iter().position(|x| x == "--system-prompt").unwrap();
            assert_eq!(args[prompt_index + 1], "You are a helpful assistant");
            
            // Check for permission mode
            assert!(args.contains(&"--permission-mode".to_string()));
            let perm_index = args.iter().position(|x| x == "--permission-mode").unwrap();
            assert_eq!(args[perm_index + 1], "acceptEdits");
            
            // Check for continue flag
            assert!(args.contains(&"--continue".to_string()));
            
            // Check for max turns
            assert!(args.contains(&"--max-turns".to_string()));
            let turns_index = args.iter().position(|x| x == "--max-turns").unwrap();
            assert_eq!(args[turns_index + 1], "10");
            
            // Check for model
            assert!(args.contains(&"--model".to_string()));
            let model_index = args.iter().position(|x| x == "--model").unwrap();
            assert_eq!(args[model_index + 1], "claude-3-5-sonnet-20241022");
            
            // Check for working directory
            assert!(args.contains(&"--cwd".to_string()));
            let cwd_index = args.iter().position(|x| x == "--cwd").unwrap();
            assert_eq!(args[cwd_index + 1], "/tmp");
            
            // Check for MCP server configuration
            assert!(args.contains(&"--mcp-server".to_string()));
            let mcp_index = args.iter().position(|x| x == "--mcp-server").unwrap();
            let mcp_config = &args[mcp_index + 1];
            assert!(mcp_config.starts_with("test_server=stdio:python"));
            assert!(mcp_config.contains("args=-m,test_server"));
            assert!(mcp_config.contains("env=DEBUG=1"));
            
            println!("Generated command args: {:?}", args);
        }
        Err(e) => {
            println!("Transport creation failed (expected if CLI not found): {}", e);
        }
    }
}

#[test]
fn test_streaming_vs_string_mode() {
    let options = ClaudeCodeOptions::default();
    
    // Test with text input (non-streaming)
    let text_prompt = PromptInput::Text("Hello".to_string());
    if let Ok(transport) = SubprocessCliTransport::new(
        text_prompt,
        options.clone(),
        Some(PathBuf::from("/usr/bin/claude")),
        true,
    ) {
        let args = transport.build_command();
        assert!(!args.contains(&"--streaming".to_string()));
    }
    
    // Test with stream input (streaming mode)
    let stream_prompt = PromptInput::Stream(Box::pin(
        tokio_stream::iter(vec![serde_json::json!({"test": "data"})])
    ));
    
    if let Ok(transport) = SubprocessCliTransport::new(
        stream_prompt,
        options,
        Some(PathBuf::from("/usr/bin/claude")),
        false,
    ) {
        let args = transport.build_command();
        assert!(args.contains(&"--streaming".to_string()));
    }
}

#[test]
fn test_mcp_server_config_serialization() {
    let options = ClaudeCodeOptions::default();
    let prompt = PromptInput::Text("test".to_string());
    
    if let Ok(_transport) = SubprocessCliTransport::new(
        prompt,
        options,
        Some(PathBuf::from("/usr/bin/claude")),
        true,
    ) {
        // Test stdio config
        let stdio_config = McpServerConfig::Stdio {
            command: "python".to_string(),
            args: Some(vec!["script.py".to_string()]),
            env: Some({
                let mut env = HashMap::new();
                env.insert("VAR1".to_string(), "value1".to_string());
                env.insert("VAR2".to_string(), "value2".to_string());
                env
            }),
        };
        
        let serialized = SubprocessCliTransport::serialize_mcp_server_config(&stdio_config);
        assert!(serialized.starts_with("stdio:python"));
        assert!(serialized.contains("args=script.py"));
        assert!(serialized.contains("env="));
        assert!(serialized.contains("VAR1=value1"));
        assert!(serialized.contains("VAR2=value2"));
        
        // Test SSE config
        let sse_config = McpServerConfig::Sse {
            url: "https://example.com/sse".to_string(),
            headers: Some({
                let mut headers = HashMap::new();
                headers.insert("Authorization".to_string(), "Bearer token".to_string());
                headers
            }),
        };
        
        let serialized = SubprocessCliTransport::serialize_mcp_server_config(&sse_config);
        assert!(serialized.starts_with("sse:https://example.com/sse"));
        assert!(serialized.contains("headers=Authorization=Bearer token"));
        
        // Test HTTP config
        let http_config = McpServerConfig::Http {
            url: "https://api.example.com".to_string(),
            headers: None,
        };
        
        let serialized = SubprocessCliTransport::serialize_mcp_server_config(&http_config);
        assert_eq!(serialized, "http:https://api.example.com");
    }
}