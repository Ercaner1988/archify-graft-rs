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
    let ask_resp = server
        .handle_method(
            "tools/call",
            json!({
                "name": "graft_ask",
                "arguments": { "query": "test" }
            }),
        )
        .await;

    assert_eq!(ask_resp["isError"], true);
    assert!(ask_resp["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("not yet indexed"));
}

#[tokio::test]
async fn test_mcp_unknown_method() {
    let server = McpServer::new();

    let resp = server.handle_method("non_existent_method", json!({})).await;
    assert_eq!(resp["error"]["code"], -32601);
}

/// `graft_hedef` iki sınıfı ayırmalı: tanınan durum sözcükleri tahtayı okur,
/// tanınmayan sözcük `isError` ile geri döner. Ajanın "hepsi tamam" diye
/// okuduğu bir tahta, ölçülmemiş satırı yeşile boyamak olurdu.
#[tokio::test]
async fn test_mcp_hedef_board_and_unreadable_row() {
    let kok = std::env::temp_dir().join(format!("agr-hedef-mcp-{}", std::process::id()));
    let harness = kok.join(".harness");
    std::fs::create_dir_all(&harness).unwrap();
    let beyan = harness.join("feature-list.json");

    std::fs::write(
        &beyan,
        r#"{"mission":"deneme","features":[{"id":"A","name":"biten","status":"done"}]}"#,
    )
    .unwrap();
    let server = McpServer::new();
    let ok = server
        .handle_method(
            "tools/call",
            json!({ "name": "graft_hedef", "arguments": { "path": kok.to_string_lossy() } }),
        )
        .await;
    assert!(ok.get("isError").is_none(), "{ok}");
    let metin = ok["content"][0]["text"].as_str().unwrap();
    assert!(metin.contains("misyon: deneme"), "{metin}");
    assert!(metin.contains("hedefler: 1 (1 tamam"), "{metin}");

    std::fs::write(
        &beyan,
        r#"{"features":[{"id":"B","name":"belirsiz","status":"kafamda_bitti"}]}"#,
    )
    .unwrap();
    let bozuk = server
        .handle_method(
            "tools/call",
            json!({ "name": "graft_hedef", "arguments": { "path": kok.to_string_lossy(), "ozet": true } }),
        )
        .await;
    assert_eq!(bozuk["isError"], true, "{bozuk}");
    let metin = bozuk["content"][0]["text"].as_str().unwrap();
    assert!(metin.contains("1 okunamadı"), "{metin}");
    assert!(metin.contains("yeşil sayılmadı"), "{metin}");

    std::fs::remove_dir_all(&kok).ok();
}
