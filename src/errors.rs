//! Error types and handling for the Claude Code SDK.
//!
//! This module defines all error types that can occur when using the SDK,
//! providing structured error information and helpful error messages.

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
}