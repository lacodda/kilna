//! `kilna --mcp` as a client sees it: the real binary, a pipe each way.
//!
//! The unit tests in `mcp.rs` cover what the tools answer; this covers the
//! part they cannot — that the executable takes the flag, opens a workspace
//! of its own, speaks one JSON line per message on stdout and nothing else,
//! and stops when the pipe closes. Those are the failures a client actually
//! meets, and none of them shows up in a function call.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

/// A fresh workspace directory for one test, under the system temp dir.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kilna-mcp-{name}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Start the server on its own workspace.
///
/// A binary that was linked a moment ago can still be busy on Linux
/// (`ETXTBSY`); the retry is narrow — that one error, a few tries — so a
/// genuinely broken binary still fails, and fails fast.
fn spawn(workspace: &PathBuf) -> Child {
    let mut attempt = 0;
    loop {
        let spawned = Command::new(env!("CARGO_BIN_EXE_kilna"))
            .arg("--mcp")
            .arg("--workspace")
            .arg(workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        match spawned {
            Ok(child) => return child,
            Err(err) if attempt < 5 && err.to_string().contains("Text file busy") => {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(err) => panic!("cannot start kilna --mcp: {err}"),
        }
    }
}

struct Server {
    child: Child,
    out: BufReader<std::process::ChildStdout>,
}

impl Server {
    fn start(workspace: &PathBuf) -> Self {
        let mut child = spawn(workspace);
        let out = BufReader::new(child.stdout.take().unwrap());
        Self { child, out }
    }

    fn send(&mut self, message: Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// The next line on stdout, as JSON. Anything that is not JSON is the
    /// failure this test exists to catch.
    fn receive(&mut self) -> Value {
        let mut line = String::new();
        let read = self.out.read_line(&mut line).unwrap();
        assert!(read > 0, "the server closed stdout before answering");
        serde_json::from_str(line.trim()).unwrap_or_else(|err| {
            panic!("stdout carried something that is not JSON: {err}: {line:?}")
        })
    }

    fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let response = self.receive();
        assert_eq!(
            response["id"], id,
            "answers arrive in order, one per request"
        );
        response
    }

    /// Close the pipe and wait: a server that outlives its client is a
    /// process nobody can see and nobody can stop.
    fn finish(mut self) -> std::process::ExitStatus {
        drop(self.child.stdin.take());
        self.child.wait().unwrap()
    }
}

#[test]
fn the_binary_serves_the_protocol_on_its_own_workspace_and_stops_with_the_pipe() {
    let workspace = scratch("protocol");
    let mut server = Server::start(&workspace);

    let init = server.call(
        1,
        "initialize",
        json!({ "protocolVersion": "2024-11-05", "capabilities": {},
                "clientInfo": { "name": "stdio test", "version": "0" } }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "kilna");
    assert_eq!(
        init["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );

    server.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    let tools = server.call(2, "tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"workspace") && names.contains(&"propose_version"),
        "{names:?}"
    );

    // A fresh workspace is seeded with the shipped profiles, and the tools
    // answer against the active one — the notification above must not have
    // been answered, or this id would not match.
    let workspace_tool = server.call(
        3,
        "tools/call",
        json!({ "name": "workspace", "arguments": {} }),
    );
    let text = workspace_tool["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let card: Value = serde_json::from_str(text).unwrap();
    assert_eq!(card["works"], 0);
    assert!(card["axes"].as_array().is_some_and(|axes| !axes.is_empty()));

    let status = server.finish();
    assert!(
        status.success(),
        "the server should exit cleanly when the client hangs up: {status}"
    );
    assert!(
        workspace.join("kilna.db").exists(),
        "the workspace lives where it was pointed"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

#[test]
fn a_line_that_is_not_json_is_ignored_rather_than_fatal() {
    let workspace = scratch("garbage");
    let mut server = Server::start(&workspace);

    let stdin = server.child.stdin.as_mut().unwrap();
    writeln!(stdin, "this is not a request").unwrap();
    stdin.flush().unwrap();

    let pong = server.call(1, "ping", json!({}));
    assert!(pong["result"].is_object(), "{pong}");

    assert!(server.finish().success());
    let _ = std::fs::remove_dir_all(&workspace);
}
