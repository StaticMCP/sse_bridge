use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde_json::json;
use staticmcp_sse_lib::{MCPBridge, MCPRequest, create_bridge};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

type AppState = Arc<MCPBridge>;

async fn mcp_message_endpoint(
    State(bridge): State<AppState>,
    Json(request): Json<MCPRequest>,
) -> Json<serde_json::Value> {
    eprintln!("📨 MCP Message received");
    let response = bridge.handle_request(request).await;
    Json(serde_json::to_value(response).unwrap_or_default())
}

async fn info_endpoint(State(bridge): State<AppState>) -> Json<serde_json::Value> {
    let manifest_info = if let Some(manifest) = bridge.get_manifest() {
        json!({
            "serverInfo": manifest.server_info,
            "capabilities": manifest.capabilities
        })
    } else {
        json!({ "error": "Manifest not loaded" })
    };

    Json(json!({
        "bridge": "SSE Static MCP Bridge (Fixed Path)",
        "version": "1.0.0",
        "type": "fixed",
        "manifest": manifest_info,
        "endpoints": {
            "info": "GET /",
            "mcp_sse": "GET /sse (standard MCP SSE connection)",
            "mcp_message": "POST /message (standard MCP messages)",
            "mcp_sse_message": "POST /sse (standard MCP SSE messages)",
            "custom_mcp": "POST /mcp (custom JSON-RPC)",
            "custom_sse": "POST /sse_custom (custom SSE with request body)"
        },
        "usage": {
            "mcp_clients": "Point MCP client to: http://localhost:PORT/",
            "standard_endpoints": [
                "GET /sse (for SSE connection)",
                "POST /message (for messages)",
                "POST /sse (for SSE messages)"
            ],
            "custom_endpoints": [
                "POST /mcp (JSON-RPC)",
                "POST /sse_custom (SSE with body)"
            ]
        }
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <DATA_PATH> [PORT]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} ./my-static-mcp 3000", args[0]);
        eprintln!("  {} /path/to/mcp/data", args[0]);
        eprintln!("  {} https://staticmcp.com/mcp 3000", args[0]);
        eprintln!();
        eprintln!("The server will serve the StaticMCP data at the specified path via SSE.");
        std::process::exit(1);
    }

    let source_path = args[1].clone();
    let port = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(3000);

    eprintln!("🚀 Fixed Path SSE Bridge starting...");
    eprintln!("📍 Source: {source_path}");

    let bridge = match create_bridge(source_path).await {
        Ok(bridge) => Arc::new(bridge),
        Err(e) => {
            eprintln!("❌ Failed to initialize bridge: {e}");
            eprintln!();
            eprintln!("🔍 Troubleshooting:");
            eprintln!("  1. Check that mcp.json exists at the specified location");
            eprintln!("  2. Verify the URL/path is accessible");
            eprintln!("  3. Ensure the JSON is valid");
            std::process::exit(1);
        }
    };

    let app = Router::new()
        .route("/", get(info_endpoint))
        .route("/sse", post(mcp_message_endpoint))
        .layer(CorsLayer::permissive())
        .with_state(bridge);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    eprintln!("✅ Server ready at http://localhost:{port}");
    eprintln!("📖 Endpoints available:");
    eprintln!("   GET  http://localhost:{port}/     (info)");
    eprintln!("   POST http://localhost:{port}/sse  (MCP SSE messages)");
    eprintln!();
    eprintln!("🔌 For MCP clients: http://localhost:{port}/");

    axum::serve(listener, app).await?;
    Ok(())
}
