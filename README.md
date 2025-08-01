# Claude Code SDK for Rust

[![Crates.io](https://img.shields.io/crates/v/claude-code-sdk.svg)](https://crates.io/crates/claude-code-sdk)
[![Documentation](https://docs.rs/claude-code-sdk/badge.svg)](https://docs.rs/claude-code-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance, memory-safe Rust SDK for interacting with the Claude Code CLI. This crate provides both one-shot queries and interactive bidirectional conversations with Claude, leveraging Rust's type safety and async ecosystem.

## Features

- 🦀 **Type Safety**: Strongly-typed message and configuration structures with compile-time guarantees
- ⚡ **Async Streams**: Native integration with Rust's async ecosystem using `tokio_stream::Stream`
- 🛡️ **Memory Safety**: Automatic resource management with explicit cleanup options
- 🔧 **Error Handling**: Comprehensive error types with actionable error messages
- 🏗️ **Flexible Configuration**: Builder pattern for ergonomic configuration
- 🔍 **CLI Integration**: Automatic CLI discovery with helpful installation guidance
- 📊 **Performance**: Minimal overhead with efficient JSON parsing and buffering

## Installation

### Prerequisites

1. **Node.js** (version 18 or higher): Required for running the Claude Code CLI
   ```bash
   # macOS with Homebrew
   brew install node
   
   # Or download from https://nodejs.org/
   ```

2. **Claude Code CLI**: The official CLI tool
   ```bash
   npm install -g @anthropic-ai/claude-code
   ```

### Add to Your Project

Add this to your `Cargo.toml`:

```toml
[dependencies]
claude-code-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
tokio-stream = "0.1"
```

## Quick Start

### One-Shot Queries

For simple, stateless interactions, use the `query` function:

```rust
use claude_code_sdk::{query, Message};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> claude_code_sdk::Result<()> {
    let stream = query("What is Rust?", None).await;
    tokio::pin!(stream);
    
    while let Some(message) = stream.next().await {
        match message? {
            Message::Assistant(msg) => {
                for content_block in &msg.content {
                    if let claude_code_sdk::ContentBlock::Text(text_block) = content_block {
                        println!("Claude: {}", text_block.text);
                    }
                }
            }
            Message::Result(result) => {
                println!("✅ Query completed in {}ms", result.duration_ms);
                break;
            }
            _ => {} // Handle other message types as needed
        }
    }
    
    Ok(())
}
```

### Interactive Sessions

For bidirectional conversations with session state, use `ClaudeSDKClient`:

```rust
use claude_code_sdk::{ClaudeSDKClient, PromptInput, Message};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> claude_code_sdk::Result<()> {
    let mut client = ClaudeSDKClient::new(None);
    client.connect(None).await?;
    
    // Send first message
    client.query(PromptInput::from("Hello, Claude!"), None).await?;
    
    // Receive response
    let responses = client.receive_response().await?;
    tokio::pin!(responses);
    
    while let Some(message) = responses.next().await {
        match message? {
            Message::Assistant(msg) => {
                println!("Claude: {:?}", msg.content);
            }
            Message::Result(_) => break,
            _ => {}
        }
    }
    
    // Always disconnect explicitly for guaranteed cleanup
    client.disconnect().await?;
    Ok(())
}
```

## Configuration

Use the builder pattern for advanced configuration:

```rust
use claude_code_sdk::{query, ClaudeCodeOptions, PermissionMode};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> claude_code_sdk::Result<()> {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful coding assistant")
        .permission_mode(PermissionMode::AcceptEdits)
        .cwd(PathBuf::from("./my-project"))
        .allowed_tools(vec!["file_editor".to_string(), "bash".to_string()])
        .max_thinking_tokens(2000)
        .build();
    
    let stream = query("Help me refactor this code", Some(options)).await;
    // Process stream...
    Ok(())
}
```

### Configuration Options

| Option | Type | Description |
|--------|------|-------------|
| `system_prompt` | `String` | Custom system prompt for Claude |
| `append_system_prompt` | `String` | Additional system prompt to append |
| `permission_mode` | `PermissionMode` | How to handle permission requests |
| `cwd` | `PathBuf` | Working directory for file operations |
| `allowed_tools` | `Vec<String>` | Tools Claude is allowed to use |
| `disallowed_tools` | `Vec<String>` | Tools Claude cannot use |
| `max_thinking_tokens` | `i32` | Maximum tokens for Claude's thinking |
| `max_turns` | `i32` | Maximum conversation turns |
| `mcp_servers` | `HashMap` | MCP server configurations |

## Error Handling

The SDK provides comprehensive error handling with actionable messages:

```rust
use claude_code_sdk::{query, SdkError};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let stream = query("Hello, Claude!", None).await;
    tokio::pin!(stream);
    
    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok(message) => {
                // Process successful message
                println!("Received: {:?}", message);
            }
            Err(SdkError::CliNotFound(_)) => {
                eprintln!("❌ Claude Code CLI not found!");
                eprintln!("💡 Install with: npm install -g @anthropic-ai/claude-code");
                break;
            }
            Err(SdkError::NodeJsNotFound) => {
                eprintln!("❌ Node.js not found!");
                eprintln!("💡 Download from: https://nodejs.org/");
                break;
            }
            Err(e) => {
                eprintln!("❌ Error: {}", e);
                eprintln!("📊 Category: {}", e.category());
                eprintln!("🔄 Recoverable: {}", e.is_recoverable());
                
                if e.is_recoverable() {
                    eprintln!("💡 This error might be resolved by retrying");
                }
                break;
            }
        }
    }
}
```

### Error Categories

- **CLI Errors**: Issues with finding or running the Claude Code CLI
- **Process Errors**: Problems with subprocess management
- **Parsing Errors**: JSON and message parsing failures
- **Transport Errors**: Communication layer issues
- **Configuration Errors**: Invalid options or settings
- **Session Errors**: Interactive session management problems

## Message Types

The SDK handles several message types in the Claude Code protocol:

### Message Enum

```rust
pub enum Message {
    User(UserMessage),       // Messages from user to Claude
    Assistant(AssistantMessage), // Responses from Claude
    System(SystemMessage),   // Metadata and control information
    Result(ResultMessage),   // Query completion with metadata
}
```

### Content Blocks

Claude's responses contain structured content blocks:

```rust
pub enum ContentBlock {
    Text(TextBlock),         // Plain text content
    ToolUse(ToolUseBlock),   // Tool usage requests
    ToolResult(ToolResultBlock), // Tool execution results
}
```

## Examples

The repository includes comprehensive examples:

### Run Examples

```bash
# Basic one-shot query
cargo run --example quick_start

# Interactive session with multiple exchanges
cargo run --example streaming_mode

# Comprehensive error handling patterns
cargo run --example error_handling
```

### Example Descriptions

- **`quick_start.rs`**: Demonstrates basic usage with the `query` function
- **`streaming_mode.rs`**: Shows interactive sessions with `ClaudeSDKClient`
- **`error_handling.rs`**: Comprehensive error handling and recovery patterns

## Best Practices

### Resource Management

1. **Always disconnect explicitly** for guaranteed cleanup:
   ```rust
   let mut client = ClaudeSDKClient::new(None);
   client.connect(None).await?;
   
   // ... use client ...
   
   // Guaranteed cleanup
   client.disconnect().await?;
   ```

2. **Use RAII patterns** for automatic cleanup:
   ```rust
   {
       let mut client = ClaudeSDKClient::new(None);
       client.connect(None).await?;
       // ... use client ...
   } // Client automatically cleaned up (best-effort)
   ```

### Error Handling

1. **Check error recoverability** for smart retry logic:
   ```rust
   match result {
       Err(e) if e.is_recoverable() => {
           // Implement retry logic
       }
       Err(e) => {
           // Handle permanent failure
       }
       Ok(value) => {
           // Process success
       }
   }
   ```

2. **Use error categories** for different handling strategies:
   ```rust
   match error.category() {
       "cli" => handle_cli_error(&error),
       "transport" => handle_transport_error(&error),
       "session" => handle_session_error(&error),
       _ => handle_generic_error(&error),
   }
   ```

### Performance

1. **Use one-shot queries** for simple interactions:
   ```rust
   // Efficient for single queries
   let stream = query("Simple question", None).await;
   ```

2. **Use interactive clients** for multiple exchanges:
   ```rust
   // Efficient for conversations
   let mut client = ClaudeSDKClient::new(None);
   client.connect(None).await?;
   // ... multiple queries ...
   client.disconnect().await?;
   ```

3. **Process streams incrementally** to avoid memory buildup:
   ```rust
   while let Some(message) = stream.next().await {
       // Process each message immediately
       process_message(message?).await;
   }
   ```

## API Reference

### Core Functions

- [`query`]: Execute a one-shot query with automatic resource management
- [`ClaudeSDKClient::new`]: Create a new interactive client
- [`ClaudeSDKClient::connect`]: Establish connection to CLI
- [`ClaudeSDKClient::query`]: Send messages in interactive mode
- [`ClaudeSDKClient::receive_messages`]: Receive all messages as stream
- [`ClaudeSDKClient::receive_response`]: Receive messages until Result
- [`ClaudeSDKClient::interrupt`]: Send interrupt signal
- [`ClaudeSDKClient::disconnect`]: Explicitly disconnect and cleanup

### Configuration

- [`ClaudeCodeOptions`]: Main configuration struct
- [`ClaudeCodeOptionsBuilder`]: Builder for ergonomic configuration
- [`PermissionMode`]: Permission handling modes
- [`McpServerConfig`]: MCP server configuration variants

### Error Types

- [`SdkError`]: Main error enum with all failure modes
- [`Result<T>`]: Type alias for `std::result::Result<T, SdkError>`

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
git clone https://github.com/anthropics/claude-code-sdk-rust
cd claude-code-sdk-rust
cargo build
cargo test
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests (requires Claude Code CLI)
cargo test --test integration_test

# All tests with output
cargo test -- --nocapture
```

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and changes.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- 📖 [Documentation](https://docs.rs/claude-code-sdk)
- 🐛 [Issue Tracker](https://github.com/anthropics/claude-code-sdk-rust/issues)
- 💬 [Discussions](https://github.com/anthropics/claude-code-sdk-rust/discussions)
- 📧 [Email Support](mailto:support@anthropic.com)

## Related Projects

- [Claude Code CLI](https://github.com/anthropics/claude-code) - The official CLI tool
- [Claude Python SDK](https://github.com/anthropics/claude-code-python) - Python implementation
- [Anthropic API](https://github.com/anthropics/anthropic-sdk-rust) - Direct API access