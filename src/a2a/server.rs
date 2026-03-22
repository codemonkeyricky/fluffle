use crate::a2a::registry::AgentRegistry;
use crate::a2a::types::{A2AMessage, A2ARequest, A2AResponse, Part, Task, TaskStatus};
use crate::config::Config;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentServer {
    pub profile_name: String,
    pub config: Config,
    pub workdir: PathBuf,
    /// Registry to propagate into spawned agents so they can call sub-agents.
    pub registry: Option<Arc<Mutex<AgentRegistry>>>,
}

/// Bind `listener` and serve the A2A HTTP API until shutdown.
pub async fn serve(
    server: AgentServer,
    listener: tokio::net::TcpListener,
) -> anyhow::Result<()> {
    let state = Arc::new(server);
    let app = Router::new()
        .route("/.well-known/agent.json", get(agent_card_handler))
        .route("/", post(tasks_send_handler))
        .with_state(state);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn agent_card_handler(State(server): State<Arc<AgentServer>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": server.profile_name,
        "description": format!("{} agent", server.profile_name),
        "version": "1.0.0",
        "capabilities": { "streaming": false },
        "skills": [{
            "id": server.profile_name,
            "name": server.profile_name,
            "description": format!("{} agent", server.profile_name)
        }]
    }))
}

async fn tasks_send_handler(
    State(server): State<Arc<AgentServer>>,
    Json(request): Json<A2ARequest>,
) -> Json<A2AResponse> {
    let input = request
        .params
        .message
        .parts
        .first()
        .map(|p| p.text.as_str())
        .unwrap_or("")
        .to_string();

    let (state, text) = match run_agent(&server, &input).await {
        Ok(text) => ("completed".to_string(), text),
        Err(e) => ("failed".to_string(), e.to_string()),
    };

    Json(A2AResponse {
        jsonrpc: "2.0".to_string(),
        id: request.id,
        result: Task {
            id: request.params.id,
            status: TaskStatus { state },
            messages: vec![A2AMessage {
                role: "agent".to_string(),
                parts: vec![Part {
                    kind: "text".to_string(),
                    text,
                }],
            }],
        },
    })
}

async fn run_agent(server: &AgentServer, input: &str) -> anyhow::Result<String> {
    let mut agent = crate::Agent::new_with_profile(
        &server.profile_name,
        server.config.clone(),
        Some(server.workdir.clone()),
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    if let Some(reg) = &server.registry {
        agent.set_registry(reg.clone());
    }

    agent.process(input).await.map_err(|e| anyhow::anyhow!("{}", e))
}
