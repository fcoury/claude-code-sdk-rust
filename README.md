# Claude Code SDK for Rust

A high-performance, memory-safe Rust SDK for interacting with the Claude Code CLI.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
claude-code-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
tokio-stream = "0.1"
```

## Prerequisites

Install the Claude Code CLI:

```bash
npm install -g @anthropic-ai/claude-code
```

## Quick Start

```rust
use claude_code_sdk::query;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = query("Hello, Claude!", None).await;
    
    while let Some(message) = stream.next().await {
        match message? {
            claude_code_sdk::Message::Assistant(msg) => {
                println!("Claude: {:?}", msg.content);
            }
            claude_code_sdk::Message::Result(result) => {
                println!("Completed in {}ms", result.duration_ms);
                break;
            }
            _ => {}
        }
    }
    
    Ok(())
}
```

## Examples

```bash
cargo run --example quick_start
cargo run --example streaming_mode
cargo run --example error_handling
```

## License

MIT License