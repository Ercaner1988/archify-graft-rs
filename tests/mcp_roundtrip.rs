//! Integration test: MCP Protocol Roundtrip (inspired by Soup's mcp_roundtrip.py).
//! Tests initialize, tools/list, tool execution, and error resilience.

use graft_mcp::McpServer;
use serde_json::json;

#[tokio::test]
async fn test_mcp_initialize_and_tools_list() {
    let server = McpServer::new();

    // 1. Initialize
    let init_resp = server.handle_method("initialize", json!({})).await;
    assert_eq!(init_resp["serverInfo"]["name"], "archify-graft-rs");
    assert_eq!(init_resp["protocolVersion"], "2024-11-05");

    // 2. tools/list
    let list_resp = server.handle_method("tools/list", json!({})).await;
    let tools = list_resp["tools"].as_array().expect("tools must be array");
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(tool_names.contains(&"graft_index"));
    assert!(tool_names.contains(&"graft_ask"));
    assert!(tool_names.contains(&"graft_trace_calls"));
    assert!(tool_names.contains(&"archify_render_diagram"));
}

#[tokio::test]
async fn test_mcp_unindexed_error_handling() {
    let server = McpServer::new();

    // Calling graft_ask before index should return graceful error, not crash
    let ask_resp = server.handle_method("tools/call", json!({
        "name": "graft_ask",
        "arguments": { "query": "test" }
    })).await;

    assert_eq!(ask_resp["isError"], true);
    assert!(ask_resp["content"][0]["text"].as_str().unwrap().contains("not yet indexed"));
}

#[tokio::test]
async fn test_mcp_unknown_method() {
    let server = McpServer::new();

    let resp = server.handle_method("non_existent_method", json!({})).await;
    assert_eq!(resp["error"]["code"], -32601);
}
