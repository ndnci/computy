#![cfg(feature = "cli")]
use computy_engine::{
    inventory::Inventory,
    mcp::{McpServer, Permissions, authorize},
    repository::FileRepository,
    service::{AppService, HardwareProvider},
};
use serde_json::json;
use std::sync::Arc;
struct Hardware;
impl HardwareProvider for Hardware {
    fn collect(&self) -> Inventory {
        Inventory::default()
    }
}
fn server() -> (McpServer, std::path::PathBuf) {
    let root =
        std::env::temp_dir().join(format!("computy-mcp-{}", computy_engine::domain::new_id()));
    let repository = Arc::new(FileRepository::new(root.join("reports")).unwrap());
    let service = AppService::new(repository, Arc::new(Hardware), root.join("scratch"));
    (McpServer::new(service, Permissions::default()), root)
}
#[test]
fn mcp_negotiates_and_readonly_tools_cannot_mutate() {
    let (server, root) = server();
    let init=server.handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    let tools = server
        .handle(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
        .unwrap();
    assert!(
        !tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "computy_start_diagnostic")
    );
    let forbidden=server.handle(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"computy_delete_report","arguments":{"id":"123"}}})).unwrap();
    assert_eq!(forbidden["result"]["isError"], true);
    assert!(
        server
            .handle(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .is_none()
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn http_requires_authentication_fresh_token_safe_host_and_origin() {
    let token = "a".repeat(64);
    let header = format!("Bearer {token}");
    assert_eq!(
        authorize(&token, None, false, None, &[], "127.0.0.1:8765", 8765),
        Err(401)
    );
    assert_eq!(
        authorize(
            &token,
            Some(&header),
            true,
            None,
            &[],
            "127.0.0.1:8765",
            8765
        ),
        Err(401)
    );
    assert_eq!(
        authorize(
            &token,
            Some(&header),
            false,
            Some("https://evil.example"),
            &[],
            "127.0.0.1:8765",
            8765
        ),
        Err(403)
    );
    assert_eq!(
        authorize(
            &token,
            Some(&header),
            false,
            None,
            &[],
            "evil.example",
            8765
        ),
        Err(403)
    );
    assert!(
        authorize(
            &token,
            Some(&header),
            false,
            None,
            &[],
            "127.0.0.1:8765",
            8765
        )
        .is_ok()
    );
}
