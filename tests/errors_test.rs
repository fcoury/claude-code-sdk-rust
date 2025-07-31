use claude_code_sdk::{SdkError, errors::SdkErrorExt};
use rstest::*;
use serde_json::json;
use std::io;

// ============================================================================
// Error Creation and Basic Properties Tests
// ============================================================================

#[test]
fn test_error_creation_methods() {
    let error = SdkError::message_parse("Invalid format", json!({"invalid": true}));
    assert!(matches!(error, SdkError::MessageParse { .. }));
    
    let error = SdkError::process(Some(1), "Command failed");
    assert!(matches!(error, SdkError::Process { .. }));
    
    let error = SdkError::transport("Connection lost");
    assert!(matches!(error, SdkError::Transport(_)));
    
    let error = SdkError::invalid_working_directory("/nonexistent");
    assert!(matches!(error, SdkError::InvalidWorkingDirectory { .. }));
    
    let error = SdkError::buffer_size_exceeded(1024);
    assert!(matches!(error, SdkError::BufferSizeExceeded { .. }));
    
    let error = SdkError::session("Session expired");
    assert!(matches!(error, SdkError::Session(_)));
    
    let error = SdkError::control_timeout(5000);
    assert!(matches!(error, SdkError::ControlTimeout { .. }));
    
    let error = SdkError::incompatible_cli_version("1.0.0", "0.9.0");
    assert!(matches!(error, SdkError::IncompatibleCliVersion { .. }));
    
    let error = SdkError::configuration("Invalid config");
    assert!(matches!(error, SdkError::Configuration { .. }));
    
    let error = SdkError::stream("Stream closed");
    assert!(matches!(error, SdkError::Stream { .. }));
    
    let error = SdkError::interrupt("Interrupt failed");
    assert!(matches!(error, SdkError::Interrupt { .. }));
}

#[test]
fn test_error_creation_with_context() {
    let error = SdkError::configuration_with_source(
        "Config parse error",
        Box::new(io::Error::new(io::ErrorKind::NotFound, "file not found"))
    );
    assert!(matches!(error, SdkError::Configuration { .. }));
    if let SdkError::Configuration { source, .. } = error {
        assert!(source.is_some());
    }
    
    let error = SdkError::stream_with_context("Stream error", "During message processing");
    if let SdkError::Stream { context, .. } = error {
        assert_eq!(context, Some("During message processing".to_string()));
    }
    
    let error = SdkError::interrupt_with_request_id("Failed to interrupt", "req_123");
    if let SdkError::Interrupt { request_id, .. } = error {
        assert_eq!(request_id, Some("req_123".to_string()));
    }
}

// ============================================================================
// Error Categories and Recoverability Tests
// ============================================================================

#[rstest]
#[case(SdkError::NodeJsNotFound, "cli", false)]
#[case(SdkError::transport("test"), "transport", true)]
#[case(SdkError::session("test"), "session", true)]
#[case(SdkError::buffer_size_exceeded(1024), "memory", true)]
#[case(SdkError::message_parse("test", json!({})), "parsing", false)]
#[case(SdkError::invalid_working_directory("/invalid"), "configuration", true)]
#[case(SdkError::control_timeout(5000), "control", true)]
#[case(SdkError::incompatible_cli_version("1.0", "0.9"), "cli", false)]
fn test_error_categories_and_recoverability(
    #[case] error: SdkError,
    #[case] expected_category: &str,
    #[case] expected_recoverable: bool,
) {
    assert_eq!(error.category(), expected_category);
    assert_eq!(error.is_recoverable(), expected_recoverable);
}

// ============================================================================
// Error Display and Formatting Tests
// ============================================================================

#[test]
fn test_error_display_messages() {
    let error = SdkError::NodeJsNotFound;
    let display = format!("{}", error);
    assert!(display.contains("Node.js runtime not found"));
    assert!(display.contains("https://nodejs.org/"));
    
    let error = SdkError::incompatible_cli_version("1.0.0", "0.9.0");
    let display = format!("{}", error);
    assert!(display.contains("Incompatible CLI version"));
    assert!(display.contains("Expected version 1.0.0"));
    assert!(display.contains("found 0.9.0"));
    assert!(display.contains("npm install -g"));
    
    let error = SdkError::process(Some(1), "stderr output");
    let display = format!("{}", error);
    assert!(display.contains("CLI process failed"));
    assert!(display.contains("exit code Some(1)"));
    assert!(display.contains("stderr output"));
    
    let error = SdkError::buffer_size_exceeded(1024);
    let display = format!("{}", error);
    assert!(display.contains("Buffer size exceeded"));
    assert!(display.contains("1024 bytes"));
    
    let error = SdkError::control_timeout(5000);
    let display = format!("{}", error);
    assert!(display.contains("Control request timed out"));
    assert!(display.contains("5000ms"));
}

#[test]
fn test_cli_not_found_error_message() {
    let error = SdkError::NodeJsNotFound;
    let message = format!("{}", error);
    
    // Should contain installation instructions
    assert!(message.contains("Node.js"));
    assert!(message.contains("https://nodejs.org/"));
    
    // Should be helpful and actionable
    assert!(message.contains("install"));
}

#[test]
fn test_message_parse_error_details() {
    let test_data = json!({
        "type": "unknown",
        "data": "test data"
    });
    let error = SdkError::message_parse("Unknown message type", test_data.clone());
    
    if let SdkError::MessageParse { message, data } = error {
        assert_eq!(message, "Unknown message type");
        assert_eq!(data, test_data);
    } else {
        panic!("Expected MessageParse error");
    }
}

// ============================================================================
// Error Debug Data Tests
// ============================================================================

#[test]
fn test_debug_data_structure() {
    let error = SdkError::message_parse("Invalid format", json!({"test": "data"}));
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["category"], "parsing");
    assert_eq!(debug_data["type"], "message_parse");
    assert_eq!(debug_data["message"], "Invalid format");
    assert_eq!(debug_data["raw_data"], json!({"test": "data"}));
    assert_eq!(debug_data["data_type"], "object");
}

#[test]
fn test_debug_data_for_different_json_types() {
    let test_cases = vec![
        (json!(null), "null"),
        (json!(true), "boolean"),
        (json!(42), "number"),
        (json!("string"), "string"),
        (json!([1, 2, 3]), "array"),
        (json!({"key": "value"}), "object"),
    ];
    
    for (data, expected_type) in test_cases {
        let error = SdkError::message_parse("test", data.clone());
        let debug_data = error.debug_data();
        assert_eq!(debug_data["data_type"], expected_type);
        assert_eq!(debug_data["raw_data"], data);
    }
}

#[test]
fn test_debug_data_process_error() {
    let error = SdkError::process(Some(1), "stderr output");
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["category"], "process");
    assert_eq!(debug_data["type"], "process_failure");
    assert_eq!(debug_data["exit_code"], 1);
    assert_eq!(debug_data["stderr"], "stderr output");
}

#[test]
fn test_debug_data_json_decode_error() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let error = SdkError::JsonDecode(json_error);
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["category"], "parsing");
    assert_eq!(debug_data["type"], "json_decode");
    assert!(debug_data["line"].is_number());
    assert!(debug_data["column"].is_number());
}

#[test]
fn test_debug_data_io_error() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let error = SdkError::CliConnection(io_error);
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["category"], "process");
    assert_eq!(debug_data["type"], "cli_connection");
    assert_eq!(debug_data["io_kind"], "NotFound");
    assert!(debug_data["io_error"].as_str().unwrap().contains("file not found"));
}

#[test]
fn test_debug_data_configuration_error_with_source() {
    let source_error = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
    let error = SdkError::configuration_with_source("Config error", Box::new(source_error));
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["category"], "configuration");
    assert_eq!(debug_data["type"], "configuration");
    assert_eq!(debug_data["message"], "Config error");
    assert!(debug_data["source"].as_str().unwrap().contains("access denied"));
}

// ============================================================================
// Error Conversion Traits Tests
// ============================================================================

#[test]
fn test_error_conversion_from_io_error() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "test");
    let sdk_error: SdkError = io_error.into();
    assert!(matches!(sdk_error, SdkError::CliConnection(_)));
}

#[test]
fn test_error_conversion_from_json_error() {
    let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
    let sdk_error: SdkError = json_error.into();
    assert!(matches!(sdk_error, SdkError::JsonDecode(_)));
}

#[test]
fn test_error_conversion_from_parse_int_error() {
    let parse_error: std::num::ParseIntError = "not_a_number".parse::<i32>().unwrap_err();
    let sdk_error: SdkError = parse_error.into();
    assert!(matches!(sdk_error, SdkError::Configuration { .. }));
    assert_eq!(sdk_error.category(), "configuration");
}

#[test]
fn test_error_conversion_from_parse_float_error() {
    let parse_error: std::num::ParseFloatError = "not_a_float".parse::<f64>().unwrap_err();
    let sdk_error: SdkError = parse_error.into();
    assert!(matches!(sdk_error, SdkError::Configuration { .. }));
}

#[test]
fn test_error_conversion_from_env_var_error() {
    let env_error = std::env::var("NONEXISTENT_VAR_12345").unwrap_err();
    let sdk_error: SdkError = env_error.into();
    assert!(matches!(sdk_error, SdkError::Configuration { .. }));
}

#[test]
fn test_error_conversion_from_tokio_timeout() {
    // Create a timeout error by using tokio::time::timeout with a very short duration
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_nanos(1),
            tokio::time::sleep(std::time::Duration::from_secs(1))
        ).await;
        
        if let Err(elapsed) = timeout_result {
            let sdk_error: SdkError = elapsed.into();
            assert!(matches!(sdk_error, SdkError::ControlTimeout { .. }));
        }
    });
}

// ============================================================================
// Error Extension Trait Tests
// ============================================================================

#[test]
fn test_error_extension_with_context() {
    let result: Result<i32, io::Error> = Err(io::Error::new(io::ErrorKind::NotFound, "test"));
    let sdk_result = result.with_context(|| "Testing context".to_string());
    
    assert!(sdk_result.is_err());
    // The context should be applied to transport/session errors, but io::Error converts to CliConnection
    assert!(matches!(sdk_result.unwrap_err(), SdkError::CliConnection(_)));
}

#[test]
fn test_error_extension_with_static_context() {
    let result: Result<i32, io::Error> = Err(io::Error::new(io::ErrorKind::NotFound, "test"));
    let sdk_result = result.with_static_context("Static context");
    
    assert!(sdk_result.is_err());
    assert!(matches!(sdk_result.unwrap_err(), SdkError::CliConnection(_)));
}

#[test]
fn test_error_extension_with_transport_error() {
    let result: Result<i32, SdkError> = Err(SdkError::transport("original error"));
    let sdk_result = result.with_context(|| "Additional context".to_string());
    
    assert!(sdk_result.is_err());
    if let Err(SdkError::Transport(msg)) = sdk_result {
        assert!(msg.contains("Additional context"));
        assert!(msg.contains("original error"));
    } else {
        panic!("Expected Transport error with context");
    }
}

#[test]
fn test_error_extension_with_session_error() {
    let result: Result<i32, SdkError> = Err(SdkError::session("session error"));
    let sdk_result = result.with_static_context("Session context");
    
    assert!(sdk_result.is_err());
    if let Err(SdkError::Session(msg)) = sdk_result {
        assert!(msg.contains("Session context"));
        assert!(msg.contains("session error"));
    } else {
        panic!("Expected Session error with context");
    }
}

// ============================================================================
// Error Serialization and Debugging Tests
// ============================================================================

#[test]
fn test_error_debug_formatting() {
    let error = SdkError::message_parse("test error", json!({"data": "value"}));
    let debug_str = format!("{:?}", error);
    
    assert!(debug_str.contains("MessageParse"));
    assert!(debug_str.contains("test error"));
}

#[test]
fn test_error_source_chain() {
    let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let config_error = SdkError::configuration_with_source("Config failed", Box::new(io_error));
    
    // Test that the source chain is preserved
    if let SdkError::Configuration { source, .. } = &config_error {
        assert!(source.is_some());
        let source_str = source.as_ref().unwrap().to_string();
        assert!(source_str.contains("permission denied"));
    }
}

#[test]
fn test_comprehensive_error_coverage() {
    // Test that all error variants can be created and have proper properties
    let errors = vec![
        SdkError::NodeJsNotFound,
        SdkError::transport("transport error"),
        SdkError::session("session error"),
        SdkError::buffer_size_exceeded(1024),
        SdkError::message_parse("parse error", json!({})),
        SdkError::invalid_working_directory("/invalid"),
        SdkError::control_timeout(5000),
        SdkError::incompatible_cli_version("1.0", "0.9"),
        SdkError::configuration("config error"),
        SdkError::stream("stream error"),
        SdkError::interrupt("interrupt error"),
        SdkError::process(Some(1), "process error"),
    ];
    
    for error in errors {
        // Each error should have a category
        assert!(!error.category().is_empty());
        
        // Each error should have a display message
        let display = format!("{}", error);
        assert!(!display.is_empty());
        
        // Each error should have debug data
        let debug_data = error.debug_data();
        assert!(debug_data.is_object());
        assert!(debug_data["category"].is_string());
        assert!(debug_data["type"].is_string());
        
        // Each error should have a recoverable status
        let _recoverable = error.is_recoverable();
    }
}

// ============================================================================
// Edge Cases and Complex Scenarios
// ============================================================================

#[test]
fn test_error_with_large_data() {
    let large_data = json!({
        "large_field": "x".repeat(10000),
        "nested": {
            "array": (0..1000).collect::<Vec<i32>>()
        }
    });
    
    let error = SdkError::message_parse("Large data error", large_data.clone());
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["raw_data"], large_data);
    assert_eq!(debug_data["data_type"], "object");
}

#[test]
fn test_error_with_unicode_content() {
    let unicode_data = json!({
        "message": "Error with unicode: 世界 🌍 мир عالم",
        "path": "/tmp/файл.txt"
    });
    
    let error = SdkError::message_parse("Unicode error", unicode_data.clone());
    let debug_data = error.debug_data();
    
    assert_eq!(debug_data["raw_data"], unicode_data);
    
    let display = format!("{}", error);
    assert!(display.contains("Unicode error"));
}

#[test]
fn test_nested_error_sources() {
    // Create a chain of errors
    let root_error = io::Error::new(io::ErrorKind::NotFound, "root cause");
    let config_error = SdkError::configuration_with_source("Config issue", Box::new(root_error));
    
    // Verify the error chain
    if let SdkError::Configuration { source, message } = config_error {
        assert_eq!(message, "Config issue");
        assert!(source.is_some());
        
        let source_error = source.unwrap();
        assert!(source_error.to_string().contains("root cause"));
    }
}

#[test]
fn test_error_equality_and_comparison() {
    let error1 = SdkError::transport("same message");
    let error2 = SdkError::transport("same message");
    let error3 = SdkError::transport("different message");
    
    // Note: SdkError doesn't implement PartialEq, so we test structural equality
    assert_eq!(error1.category(), error2.category());
    assert_ne!(format!("{}", error1), format!("{}", error3));
}