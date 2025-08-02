# Changelog

All notable changes to the Claude Code SDK for Rust will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of Claude Code SDK for Rust
- One-shot query functionality via `query()` function
- Interactive client via `ClaudeSDKClient`
- Comprehensive error handling with `SdkError` enum
- Type-safe message parsing and serialization
- Automatic CLI discovery and process management
- Builder pattern for configuration options
- Async stream-based message processing
- Resource management with explicit cleanup
- Comprehensive examples and documentation

### Fixed
- CLI integration issues with argument generation and process management
- Message parsing for nested CLI output format (message.content structure)
- JSON parsing errors from tool output and trailing content
- Transport layer CLI flag usage (--print, --allowedTools, --append-system-prompt)
- Streaming functionality in examples (quick_start and streaming_mode now work)
- Removed unsupported CLI options that caused process failures

### Features
- **Type Safety**: Strongly-typed message and configuration structures
- **Async Streams**: Native integration with `tokio_stream::Stream`
- **Memory Safety**: Automatic resource management with RAII patterns
- **Error Handling**: Detailed error types with actionable guidance
- **CLI Integration**: Automatic discovery with helpful error messages
- **Performance**: Efficient JSON parsing and buffering

## [0.1.0] - 2024-01-XX

### Added
- Initial implementation of Claude Code SDK for Rust
- Core message types: `UserMessage`, `AssistantMessage`, `SystemMessage`, `ResultMessage`
- Content block system: `TextBlock`, `ToolUseBlock`, `ToolResultBlock`
- Configuration system with `ClaudeCodeOptions` and builder pattern
- Transport layer with subprocess CLI communication
- JSON message parsing and validation
- Comprehensive error system with recovery guidance
- One-shot query API for simple interactions
- Interactive client API for bidirectional conversations
- Stream-based message processing with backpressure handling
- Automatic resource cleanup with explicit disconnect options
- CLI discovery with Node.js dependency checking
- Permission mode handling for edit operations
- MCP server configuration support
- Buffer size limits and memory protection
- Control flow features (interrupt support)
- Comprehensive test suite with mock CLI
- Runnable examples demonstrating all features
- Complete API documentation with examples

### Documentation
- Comprehensive README with installation and usage guide
- API documentation with examples for all public functions
- Contributing guidelines and development setup
- Error handling patterns and best practices
- Performance optimization recommendations
- Architecture overview and design principles

### Examples
- `quick_start.rs`: Basic one-shot query usage
- `streaming_mode.rs`: Interactive client with multiple exchanges
- `error_handling.rs`: Comprehensive error handling patterns

### Dependencies
- `tokio`: Async runtime and I/O operations
- `tokio-stream`: Stream utilities and async iteration
- `serde`: Serialization and deserialization
- `serde_json`: JSON parsing and generation
- `thiserror`: Error type derivation
- `which`: CLI discovery and path resolution
- `async-stream`: Async stream generation macros

### Requirements
- Rust 1.70 or higher
- Node.js 18 or higher
- Claude Code CLI (`npm install -g @anthropic-ai/claude-code`)

### Breaking Changes
- N/A (initial release)

### Migration Guide
- N/A (initial release)

---

## Version History

### Pre-release Development

#### 2024-01-XX - Architecture Design
- Defined core architecture with transport, client, and type layers
- Established error handling strategy with comprehensive error types
- Designed message parsing system with robust JSON buffering
- Created configuration system with builder pattern
- Planned resource management with explicit cleanup options

#### 2024-01-XX - Core Implementation
- Implemented transport layer with subprocess management
- Added JSON message parsing with error recovery
- Created type system with serde serialization
- Built error system with actionable guidance
- Added CLI discovery with helpful error messages

#### 2024-01-XX - Client APIs
- Implemented one-shot query function
- Created interactive client with session management
- Added stream-based message processing
- Implemented control flow features (interrupt)
- Added resource cleanup with Drop trait

#### 2024-01-XX - Testing and Documentation
- Created comprehensive test suite with mock CLI
- Added integration tests for all major features
- Implemented property-based testing for JSON handling
- Created runnable examples for all use cases
- Added complete API documentation

#### 2024-01-XX - Polish and Release Preparation
- Optimized performance and memory usage
- Added comprehensive error handling examples
- Created contributing guidelines and development docs
- Finalized API design and documentation
- Prepared for initial crates.io release

---

## Future Roadmap

### Planned Features

#### v0.2.0
- [ ] Streaming response support for real-time updates
- [ ] Connection pooling for improved performance
- [ ] Advanced retry logic with exponential backoff
- [ ] Metrics and observability features
- [ ] Custom serialization options

#### v0.3.0
- [ ] WebSocket transport support
- [ ] Plugin system for custom message handlers
- [ ] Advanced configuration validation
- [ ] Performance profiling tools
- [ ] Extended MCP server support

#### v1.0.0
- [ ] Stable API with backward compatibility guarantees
- [ ] Production-ready performance optimizations
- [ ] Comprehensive benchmarking suite
- [ ] Advanced error recovery strategies
- [ ] Full feature parity with Python SDK

### Long-term Goals
- Integration with Rust async ecosystem standards
- Support for custom transport implementations
- Advanced debugging and diagnostic tools
- Performance optimization for high-throughput scenarios
- Extended platform support and compatibility

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for information on how to contribute to this project.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.