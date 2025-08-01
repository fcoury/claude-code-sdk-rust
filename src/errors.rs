//! Error types and handling for the Claude Code SDK.
//!
//! This module defines all error types that can occur when using the SDK,
//! providing structured error information and helpful error messages.
//!
//! # Error Categories
//!
//! - **CLI Errors**: Issues with finding or running the Claude Code CLI
//! - **Process Errors**: Problems with subprocess management
//! - **Parsing Errors**: JSON and message parsing failures
//! - **Transport Errors**: Communication layer issues
//! - **Configuration Errors**: Invalid options or settings
//! - **Session Errors**: Interactive session management problems

use thiserror::Error;

/// Result type alias for SDK operations.
pub type Result<T> = std::result::Result<T, SdkError>;

/// Comprehensive error type for all SDK operations.
#[derive(Error, Debug)]
pub enum SdkError {
    /// Claude Code CLI was not found in the system PATH.
    #[error("Claude Code CLI not found. Please ensure it is installed and in your PATH.\n\nInstall with: npm install -g @anthropic-ai/claude-code\n\nFor more information, visit: https://github.com/anthropics/claude-code")]
    CliNotFound(#[from] which::Error),

    /// Failed to connect to or start the Claude Code CLI process.
    #[error("Failed to connect to or start the Claude Code CLI process: {0}")]
    CliConnection(#[from] std::io::Error),

    /// The CLI process failed with a non-zero exit code.
    #[error("The CLI process failed with exit code {exit_code:?}: {stderr}")]
    Process {
        /// The exit code of the failed process
        exit_code: Option<i32>,
        /// Standard error output from the process
        stderr: String,
    },

    /// Failed to decode JSON from CLI output.
    #[error("Failed to decode JSON from CLI output: {0}")]
    JsonDecode(#[from] serde_json::Error),

    /// Failed to parse a message from the received data.
    #[error("Failed to parse message from data: {message}")]
    MessageParse {
        /// Description of the parsing error
        message: String,
        /// The raw data that failed to parse
        data: serde_json::Value,
    },

    /// Transport layer error.
    #[error("Transport error: {0}")]
    Transport(String),

    /// Invalid working directory specified.
    #[error("Invalid working directory: {path}")]
    InvalidWorkingDirectory {
        /// The invalid path that was specified
        path: String,
    },

    /// Buffer size exceeded the maximum allowed limit.
    #[error("Buffer size exceeded maximum limit of {limit} bytes")]
    BufferSizeExceeded {
        /// The maximum buffer size limit
        limit: usize,
    },

    /// Node.js runtime not found.
    #[error("Node.js runtime not found. Please install Node.js to use the Claude Code CLI.\n\nDownload from: https://nodejs.org/")]
    NodeJsNotFound,

    /// CLI version is incompatible with this SDK.
    #[error("Incompatible CLI version. Expected version {expected}, found {found}.\n\nUpdate with: npm install -g @anthropic-ai/claude-code@latest")]
    IncompatibleCliVersion {
        /// The expected CLI version
        expected: String,
        /// The found CLI version
        found: String,
    },

    /// Session management error.
    #[error("Session error: {0}")]
    Session(String),

    /// Control request timeout.
    #[error("Control request timed out after {timeout_ms}ms")]
    ControlTimeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Configuration validation error.
    #[error("Configuration error: {message}")]
    Configuration {
        /// Description of the configuration error
        message: String,
        /// Optional context data for debugging
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Stream processing error.
    #[error("Stream error: {message}")]
    Stream {
        /// Description of the stream error
        message: String,
        /// Optional context for debugging
        context: Option<String>,
    },

    /// Interrupt handling error.
    #[error("Interrupt error: {message}")]
    Interrupt {
        /// Description of the interrupt error
        message: String,
        /// Request ID that failed to interrupt
        request_id: Option<String>,
    },
}

impl SdkError {
    /// Create a new MessageParse error.
    pub fn message_parse<S: Into<String>>(message: S, data: serde_json::Value) -> Self {
        Self::MessageParse {
            message: message.into(),
            data,
        }
    }

    /// Create a new Process error.
    pub fn process<S: Into<String>>(exit_code: Option<i32>, stderr: S) -> Self {
        Self::Process {
            exit_code,
            stderr: stderr.into(),
        }
    }

    /// Create a new Transport error.
    pub fn transport<S: Into<String>>(message: S) -> Self {
        Self::Transport(message.into())
    }

    /// Create a new InvalidWorkingDirectory error.
    pub fn invalid_working_directory<S: Into<String>>(path: S) -> Self {
        Self::InvalidWorkingDirectory { path: path.into() }
    }

    /// Create a new BufferSizeExceeded error.
    pub fn buffer_size_exceeded(limit: usize) -> Self {
        Self::BufferSizeExceeded { limit }
    }

    /// Create a new Session error.
    pub fn session<S: Into<String>>(message: S) -> Self {
        Self::Session(message.into())
    }

    /// Create a new ControlTimeout error.
    pub fn control_timeout(timeout_ms: u64) -> Self {
        Self::ControlTimeout { timeout_ms }
    }

    /// Create a new IncompatibleCliVersion error.
    pub fn incompatible_cli_version<S: Into<String>>(expected: S, found: S) -> Self {
        Self::IncompatibleCliVersion {
            expected: expected.into(),
            found: found.into(),
        }
    }

    /// Create a new Configuration error.
    pub fn configuration<S: Into<String>>(message: S) -> Self {
        Self::Configuration {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new Configuration error with source.
    pub fn configuration_with_source<S: Into<String>>(
        message: S,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::Configuration {
            message: message.into(),
            source: Some(source),
        }
    }

    /// Create a new Stream error.
    pub fn stream<S: Into<String>>(message: S) -> Self {
        Self::Stream {
            message: message.into(),
            context: None,
        }
    }

    /// Create a new Stream error with context.
    pub fn stream_with_context<S: Into<String>, C: Into<String>>(message: S, context: C) -> Self {
        Self::Stream {
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create a new Interrupt error.
    pub fn interrupt<S: Into<String>>(message: S) -> Self {
        Self::Interrupt {
            message: message.into(),
            request_id: None,
        }
    }

    /// Create a new Interrupt error with request ID.
    pub fn interrupt_with_request_id<S: Into<String>, R: Into<String>>(
        message: S,
        request_id: R,
    ) -> Self {
        Self::Interrupt {
            message: message.into(),
            request_id: Some(request_id.into()),
        }
    }

    /// Check if this error is recoverable.
    ///
    /// Returns `true` for errors that might be resolved by retrying
    /// or changing configuration, `false` for permanent failures.
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Permanent failures
            Self::CliNotFound(_) | Self::NodeJsNotFound | Self::IncompatibleCliVersion { .. } => {
                false
            }
            // Configuration issues that can be fixed
            Self::InvalidWorkingDirectory { .. } | Self::Configuration { .. } => true,
            // Process and transport errors might be temporary
            Self::Process { .. }
            | Self::Transport(_)
            | Self::CliConnection(_)
            | Self::ControlTimeout { .. } => true,
            // Parsing errors are usually permanent for the specific data
            Self::JsonDecode(_) | Self::MessageParse { .. } => false,
            // Buffer and stream errors might be recoverable
            Self::BufferSizeExceeded { .. } | Self::Stream { .. } => true,
            // Session and interrupt errors might be recoverable
            Self::Session(_) | Self::Interrupt { .. } => true,
        }
    }

    /// Get the error category as a string.
    pub fn category(&self) -> &'static str {
        match self {
            Self::CliNotFound(_) | Self::NodeJsNotFound | Self::IncompatibleCliVersion { .. } => {
                "cli"
            }
            Self::CliConnection(_) | Self::Process { .. } => "process",
            Self::JsonDecode(_) | Self::MessageParse { .. } => "parsing",
            Self::Transport(_) | Self::Stream { .. } => "transport",
            Self::InvalidWorkingDirectory { .. } | Self::Configuration { .. } => "configuration",
            Self::BufferSizeExceeded { .. } => "memory",
            Self::Session(_) => "session",
            Self::ControlTimeout { .. } | Self::Interrupt { .. } => "control",
        }
    }

    /// Get structured error data for debugging.
    ///
    /// Returns a JSON object containing error details that can be
    /// used for logging, debugging, or error reporting.
    pub fn debug_data(&self) -> serde_json::Value {
        use serde_json::json;

        match self {
            Self::CliNotFound(e) => json!({
                "category": "cli",
                "type": "cli_not_found",
                "message": self.to_string(),
                "source_error": e.to_string(),
            }),
            Self::CliConnection(e) => json!({
                "category": "process",
                "type": "cli_connection",
                "message": self.to_string(),
                "io_error": e.to_string(),
                "io_kind": format!("{:?}", e.kind()),
            }),
            Self::Process { exit_code, stderr } => json!({
                "category": "process",
                "type": "process_failure",
                "message": self.to_string(),
                "exit_code": exit_code,
                "stderr": stderr,
            }),
            Self::JsonDecode(e) => json!({
                "category": "parsing",
                "type": "json_decode",
                "message": self.to_string(),
                "json_error": e.to_string(),
                "line": e.line(),
                "column": e.column(),
            }),
            Self::MessageParse { message, data } => json!({
                "category": "parsing",
                "type": "message_parse",
                "message": message,
                "raw_data": data,
                "data_type": match data {
                    serde_json::Value::Null => "null",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                },
            }),
            Self::Transport(msg) => json!({
                "category": "transport",
                "type": "transport",
                "message": msg,
            }),
            Self::InvalidWorkingDirectory { path } => json!({
                "category": "configuration",
                "type": "invalid_working_directory",
                "message": self.to_string(),
                "path": path,
            }),
            Self::BufferSizeExceeded { limit } => json!({
                "category": "memory",
                "type": "buffer_size_exceeded",
                "message": self.to_string(),
                "limit": limit,
            }),
            Self::NodeJsNotFound => json!({
                "category": "cli",
                "type": "nodejs_not_found",
                "message": self.to_string(),
            }),
            Self::IncompatibleCliVersion { expected, found } => json!({
                "category": "cli",
                "type": "incompatible_cli_version",
                "message": self.to_string(),
                "expected": expected,
                "found": found,
            }),
            Self::Session(msg) => json!({
                "category": "session",
                "type": "session",
                "message": msg,
            }),
            Self::ControlTimeout { timeout_ms } => json!({
                "category": "control",
                "type": "control_timeout",
                "message": self.to_string(),
                "timeout_ms": timeout_ms,
            }),
            Self::Configuration { message, source } => json!({
                "category": "configuration",
                "type": "configuration",
                "message": message,
                "source": source.as_ref().map(|e| e.to_string()),
            }),
            Self::Stream { message, context } => json!({
                "category": "transport",
                "type": "stream",
                "message": message,
                "context": context,
            }),
            Self::Interrupt {
                message,
                request_id,
            } => json!({
                "category": "control",
                "type": "interrupt",
                "message": message,
                "request_id": request_id,
            }),
        }
    }
}

// Additional error conversion traits for common error types

impl From<std::path::StripPrefixError> for SdkError {
    fn from(e: std::path::StripPrefixError) -> Self {
        Self::configuration_with_source("Path prefix error", Box::new(e))
    }
}

impl From<std::env::VarError> for SdkError {
    fn from(e: std::env::VarError) -> Self {
        Self::configuration_with_source("Environment variable error", Box::new(e))
    }
}

impl From<std::num::ParseIntError> for SdkError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::configuration_with_source("Integer parsing error", Box::new(e))
    }
}

impl From<std::num::ParseFloatError> for SdkError {
    fn from(e: std::num::ParseFloatError) -> Self {
        Self::configuration_with_source("Float parsing error", Box::new(e))
    }
}

impl From<tokio::time::error::Elapsed> for SdkError {
    fn from(_e: tokio::time::error::Elapsed) -> Self {
        Self::control_timeout(5000) // Default timeout value
    }
}

/// Extension trait for converting Results to SdkError with context.
pub trait SdkErrorExt<T> {
    /// Add context to an error result.
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Add static context to an error result.
    fn with_static_context(self, context: &'static str) -> Result<T>;
}

impl<T, E> SdkErrorExt<T> for std::result::Result<T, E>
where
    E: Into<SdkError>,
{
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let sdk_error = e.into();
            match sdk_error {
                SdkError::Transport(msg) => SdkError::transport(format!("{}: {}", f(), msg)),
                SdkError::Session(msg) => SdkError::session(format!("{}: {}", f(), msg)),
                other => other,
            }
        })
    }

    fn with_static_context(self, context: &'static str) -> Result<T> {
        self.with_context(|| context.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_error_creation() {
        let error = SdkError::message_parse("Invalid format", json!({"invalid": true}));
        assert!(matches!(error, SdkError::MessageParse { .. }));

        let error = SdkError::process(Some(1), "Command failed");
        assert!(matches!(error, SdkError::Process { .. }));

        let error = SdkError::transport("Connection lost");
        assert!(matches!(error, SdkError::Transport(_)));
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(SdkError::NodeJsNotFound.category(), "cli");
        assert_eq!(SdkError::transport("test").category(), "transport");
        assert_eq!(SdkError::session("test").category(), "session");
        assert_eq!(SdkError::buffer_size_exceeded(1024).category(), "memory");
    }

    #[test]
    fn test_error_recoverability() {
        assert!(!SdkError::NodeJsNotFound.is_recoverable());
        assert!(SdkError::transport("test").is_recoverable());
        assert!(SdkError::invalid_working_directory("/invalid").is_recoverable());
        assert!(!SdkError::message_parse("test", json!({})).is_recoverable());
    }

    #[test]
    fn test_debug_data() {
        let error = SdkError::message_parse("Invalid format", json!({"test": "data"}));
        let debug_data = error.debug_data();

        assert_eq!(debug_data["category"], "parsing");
        assert_eq!(debug_data["type"], "message_parse");
        assert_eq!(debug_data["raw_data"], json!({"test": "data"}));
        assert_eq!(debug_data["data_type"], "object");
    }

    #[test]
    fn test_error_conversion_traits() {
        let parse_error: std::num::ParseIntError = "not_a_number".parse::<i32>().unwrap_err();
        let sdk_error: SdkError = parse_error.into();
        assert!(matches!(sdk_error, SdkError::Configuration { .. }));
        assert_eq!(sdk_error.category(), "configuration");
    }

    #[test]
    fn test_error_extension_trait() {
        let result: std::result::Result<i32, std::io::Error> =
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));

        let sdk_result = result.with_static_context("Testing context");
        assert!(sdk_result.is_err());
        assert!(matches!(
            sdk_result.unwrap_err(),
            SdkError::CliConnection(_)
        ));
    }

    #[test]
    fn test_error_display() {
        let error = SdkError::incompatible_cli_version("1.0.0", "0.9.0");
        let display = format!("{error}");
        assert!(display.contains("Incompatible CLI version"));
        assert!(display.contains("1.0.0"));
        assert!(display.contains("0.9.0"));
    }

    #[test]
    fn test_structured_error_data() {
        let error = SdkError::process(Some(1), "stderr output");
        let debug_data = error.debug_data();

        assert_eq!(debug_data["exit_code"], 1);
        assert_eq!(debug_data["stderr"], "stderr output");
        assert_eq!(debug_data["category"], "process");
    }
}
