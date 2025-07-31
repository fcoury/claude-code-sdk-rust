# Requirements Document

## Introduction

This document outlines the requirements for migrating the Claude Code SDK from Python to Rust. The migration aims to provide a high-performance, memory-safe, and idiomatic Rust implementation that maintains full API compatibility and feature parity with the existing Python SDK. The Rust SDK will enable Rust developers to interact with Claude Code through both one-shot queries and interactive bidirectional conversations, while leveraging Rust's type safety and performance characteristics.

## Requirements

### Requirement 1: Core Type System

**User Story:** As a Rust developer, I want strongly-typed message and configuration structures, so that I can benefit from compile-time safety and clear API contracts.

#### Acceptance Criteria

1. WHEN defining message types THEN the system SHALL provide `UserMessage`, `AssistantMessage`, `SystemMessage`, and `ResultMessage` structs with serde serialization support
2. WHEN defining content blocks THEN the system SHALL provide `TextBlock`, `ToolUseBlock`, and `ToolResultBlock` structs with a `ContentBlock` enum wrapper
3. WHEN configuring options THEN the system SHALL provide a `ClaudeCodeOptions` struct with builder pattern implementation
4. WHEN handling MCP server configurations THEN the system SHALL support stdio, SSE, and HTTP server types through typed enums
5. WHEN working with permission modes THEN the system SHALL provide a `PermissionMode` enum with variants for default, acceptEdits, and bypassPermissions

### Requirement 2: Error Handling System

**User Story:** As a Rust developer, I want comprehensive error handling that follows Rust idioms, so that I can handle failures gracefully and understand what went wrong.

#### Acceptance Criteria

1. WHEN errors occur THEN the system SHALL provide a `SdkError` enum using thiserror for all failure modes
2. WHEN CLI is not found THEN the system SHALL raise `CliNotFound` error with helpful installation instructions
3. WHEN CLI process fails THEN the system SHALL raise `Process` error with exit code and stderr information
4. WHEN JSON parsing fails THEN the system SHALL raise `JsonDecode` error with context about the failed data
5. WHEN message parsing fails THEN the system SHALL raise `MessageParse` error with the problematic data included

### Requirement 3: Transport Layer

**User Story:** As a developer, I want reliable subprocess communication with the Claude CLI, so that I can send requests and receive responses without data corruption or loss.

#### Acceptance Criteria

1. WHEN discovering CLI THEN the system SHALL search standard installation paths and provide clear error messages if not found
2. WHEN spawning subprocess THEN the system SHALL use tokio::process::Command with proper stdin/stdout/stderr configuration
3. WHEN receiving messages THEN the system SHALL implement robust JSON buffering to handle split and concatenated messages
4. WHEN process terminates THEN the system SHALL capture stderr output and include it in error messages for non-zero exit codes
5. WHEN disconnecting THEN the system SHALL properly terminate child processes and clean up resources

### Requirement 4: One-Shot Query API

**User Story:** As a Rust developer, I want a simple query function for one-off interactions, so that I can quickly get responses from Claude without managing connection state.

#### Acceptance Criteria

1. WHEN calling query function THEN the system SHALL accept a prompt string and optional ClaudeCodeOptions
2. WHEN processing query THEN the system SHALL return an async Stream of Message results
3. WHEN query completes THEN the system SHALL automatically clean up transport resources
4. WHEN query fails THEN the system SHALL propagate errors through the Result type system
5. WHEN using string prompts THEN the system SHALL handle the conversion to the CLI's expected message format

### Requirement 5: Interactive Client API

**User Story:** As a Rust developer, I want a stateful client for bidirectional conversations, so that I can build interactive applications with follow-up messages and interrupts.

#### Acceptance Criteria

1. WHEN creating client THEN the system SHALL provide ClaudeSDKClient struct with connection management
2. WHEN connecting THEN the system SHALL support both initial prompt and empty stream for interactive use
3. WHEN sending messages THEN the system SHALL provide query method that accepts strings or async iterables
4. WHEN receiving messages THEN the system SHALL provide receive_messages method returning async Stream
5. WHEN interrupting THEN the system SHALL provide interrupt method that sends control signals to CLI
6. WHEN using async context THEN the system SHALL implement proper async Drop semantics for resource cleanup

### Requirement 6: Message Streaming and Parsing

**User Story:** As a developer, I want reliable message parsing from CLI output, so that I can receive structured data without worrying about JSON formatting issues.

#### Acceptance Criteria

1. WHEN parsing messages THEN the system SHALL handle line-by-line JSON parsing with proper buffering
2. WHEN encountering partial JSON THEN the system SHALL accumulate data until complete objects are formed
3. WHEN parsing different message types THEN the system SHALL correctly map to appropriate Rust structs
4. WHEN handling control responses THEN the system SHALL manage request/response correlation for interrupts
5. WHEN buffer exceeds limits THEN the system SHALL raise appropriate errors to prevent memory exhaustion

### Requirement 7: Async Stream Integration

**User Story:** As a Rust developer, I want native async Stream support, so that I can integrate with Rust's async ecosystem and use familiar patterns.

#### Acceptance Criteria

1. WHEN returning message streams THEN the system SHALL use tokio-stream's Stream trait
2. WHEN processing async iterables THEN the system SHALL accept any type implementing AsyncIterable
3. WHEN handling backpressure THEN the system SHALL properly manage flow control in streaming scenarios
4. WHEN cancelling streams THEN the system SHALL handle early termination gracefully
5. WHEN chaining operations THEN the system SHALL support standard Stream combinators

### Requirement 8: Configuration and Options

**User Story:** As a Rust developer, I want ergonomic configuration options, so that I can easily customize Claude's behavior for my specific use case.

#### Acceptance Criteria

1. WHEN building options THEN the system SHALL provide builder pattern with method chaining
2. WHEN setting system prompts THEN the system SHALL support both system_prompt and append_system_prompt options
3. WHEN configuring tools THEN the system SHALL support allowed_tools and disallowed_tools lists
4. WHEN setting MCP servers THEN the system SHALL support typed server configurations with proper validation
5. WHEN specifying working directory THEN the system SHALL accept Path types and handle path conversion

### Requirement 9: CLI Integration and Discovery

**User Story:** As a user, I want automatic CLI discovery and helpful error messages, so that I can quickly identify and resolve installation issues.

#### Acceptance Criteria

1. WHEN CLI is missing THEN the system SHALL provide installation instructions for Node.js and claude-code
2. WHEN CLI is in non-standard location THEN the system SHALL search common installation paths
3. WHEN Node.js is missing THEN the system SHALL detect this condition and provide specific guidance
4. WHEN working directory is invalid THEN the system SHALL provide clear error messages
5. WHEN CLI version is incompatible THEN the system SHALL detect and report version mismatches

### Requirement 10: Resource Management

**User Story:** As a Rust developer, I want reliable resource cleanup with both explicit and automatic options, so that I can ensure processes are properly terminated while having fallback protection.

#### Acceptance Criteria

1. WHEN calling disconnect explicitly THEN the system SHALL guarantee child process termination and resource cleanup
2. WHEN client goes out of scope THEN the system SHALL provide best-effort automatic cleanup via Drop trait
3. WHEN connection fails THEN the system SHALL clean up any partially created resources
4. WHEN using RAII patterns THEN the system SHALL implement Drop trait with documented limitations for async cleanup
5. WHEN managing timeouts THEN the system SHALL prevent indefinite blocking on process operations

### Requirement 11: API Ergonomics

**User Story:** As a Rust developer, I want ergonomic APIs that work with various string types and follow Rust conventions, so that I can integrate the SDK seamlessly into my applications.

#### Acceptance Criteria

1. WHEN calling query function THEN the system SHALL accept any type implementing AsRef<str> for the prompt parameter
2. WHEN creating empty streams THEN the system SHALL use tokio_stream::pending() for clean never-yielding streams
3. WHEN building configurations THEN the system SHALL provide fluent builder pattern with method chaining
4. WHEN handling errors THEN the system SHALL include original data and actionable guidance in error messages
5. WHEN working with paths THEN the system SHALL accept both String and PathBuf types for file system operations