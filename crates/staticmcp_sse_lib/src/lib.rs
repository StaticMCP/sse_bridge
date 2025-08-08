use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MCPManifest {
    #[serde(rename = "serverInfo")]
    pub server_info: Option<ServerInfo>,
    pub capabilities: Option<Capabilities>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Capabilities {
    pub resources: Option<Vec<Value>>,
    pub tools: Option<Vec<Value>>,
}

#[async_trait]
pub trait MCPDataSource: Send + Sync {
    async fn load_json(&self, relative_path: &str) -> anyhow::Result<Value>;
    async fn load_manifest(&self) -> anyhow::Result<MCPManifest>;
}

pub struct LocalDataSource {
    pub base_path: PathBuf,
}

impl LocalDataSource {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait]
impl MCPDataSource for LocalDataSource {
    async fn load_json(&self, relative_path: &str) -> anyhow::Result<Value> {
        let full_path = self.base_path.join(relative_path);
        eprintln!("📁 Reading: {}", full_path.display());
        let content = fs::read_to_string(full_path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    async fn load_manifest(&self) -> anyhow::Result<MCPManifest> {
        let manifest_data = self.load_json("mcp.json").await?;
        Ok(serde_json::from_value(manifest_data)?)
    }
}

pub struct RemoteDataSource {
    pub base_url: String,
    pub client: reqwest::Client,
}

impl RemoteDataSource {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl MCPDataSource for RemoteDataSource {
    async fn load_json(&self, relative_path: &str) -> anyhow::Result<Value> {
        let url = format!("{}/{}", self.base_url, relative_path);
        eprintln!("🌐 Fetching: {url}");

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            );
        }

        let text = response.text().await?;
        Ok(serde_json::from_str(&text)?)
    }

    async fn load_manifest(&self) -> anyhow::Result<MCPManifest> {
        let manifest_data = self.load_json("mcp.json").await?;
        Ok(serde_json::from_value(manifest_data)?)
    }
}

pub struct MCPBridge {
    pub data_source: Box<dyn MCPDataSource>,
    pub manifest: Option<MCPManifest>,
}

impl MCPBridge {
    pub fn new(data_source: Box<dyn MCPDataSource>) -> Self {
        Self {
            data_source,
            manifest: None,
        }
    }

    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        self.manifest = Some(self.data_source.load_manifest().await?);

        if let Some(manifest) = &self.manifest {
            let server_name = manifest
                .server_info
                .as_ref()
                .map(|s| s.name.as_str())
                .unwrap_or("Unknown");
            let server_version = manifest
                .server_info
                .as_ref()
                .map(|s| s.version.as_str())
                .unwrap_or("0.0.0");

            eprintln!("✅ Loaded manifest: {server_name} v{server_version}");
        }

        Ok(())
    }

    pub fn get_manifest(&self) -> Option<&MCPManifest> {
        self.manifest.as_ref()
    }

    pub fn uri_to_path(&self, uri: &str) -> String {
        if uri.starts_with("file://") {
            format!("resources/{}.json", uri.strip_prefix("file://").unwrap())
        } else if uri.contains("://") {
            let parts: Vec<&str> = uri.split("://").collect();
            if parts.len() == 2 {
                format!("resources/{}.json", parts[1])
            } else {
                format!("{uri}.json")
            }
        } else if uri.ends_with(".json") {
            uri.to_string()
        } else {
            format!("{uri}.json")
        }
    }

    pub fn tool_to_path(&self, tool_name: &str, args: &HashMap<String, Value>) -> String {
        let tool_dir = format!("tools/{tool_name}");

        if args.is_empty() {
            return format!("{tool_dir}.json");
        }

        if args.len() == 1 {
            let arg_value = args.values().next().unwrap();
            let arg_str = match arg_value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => serde_json::to_string(arg_value).unwrap_or_default(),
            };
            return format!("{tool_dir}/{arg_str}.json");
        }

        if args.len() == 2 {
            let mut values: Vec<String> = args
                .values()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => serde_json::to_string(v).unwrap_or_default(),
                })
                .collect();
            values.sort();
            return format!("{}/{}/{}.json", tool_dir, values[0], values[1]);
        }

        // Multiple arguments - create a hash-like path
        let mut sorted_args: Vec<(String, String)> = args
            .iter()
            .map(|(k, v)| {
                let val_str = match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => serde_json::to_string(v).unwrap_or_default(),
                };
                (k.clone(), val_str)
            })
            .collect();
        sorted_args.sort_by(|a, b| a.0.cmp(&b.0));

        let arg_string = sorted_args
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");

        let hash = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            arg_string.as_bytes(),
        )
        .replace(['/', '+', '='], "_");

        format!("{tool_dir}/{hash}.json")
    }

    pub async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id),
            "resources/list" => self.handle_list_resources(request.id).await,
            "resources/read" => {
                self.handle_read_resource(request.id, request.params.unwrap_or(json!({})))
                    .await
            }
            "tools/list" => self.handle_list_tools(request.id).await,
            "tools/call" => {
                self.handle_call_tool(request.id, request.params.unwrap_or(json!({})))
                    .await
            }
            _ => MCPResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(MCPError {
                    code: -32601,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            },
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> MCPResponse {
        MCPResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "resources": {},
                    "tools": {}
                },
                "serverInfo": {
                    "name": "sse-static-mcp-bridge",
                    "version": "1.0.0"
                }
            })),
            error: None,
        }
    }

    async fn handle_list_resources(&self, id: Option<Value>) -> MCPResponse {
        if let Some(manifest) = &self.manifest {
            let resources = manifest
                .capabilities
                .as_ref()
                .and_then(|c| c.resources.as_ref())
                .cloned()
                .unwrap_or_default();

            eprintln!("📋 Listed {} resources", resources.len());

            MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "resources": resources })),
                error: None,
            }
        } else {
            MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(MCPError {
                    code: -32603,
                    message: "Manifest not loaded".to_string(),
                    data: None,
                }),
            }
        }
    }

    async fn handle_read_resource(&self, id: Option<Value>, params: Value) -> MCPResponse {
        let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
        eprintln!("📖 Reading resource: {uri}");

        let resource_path = self.uri_to_path(uri);

        match self.data_source.load_json(&resource_path).await {
            Ok(resource) => {
                let contents = if let Some(contents) = resource.get("contents") {
                    contents.clone()
                } else if resource.get("uri").is_some()
                    && resource.get("mimeType").is_some()
                    && resource.get("text").is_some()
                {
                    json!([{
                        "uri": resource["uri"],
                        "mimeType": resource["mimeType"],
                        "text": resource["text"]
                    }])
                } else {
                    json!([{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&resource).unwrap_or_default()
                    }])
                };

                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "contents": contents })),
                    error: None,
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading resource {uri}: {e}");
                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(MCPError {
                        code: -32603,
                        message: format!("Failed to read resource {uri}: {e}"),
                        data: None,
                    }),
                }
            }
        }
    }

    async fn handle_list_tools(&self, id: Option<Value>) -> MCPResponse {
        if let Some(manifest) = &self.manifest {
            let tools = manifest
                .capabilities
                .as_ref()
                .and_then(|c| c.tools.as_ref())
                .cloned()
                .unwrap_or_default();

            eprintln!("🔧 Listed {} tools", tools.len());

            MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({ "tools": tools })),
                error: None,
            }
        } else {
            MCPResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(MCPError {
                    code: -32603,
                    message: "Manifest not loaded".to_string(),
                    data: None,
                }),
            }
        }
    }

    async fn handle_call_tool(&self, id: Option<Value>, params: Value) -> MCPResponse {
        let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let arguments = params
            .get("arguments")
            .and_then(|a| a.as_object())
            .cloned()
            .unwrap_or_default();

        let args_map: HashMap<String, Value> = arguments.into_iter().collect();

        eprintln!("🛠️  Calling tool: {name} with args: {args_map:?}");

        let tool_path = self.tool_to_path(name, &args_map);

        match self.data_source.load_json(&tool_path).await {
            Ok(result) => {
                let content = if result.get("content").is_some() || result.get("contents").is_some()
                {
                    result
                } else {
                    json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    })
                };

                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(content),
                    error: None,
                }
            }
            Err(e) => {
                eprintln!("❌ Error calling tool {name}: {e}");
                MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Error calling {}: {}", name, e)
                        }],
                        "isError": true
                    })),
                    error: None,
                }
            }
        }
    }
}

// Convenience functions to create bridges
pub async fn create_local_bridge(path: PathBuf) -> anyhow::Result<MCPBridge> {
    let data_source = Box::new(LocalDataSource::new(path));
    let mut bridge = MCPBridge::new(data_source);
    bridge.initialize().await?;
    Ok(bridge)
}

pub async fn create_remote_bridge(url: String) -> anyhow::Result<MCPBridge> {
    let data_source = Box::new(RemoteDataSource::new(url));
    let mut bridge = MCPBridge::new(data_source);
    bridge.initialize().await?;
    Ok(bridge)
}

pub async fn create_bridge(source_path: String) -> anyhow::Result<MCPBridge> {
    if source_path.starts_with("http://") || source_path.starts_with("https://") {
        create_remote_bridge(source_path).await
    } else {
        create_local_bridge(PathBuf::from(source_path)).await
    }
}
