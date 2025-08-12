use axum::response::sse::Event;
use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::Sse,
    routing::{get, post},
};
use futures::stream;
use serde::Deserialize;
use serde_json::json;
use staticmcp_sse_lib::{MCPRequest, create_remote_bridge};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {}

#[derive(Deserialize)]
struct RemoteParams {
    url: String,
}

async fn mcp_sse_endpoint(
    Query(params): Query<RemoteParams>,
    Json(request): Json<MCPRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    eprintln!("🎯 MCP Message to: {}", params.url);

    match create_remote_bridge(params.url).await {
        Ok(bridge) => {
            let response = bridge.handle_request(request).await;
            Ok(Json(serde_json::to_value(response).unwrap_or_default()))
        }
        Err(e) => {
            eprintln!("❌ Failed to create remote bridge: {e}");
            Ok(Json(json!({
                "jsonrpc": "2.0",
                "id": request.id,
                "error": {
                    "code": -32603,
                    "message": format!("Failed to connect to remote MCP: {}", e)
                }
            })))
        }
    }
}

async fn sse_endpoint(
    Query(_params): Query<RemoteParams>,
) -> Sse<impl futures::Stream<Item = Result<Event, axum::Error>>> {
    let stream = stream::iter(vec![
        Ok(Event::default().data("Hello SSE")),
        Ok(Event::default().data("Connection established")),
        Ok(Event::default()
            .event("ready")
            .data(r#"{"jsonrpc":"2.0","method":"ready"}"#)),
    ]);

    Sse::new(stream)
}

async fn info_endpoint() -> Json<serde_json::Value> {
    Json(json!({
        "bridge": "SSE Static MCP Bridge (Generic Remote)",
        "version": "1.0.0",
        "type": "generic",
        "description": "Generic bridge that can proxy to any remote StaticMCP",
        "endpoints": {
            "info": "GET /",
            "mcp_sse": "POST /sse?url={target_mcp_url}",
            "mcp_sse_events": "GET /events?url={target_mcp_url}",
        },
        "usage": {
            "mcp_clients": "Point MCP client to: http://localhost:PORT/sse?url=TARGET_URL",
            "standard_endpoints": [
                "GET / (for info)",
                "POST /sse?url=https://staticmcp.com/mcp (for SSE messages)"
            ],
        }
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(3000);

    let state = Arc::new(AppState {});

    eprintln!("🚀 Generic SSE Static MCP Bridge starting...");
    eprintln!("🌐 Server will be available at: http://localhost:{port}");
    eprintln!();
    eprintln!("📖 Usage Examples:");
    eprintln!("  Info: GET http://localhost:{port}/");
    eprintln!("  SSE: POST http://localhost:{port}/sse?url=https://staticmcp.com/mcp");

    let app = Router::new()
        .route("/", get(info_endpoint))
        .route("/sse", post(mcp_sse_endpoint))
        .route("/events", get(sse_endpoint))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    eprintln!("✅ Generic bridge ready!");
    eprintln!("🔗 Point your MCP client to: http://localhost:{port}/sse?url=TARGET_URL");
    eprintln!("🧪 Test:");
    eprintln!("> curl -X POST 'http://localhost:{port}/sse?url=https://staticmcp.com/mcp' \\");
    eprintln!("-H \"Content-Type: application/json\" \\");
    eprintln!("-d '{{");
    eprintln!("  \"jsonrpc\": \"2.0\",");
    eprintln!("  \"id\": 1,");
    eprintln!("  \"method\": \"initialize\",");
    eprintln!("  \"params\": {{");
    eprintln!("    \"protocolVersion\": \"2025-06-18\",");
    eprintln!("    \"capabilities\": {{}},");
    eprintln!("    \"clientInfo\": {{\"name\": \"test\", \"version\": \"1.0\"}}");
    eprintln!("  }}");
    eprintln!("}}'");

    axum::serve(listener, app).await?;

    Ok(())
}
