//! OpenAI provider implementation.
//!
//! This module provides an `AiProvider` implementation using the OpenAI API.

use crate::ai::{AiProvider, AiResponse, Message, ToolCall, ToolDefinition};
use crate::error::{Error, Result};
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestToolMessage,
        ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionTool, ChatCompletionTools,
        FunctionCall, FunctionObject,
    },
};
use async_trait::async_trait;
use serde_json::json;
use std::env;

pub struct OpenAiProvider {
    client: async_openai::Client<OpenAIConfig>,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: Option<&str>) -> Result<Self> {
        let mut config = OpenAIConfig::new();

        if let Some(key) = api_key {
            config = config.with_api_key(key);
        }

        // Check for custom API base URL (e.g., for local LLM servers like llama.cpp)
        if let Ok(api_base) = env::var("OPENAI_API_BASE") {
            config = config.with_api_base(&api_base);
        }

        let client = async_openai::Client::with_config(config);
        Ok(Self {
            client,
            model: "gpt-4".to_string(),
        })
    }

    fn convert_messages(messages: &[Message]) -> Vec<ChatCompletionRequestMessage> {
        messages
            .iter()
            .map(|msg| {
                match msg.role {
                    crate::ai::MessageRole::User => {
                        let user_message = ChatCompletionRequestUserMessage {
                            content: ChatCompletionRequestUserMessageContent::Text(
                                msg.content.clone(),
                            ),
                            name: None,
                        };
                        ChatCompletionRequestMessage::User(user_message)
                    }
                    crate::ai::MessageRole::Assistant => {
                        // Convert tool calls from our format to OpenAI's format
                        let tool_calls = if let Some(ref tool_calls) = msg.tool_calls {
                            Some(
                                tool_calls
                                    .iter()
                                    .map(|tc| {
                                        // Convert arguments Value to JSON string
                                        let arguments = serde_json::to_string(&tc.arguments)
                                            .unwrap_or_else(|_| "{}".to_string());

                                        ChatCompletionMessageToolCalls::Function(
                                            ChatCompletionMessageToolCall {
                                                id: tc.id.clone(),
                                                function: FunctionCall {
                                                    name: tc.name.clone(),
                                                    arguments,
                                                },
                                            },
                                        )
                                    })
                                    .collect(),
                            )
                        } else {
                            None
                        };

                        #[allow(deprecated)]
                        let assistant_message = ChatCompletionRequestAssistantMessage {
                            content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                                msg.content.clone(),
                            )),
                            refusal: None,
                            name: None,
                            audio: None,
                            tool_calls,
                            function_call: None,
                        };
                        ChatCompletionRequestMessage::Assistant(assistant_message)
                    }
                    crate::ai::MessageRole::Tool => {
                        let tool_message = ChatCompletionRequestToolMessage {
                            content: ChatCompletionRequestToolMessageContent::Text(
                                msg.content.clone(),
                            ),
                            tool_call_id: msg
                                .tool_call_id
                                .clone()
                                .unwrap_or_else(|| "test".to_string()),
                        };
                        ChatCompletionRequestMessage::Tool(tool_message)
                    }
                }
            })
            .collect()
    }

    fn convert_tools(tools: &[ToolDefinition]) -> Vec<ChatCompletionTools> {
        tools
            .iter()
            .map(|tool| {
                ChatCompletionTools::Function(ChatCompletionTool {
                    function: FunctionObject {
                        name: tool.name.clone(),
                        description: Some(tool.description.clone()),
                        parameters: Some(tool.parameters.clone()),
                        strict: None,
                    },
                })
            })
            .collect()
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<AiResponse> {
        use async_openai::types::chat::ChatCompletionToolChoiceOption::Mode;
        use async_openai::types::chat::CreateChatCompletionRequest;
        use async_openai::types::chat::ToolChoiceOptions;

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: Self::convert_messages(&messages),
            tools: Some(Self::convert_tools(&tools)),
            tool_choice: Some(Mode(ToolChoiceOptions::Auto)),
            ..Default::default()
        };

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .map_err(|e| Error::Ai(format!("OpenAI API error: {}", e)))?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_default();

        let tool_calls = response
            .choices
            .first()
            .and_then(|choice| choice.message.tool_calls.clone())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tc| match tc {
                ChatCompletionMessageToolCalls::Function(tc) => Some(tc),
                _ => None, // ignore custom tool calls for now
            })
            .map(|tc| ToolCall {
                id: tc.id,
                name: tc.function.name,
                arguments: serde_json::from_str(&tc.function.arguments).unwrap_or(json!({})),
            })
            .collect();

        Ok(AiResponse {
            content,
            tool_calls,
        })
    }
}
