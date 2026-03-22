use crate::a2a::types::{A2AMessage, A2ARequest, A2AResponse, Part, TaskParams};
use crate::types::ToolResult;
use uuid::Uuid;

/// POST a `tasks/send` request to `url` and return the agent's response as a `ToolResult`.
pub async fn send_task(url: &str, input: &str) -> ToolResult {
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        id: Uuid::new_v4().to_string(),
        method: "tasks/send".to_string(),
        params: TaskParams {
            id: Uuid::new_v4().to_string(),
            session_id: Uuid::new_v4().to_string(),
            message: A2AMessage {
                role: "user".to_string(),
                parts: vec![Part {
                    kind: "text".to_string(),
                    text: input.to_string(),
                }],
            },
        },
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
    {
        Ok(c) => c,
        Err(e) => return ToolResult::error(format!("Failed to create HTTP client: {}", e)),
    };

    let response = match client.post(url).json(&request).send().await {
        Ok(r) => r,
        Err(e) => return ToolResult::error(format!("A2A request to {} failed: {}", url, e)),
    };

    let a2a_response: A2AResponse = match response.json().await {
        Ok(r) => r,
        Err(e) => return ToolResult::error(format!("Failed to parse A2A response: {}", e)),
    };

    let task = a2a_response.result;
    let text = task
        .messages
        .first()
        .and_then(|m| m.parts.first())
        .map(|p| p.text.as_str())
        .unwrap_or("")
        .to_string();

    if task.status.state == "completed" {
        ToolResult::success(text)
    } else {
        ToolResult::error(text)
    }
}
