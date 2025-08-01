# Contributing to Claude Code SDK for Rust

Thank you for your interest in contributing to the Claude Code SDK for Rust! This document provides guidelines and information for contributors.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Testing](#testing)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Architecture Overview](#architecture-overview)

## Getting Started

### Prerequisites

- **Rust**: Latest stable version (1.70+)
- **Node.js**: Version 18 or higher
- **Claude Code CLI**: `npm install -g @anthropic-ai/claude-code`
- **Git**: For version control

### Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/anthropics/claude-code-sdk-rust
   cd claude-code-sdk-rust
   ```

2. **Install dependencies**:
   ```bash
   cargo build
   ```

3. **Run tests**:
   ```bash
   cargo test
   ```

4. **Run examples**:
   ```bash
   cargo run --example quick_start
   ```

## Code Style

### Formatting

We use `rustfmt` for consistent code formatting:

```bash
# Format all code
cargo fmt

# Check formatting without making changes
cargo fmt -- --check
```

### Linting

We use `clippy` for additional linting:

```bash
# Run clippy
cargo clippy

# Run clippy with all features
cargo clippy --all-features

# Treat warnings as errors (CI configuration)
cargo clippy -- -D warnings
```

### Code Guidelines

1. **Error Handling**: Always use `Result<T>` for fallible operations
2. **Documentation**: All public APIs must have comprehensive doc comments
3. **Testing**: New features require corresponding tests
4. **Async**: Use `async`/`await` consistently, avoid blocking operations
5. **Memory Safety**: Leverage Rust's ownership system, avoid `unsafe` unless absolutely necessary

### Naming Conventions

- **Types**: `PascalCase` (e.g., `ClaudeSDKClient`)
- **Functions**: `snake_case` (e.g., `receive_messages`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BUFFER_SIZE`)
- **Modules**: `snake_case` (e.g., `message_parser`)

## Testing

### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test component interactions
3. **Example Tests**: Ensure examples compile and run

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_test

# Specific test
cargo test test_message_parsing

# With output
cargo test -- --nocapture
```

### Test Guidelines

1. **Coverage**: Aim for high test coverage, especially for error paths
2. **Isolation**: Tests should be independent and not rely on external state
3. **Naming**: Use descriptive test names that explain what is being tested
4. **Assertions**: Use appropriate assertion macros (`assert_eq!`, `assert!`, etc.)

### Mock Testing

For tests that require the Claude CLI, we use a mock CLI script:

```bash
# Run integration tests with mock CLI
cargo test --test integration_test
```

## Documentation

### Doc Comments

All public APIs must have comprehensive documentation:

```rust
/// Brief description of the function.
///
/// Longer description explaining the purpose, behavior, and usage.
///
/// # Arguments
///
/// * `param1` - Description of the first parameter
/// * `param2` - Description of the second parameter
///
/// # Returns
///
/// Description of what the function returns.
///
/// # Errors
///
/// Description of possible error conditions.
///
/// # Examples
///
/// ```rust
/// use claude_code_sdk::example_function;
///
/// let result = example_function("input").await?;
/// assert_eq!(result, "expected");
/// ```
pub async fn example_function(input: &str) -> Result<String> {
    // Implementation
}
```

### Documentation Generation

```bash
# Generate documentation
cargo doc

# Generate and open documentation
cargo doc --open

# Generate documentation with private items
cargo doc --document-private-items
```

## Pull Request Process

### Before Submitting

1. **Run all checks**:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   cargo doc
   ```

2. **Update documentation** if needed
3. **Add tests** for new functionality
4. **Update examples** if the API changes

### PR Guidelines

1. **Title**: Use a clear, descriptive title
2. **Description**: Explain what changes were made and why
3. **Breaking Changes**: Clearly mark any breaking changes
4. **Testing**: Describe how the changes were tested
5. **Documentation**: Note any documentation updates

### Review Process

1. All PRs require at least one review
2. CI checks must pass
3. Documentation must be updated for API changes
4. Breaking changes require special consideration

## Issue Reporting

### Bug Reports

When reporting bugs, please include:

1. **Environment**: OS, Rust version, CLI version
2. **Steps to reproduce**: Minimal example that demonstrates the issue
3. **Expected behavior**: What you expected to happen
4. **Actual behavior**: What actually happened
5. **Error messages**: Full error output if applicable

### Feature Requests

For feature requests, please include:

1. **Use case**: Why is this feature needed?
2. **Proposed API**: How should the feature work?
3. **Alternatives**: What alternatives have you considered?
4. **Implementation**: Any thoughts on implementation approach?

## Architecture Overview

### Module Structure

```
src/
├── lib.rs           # Public API and re-exports
├── client.rs        # Client implementations
├── errors.rs        # Error types and handling
├── message_parser.rs # JSON message parsing
├── transport.rs     # CLI communication layer
└── types.rs         # Message and configuration types
```

### Key Components

1. **Transport Layer**: Manages subprocess communication with CLI
2. **Message Parser**: Converts JSON to typed Rust structures
3. **Client Layer**: Provides high-level APIs for users
4. **Error System**: Comprehensive error handling with recovery guidance
5. **Type System**: Strongly-typed message and configuration structures

### Design Principles

1. **Type Safety**: Leverage Rust's type system for compile-time guarantees
2. **Memory Safety**: Automatic resource management with explicit cleanup options
3. **Performance**: Minimal overhead with efficient parsing and buffering
4. **Ergonomics**: Easy-to-use APIs that follow Rust conventions
5. **Error Handling**: Comprehensive errors with actionable guidance

### Adding New Features

When adding new features:

1. **Start with types**: Define the data structures first
2. **Add parsing**: Implement JSON parsing for new message types
3. **Update transport**: Add any new CLI communication needs
4. **Implement client APIs**: Add high-level user-facing functions
5. **Add error handling**: Define new error types if needed
6. **Write tests**: Comprehensive test coverage
7. **Update documentation**: API docs and examples

## Release Process

### Version Numbering

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Update documentation
5. Create release PR
6. Tag release after merge
7. Publish to crates.io

## Getting Help

- **Documentation**: https://docs.rs/claude-code-sdk
- **Issues**: https://github.com/anthropics/claude-code-sdk-rust/issues
- **Discussions**: https://github.com/anthropics/claude-code-sdk-rust/discussions
- **Email**: support@anthropic.com

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Please be respectful and inclusive in all interactions.

## License

By contributing to this project, you agree that your contributions will be licensed under the MIT License.