//! Anthropic provider implementation.
//!
//! This module provides an `AiProvider` implementation using the Anthropic API.

use crate::ai::{AiProvider, AiResponse, Message, ToolCall, ToolDefinition};
use crate::error::{Error, Result};
use anthropic_sdk::{Client, ToolChoice};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct AnthropicProvider {
    api_key: Option<String>,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: Option<&str>) -> Result<Self> {
        Ok(Self {
            api_key: api_key.map(|s| s.to_string()),
            model: "claude-3-haiku-20240307".to_string(), // TODO: make configurable
        })
    }

    fn convert_messages(messages: &[Message]) -> Value {
        let mut anthropic_messages = Vec::new();
        for msg in messages {
            match msg.role {
                crate::ai::MessageRole::User => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
                crate::ai::MessageRole::Assistant => {
                    // Assistant messages may contain tool calls (tool_use blocks).
                    let mut content_blocks = Vec::new();
                    // Add text block if content is not empty.
                    if !msg.content.is_empty() {
                        content_blocks.push(json!({
                            "type": "text",
                            "text": msg.content,
                        }));
                    }
                    // Add tool_use blocks if any.
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tool_call in tool_calls {
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": tool_call.id,
                                "name": tool_call.name,
                                "input": tool_call.arguments,
                            }));
                        }
                    }
                    // Determine content field: if only one text block and no tool calls, can be string.
                    let content = if content_blocks.len() == 1
                        && content_blocks[0].get("type").and_then(|t| t.as_str()) == Some("text")
                        && msg.tool_calls.is_none()
                    {
                        json!(msg.content)
                    } else {
                        json!(content_blocks)
                    };
                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": content,
                    }));
                }
                crate::ai::MessageRole::Tool => {
                    // Tool messages are represented as user messages with tool_result content block.
                    // tool_call_id is required.
                    if let Some(tool_call_id) = &msg.tool_call_id {
                        anthropic_messages.push(json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": tool_call_id,
                                "content": msg.content,
                            }],
                        }));
                    } else {
                        // fallback: just text
                        anthropic_messages.push(json!({
                            "role": "user",
                            "content": msg.content,
                        }));
                    }
                }
            }
        }
        json!(anthropic_messages)
    }

    fn convert_tools(tools: &[ToolDefinition]) -> Value {
        let mut anthropic_tools = Vec::new();
        for tool in tools {
            anthropic_tools.push(json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters,
            }));
        }
        json!(anthropic_tools)
    }

    fn parse_response(response_json: &str) -> Result<AiResponse> {
        let v: Value = serde_json::from_str(response_json)
            .map_err(|e| Error::Ai(format!("Failed to parse Anthropic response: {}", e)))?;

        // Check if it's an error response
        if let Some(error_type) = v.get("type").and_then(|t| t.as_str()) {
            if error_type == "error" {
                let message = v
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(Error::Ai(format!("Anthropic API error: {}", message)));
            }
        }

        let content = v
            .get("content")
            .and_then(|c| c.as_array())
            .ok_or_else(|| Error::Ai("Missing content array in response".to_string()))?;

        let mut text_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for block in content {
            let block_type = block
                .get("type")
                .and_then(|t| t.as_str())
                .ok_or_else(|| Error::Ai("Missing type in content block".to_string()))?;
            match block_type {
                "text" => {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        text_parts.push(text.to_string());
                    }
                }
                "tool_use" => {
                    let id = block
                        .get("id")
                        .and_then(|id| id.as_str())
                        .ok_or_else(|| Error::Ai("Missing id in tool_use block".to_string()))?
                        .to_string();
                    let name = block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| Error::Ai("Missing name in tool_use block".to_string()))?
                        .to_string();
                    let input = block.get("input").cloned().unwrap_or_else(|| json!({}));
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: input,
                    });
                }
                _ => {}
            }
        }

        let combined_text = text_parts.join("");
        Ok(AiResponse {
            content: combined_text,
            tool_calls,
        })
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<AiResponse> {
        let mut client = Client::new();

        if let Some(api_key) = &self.api_key {
            client = client.auth(api_key);
        }

        let messages_json = Self::convert_messages(&messages);
        let tools_json = Self::convert_tools(&tools);

        let request_builder = client
            .model(&self.model)
            .max_tokens(4096)
            .stream(false)
            .tool_choice(ToolChoice::Auto)
            .messages(&messages_json)
            .tools(&tools_json)
            .builder()
            .map_err(|e| Error::Ai(format!("Failed to build Anthropic request: {}", e)))?;

        let response = request_builder
            .send()
            .await
            .map_err(|e| Error::Ai(format!("Failed to send Anthropic request: {}", e)))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::Ai(format!("Failed to read Anthropic response: {}", e)))?;

        if !status.is_success() {
            return Err(Error::Ai(format!(
                "Anthropic API error ({}): {}",
                status, response_text
            )));
        }

        Self::parse_response(&response_text)
    }
}
