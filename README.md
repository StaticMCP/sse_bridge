# StaticMCP SSE Bridge

A SSE bridge that enables MCP clients to connect to StaticMCP servers over HTTP. This bridge translates MCP protocol requests into static file operations, allowing AI models and applications to seamlessly interact with pre-generated static content.

## Overview

This repository contains two SSE bridge implementations:

- **`staticmcp_sse_dynamic`**: A generic bridge that can proxy to any StaticMCP URL via query parameters
- **`staticmcp_sse_fixed`**: A fixed bridge that serves a specific StaticMCP directory or URL

## Features

- 🌐 **HTTP/SSE Transport**: Modern web-compatible MCP transport
- 🔗 **Dynamic URL Support**: Connect to any StaticMCP endpoint via URL parameters
- ⚡ **High Performance**: Async Rust implementation with efficient HTTP handling
- 🛡️ **CORS Support**: Built-in CORS headers for web client compatibility
- 🧪 **Testing Endpoints**: Built-in connection testing and manifest inspection
- 📊 **Multiple Endpoints**: Support for both standard MCP and custom endpoints

## Quick Start

### Installation

```bash
# Clone and build
git clone git@github.com/StaticMCP/sse_bridge
cd sse_bridge
cargo build --release

# Or install via cargo
cargo install staticmcp_sse_dynamic staticmcp_sse_fixed
```

### Running the Bridge

```bash
# Start on default port 3000
./target/release/staticmcp_sse_dynamic

# Start on custom port
./target/release/staticmcp_sse_dynamic 8080

# Start fixed bridge
./target/release/staticmcp_sse_fixed ./mcp-json-dir/

# Start fixed bridge for remote content
./target/release/staticmcp_sse_fixed https://staticmcp.com/mcp
```

## Usage

### For MCP Clients

```bash
# Using MCP Inspector with dynamic bridge
npx @modelcontextprotocol/inspector "http://localhost:3000/sse?url=https://staticmcp.com/mcp"

# Using MCP Inspector with fixed bridge
npx @modelcontextprotocol/inspector "http://localhost:3000/sse"
```

### API Endpoints

- **`GET /`** - Bridge information and usage examples
- **`POST /sse?url={target_url}`** - (dynamic-only) SSE message posting endpoint

## How It Works

1. **Request Reception**: The bridge receives MCP requests via HTTP/SSE
2. **URL Resolution**: Extracts the target StaticMCP URL from query parameters
3. **Remote Bridge Creation**: Creates a connection to the remote StaticMCP endpoint
4. **Request Forwarding**: Translates and forwards MCP requests to static files
5. **Response Translation**: Converts file responses back to MCP format

### File Path Mapping

The bridge automatically maps MCP operations to static file paths:

- `resources/read` → `resources/{resource_name}.json`
- `tools/call` → `tools/{tool_name}/{args}.json`
- `tools/list` → manifest from `mcp.json`

## Configuration

### Environment Variables

- **Port**: Set via command line argument (default: 3000)
- **CORS**: Permissive CORS enabled by default

## Comparisons

| Feature | Dynamic Bridge | Fixed Bridge |
|---------|---------------------------|--------------|
| **Flexibility** | Connect to any StaticMCP URL | Single pre-configured endpoint |
| **Use Case** | Multi-tenant | Single StaticMCP |
| **Performance** | Slight overhead for URL parsing | Optimized for single endpoint |
| **Security** | URL parameter validation needed | Controlled access |

## Development

### Prerequisites

- Rust 1.70+
- Cargo

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

## Related

- [StaticMCP](https://staticmcp.org) - Static file-based MCP servers
- [StaticMCP STDIO Bridge](https://github.com/StaticMCP/staticmcp_stdio_bridge) - Command-line bridge
- [MCP Inspector](https://github.com/modelcontextprotocol/inspector) - MCP debugging tool
