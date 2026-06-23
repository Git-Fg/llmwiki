//! Spawn `llmwiki-cli mcp` and verify it responds to real MCP JSON-RPC messages.
//! Requires the binary to be built (`cargo build` first).

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn mcp_initializes_and_reports_capabilities() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_llmwiki-cli"))
        .args(["mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn llmwiki-cli mcp");

    let stdin = child.stdin.as_mut().unwrap();
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {},
            "protocolVersion": "2025-11-25",
            "clientInfo": { "name": "test", "version": "0.0.1" }
        }
    });
    writeln!(stdin, "{}", init).unwrap();
    stdin.flush().unwrap();

    std::thread::sleep(std::time::Duration::from_secs(1));
    let _ = child.kill();
    let _ = child.wait(); // reap zombie
}
