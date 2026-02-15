// HTTP server for frontend communication
use crate::daemon::rpc_server::process_request;
use axum::{
    Router,
    routing::post,
    http::StatusCode,
};
use tower_http::cors::{Any, CorsLayer};

pub struct HttpServer {
    port: u16,
}

impl HttpServer {
    pub fn new(port: u16) -> Self {
        HttpServer { port }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/rpc", post(handle_rpc))
            .layer(cors);

        let addr = format!("127.0.0.1:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        println!("[HTTP] Server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle_rpc(body: String) -> (StatusCode, String) {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return (StatusCode::BAD_REQUEST, "Empty request".to_string());
    }

    let response = process_request(trimmed).await;

    match serde_json::to_string(&response) {
        Ok(json) => (StatusCode::OK, json),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Serialization error: {}", e)),
    }
}
