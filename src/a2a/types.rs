use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct A2ARequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: TaskParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskParams {
    pub id: String,
    pub session_id: String,
    pub message: A2AMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AMessage {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Part {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct A2AResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: Task,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub status: TaskStatus,
    pub messages: Vec<A2AMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStatus {
    /// "submitted" | "working" | "completed" | "failed"
    pub state: String,
}
