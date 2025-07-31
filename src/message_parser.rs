//! Message parsing utilities for converting JSON to typed messages.
//!
//! This module handles the conversion of raw JSON data from the CLI
//! into strongly-typed message structures.

use crate::errors::{Result, SdkError};
use crate::types::*;
use serde_json;
use std::collections::HashMap;

/// Parse a JSON value into a typed Message.
pub fn parse_message(data: serde_json::Value) -> Result<Message> {
    let obj = data.as_object().ok_or_else(|| {
        SdkError::message_parse("Expected JSON object", data.clone())
    })?;

    let message_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing or invalid 'type' field", data.clone()))?;

    match message_type {
        "user" => parse_user_message(obj, &data),
        "assistant" => parse_assistant_message(obj, &data),
        "system" => parse_system_message(obj, &data),
        "result" => parse_result_message(obj, &data),
        _ => Err(SdkError::message_parse(
            format!("Unknown message type: {}", message_type),
            data,
        )),
    }
}

/// Parse a user message from JSON.
fn parse_user_message(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<Message> {
    let content = obj
        .get("content")
        .ok_or_else(|| SdkError::message_parse("Missing 'content' field", data.clone()))?;

    let message_content = parse_message_content(content, data)?;

    Ok(Message::User(UserMessage {
        content: message_content,
    }))
}

/// Parse an assistant message from JSON.
fn parse_assistant_message(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<Message> {
    let content = obj
        .get("content")
        .ok_or_else(|| SdkError::message_parse("Missing 'content' field", data.clone()))?;

    let content_blocks = content
        .as_array()
        .ok_or_else(|| {
            SdkError::message_parse("Assistant content must be an array", data.clone())
        })?
        .iter()
        .map(|block| parse_content_block(block, data))
        .collect::<Result<Vec<_>>>()?;

    Ok(Message::Assistant(AssistantMessage {
        content: content_blocks,
    }))
}

/// Parse a system message from JSON.
fn parse_system_message(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<Message> {
    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'subtype' field", data.clone()))?
        .to_string();

    let data_field = obj
        .get("data")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<HashMap<String, serde_json::Value>>()
        })
        .unwrap_or_default();

    Ok(Message::System(SystemMessage {
        subtype,
        data: data_field,
    }))
}

/// Parse a result message from JSON.
fn parse_result_message(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<Message> {
    let subtype = obj
        .get("subtype")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'subtype' field", data.clone()))?
        .to_string();

    let duration_ms = obj
        .get("duration_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| SdkError::message_parse("Missing 'duration_ms' field", data.clone()))?;

    let duration_api_ms = obj
        .get("duration_api_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| SdkError::message_parse("Missing 'duration_api_ms' field", data.clone()))?;

    let is_error = obj
        .get("is_error")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| SdkError::message_parse("Missing 'is_error' field", data.clone()))?;

    let num_turns = obj
        .get("num_turns")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| SdkError::message_parse("Missing 'num_turns' field", data.clone()))?
        as i32;

    let session_id = obj
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'session_id' field", data.clone()))?
        .to_string();

    let total_cost_usd = obj.get("total_cost_usd").and_then(|v| v.as_f64());

    let usage = obj.get("usage").and_then(|v| v.as_object()).map(|obj| {
        obj.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<String, serde_json::Value>>()
    });

    let result = obj.get("result").and_then(|v| v.as_str()).map(|s| s.to_string());

    Ok(Message::Result(ResultMessage {
        subtype,
        duration_ms,
        duration_api_ms,
        is_error,
        num_turns,
        session_id,
        total_cost_usd,
        usage,
        result,
    }))
}

/// Parse message content (can be text or blocks).
fn parse_message_content(
    content: &serde_json::Value,
    data: &serde_json::Value,
) -> Result<MessageContent> {
    if let Some(text) = content.as_str() {
        Ok(MessageContent::Text(text.to_string()))
    } else if let Some(blocks) = content.as_array() {
        let content_blocks = blocks
            .iter()
            .map(|block| parse_content_block(block, data))
            .collect::<Result<Vec<_>>>()?;
        Ok(MessageContent::Blocks(content_blocks))
    } else {
        Err(SdkError::message_parse(
            "Content must be string or array",
            data.clone(),
        ))
    }
}

/// Parse a content block from JSON.
fn parse_content_block(
    block: &serde_json::Value,
    data: &serde_json::Value,
) -> Result<ContentBlock> {
    let obj = block.as_object().ok_or_else(|| {
        SdkError::message_parse("Content block must be an object", data.clone())
    })?;

    let block_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'type' field in content block", data.clone()))?;

    match block_type {
        "text" => parse_text_block(obj, data),
        "tool_use" => parse_tool_use_block(obj, data),
        "tool_result" => parse_tool_result_block(obj, data),
        _ => Err(SdkError::message_parse(
            format!("Unknown content block type: {}", block_type),
            data.clone(),
        )),
    }
}

/// Parse a text block from JSON.
fn parse_text_block(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<ContentBlock> {
    let text = obj
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'text' field in text block", data.clone()))?
        .to_string();

    Ok(ContentBlock::Text(TextBlock { text }))
}

/// Parse a tool use block from JSON.
fn parse_tool_use_block(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<ContentBlock> {
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'id' field in tool_use block", data.clone()))?
        .to_string();

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| SdkError::message_parse("Missing 'name' field in tool_use block", data.clone()))?
        .to_string();

    let input = obj
        .get("input")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<HashMap<String, serde_json::Value>>()
        })
        .unwrap_or_default();

    Ok(ContentBlock::ToolUse(ToolUseBlock { id, name, input }))
}

/// Parse a tool result block from JSON.
fn parse_tool_result_block(
    obj: &serde_json::Map<String, serde_json::Value>,
    data: &serde_json::Value,
) -> Result<ContentBlock> {
    let tool_use_id = obj
        .get("tool_use_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            SdkError::message_parse("Missing 'tool_use_id' field in tool_result block", data.clone())
        })?
        .to_string();

    let content = obj.get("content").map(|v| parse_tool_result_content(v, data)).transpose()?;

    let is_error = obj.get("is_error").and_then(|v| v.as_bool());

    Ok(ContentBlock::ToolResult(ToolResultBlock {
        tool_use_id,
        content,
        is_error,
    }))
}

/// Parse tool result content.
fn parse_tool_result_content(
    content: &serde_json::Value,
    data: &serde_json::Value,
) -> Result<ToolResultContent> {
    if let Some(text) = content.as_str() {
        Ok(ToolResultContent::Text(text.to_string()))
    } else if let Some(array) = content.as_array() {
        let structured = array
            .iter()
            .map(|item| {
                item.as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<HashMap<String, serde_json::Value>>()
                    })
                    .ok_or_else(|| {
                        SdkError::message_parse(
                            "Structured tool result content must be array of objects",
                            data.clone(),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ToolResultContent::Structured(structured))
    } else {
        Err(SdkError::message_parse(
            "Tool result content must be string or array",
            data.clone(),
        ))
    }
}