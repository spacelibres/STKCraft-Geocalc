//! Stdio JSON-RPC MCP server. All calculations reuse fixed CLI backends.
mod backend;
mod catalog;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
const MAX_MESSAGE: usize = 1_048_576;
const MODERN: &str = "2026-07-28";
const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2024-11-05"];
const VERSIONS: &[&str] = &[MODERN, "2025-11-25", "2025-06-18", "2024-11-05"];
#[derive(Default)]
pub struct Server {
    initialized: bool,
    ready: bool,
}
fn error(id: Value, code: i32, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn identity() -> Value {
    json!({"name":"stkcraft-geocalc","version":env!("CARGO_PKG_VERSION")})
}
fn capabilities() -> Value {
    json!({"tools":{"listChanged":false},"resources":{"subscribe":false,"listChanged":false}})
}
impl Server {
    /// Handle one UTF-8 JSON-RPC message. Notifications return no response.
    pub fn handle_message(&mut self, message: &str) -> Option<String> {
        self.handle(message).map(|v| v.to_string())
    }
    fn handle(&mut self, message: &str) -> Option<Value> {
        if message.len() > MAX_MESSAGE {
            return Some(error(Value::Null, -32600, "message exceeds1MiB"));
        }
        let request: Value = match serde_json::from_str(message) {
            Ok(v) => v,
            Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        let Some(object) = request.as_object() else {
            return Some(error(Value::Null, -32600, "request must be an object"));
        };
        let id = object.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str);
        if request["jsonrpc"] != "2.0" || method.is_none() {
            return Some(error(
                id.unwrap_or(Value::Null),
                -32600,
                "invalid JSON-RPC request",
            ));
        }
        let method = method.unwrap();
        if id.is_none() {
            if method == "notifications/initialized" && self.initialized {
                self.ready = true;
            }
            return None;
        }
        let id = id.unwrap();
        if !(id.is_string() || id.is_i64() || id.is_u64()) {
            return Some(error(
                Value::Null,
                -32600,
                "MCP IDs must be strings or integers",
            ));
        }
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        if !params.is_object() {
            return Some(error(id, -32602, "params must be an object"));
        }
        let version = params
            .get("_meta")
            .and_then(|v| v.get("io.modelcontextprotocol/protocolVersion"))
            .and_then(Value::as_str);
        let modern = version == Some(MODERN);
        if let Some(version) = version {
            if !VERSIONS.contains(&version) {
                let mut e = error(id, -32022, "Unsupported protocol version");
                e["error"]["data"] = json!({"supported":VERSIONS,"requested":version});
                return Some(e);
            }
        }
        if modern {
            let meta = &params["_meta"];
            if !meta["io.modelcontextprotocol/clientCapabilities"].is_object()
                || !meta["io.modelcontextprotocol/clientInfo"]["name"].is_string()
                || !meta["io.modelcontextprotocol/clientInfo"]["version"].is_string()
            {
                return Some(error(
                    id,
                    -32602,
                    "modern requests require clientInfo and clientCapabilities metadata",
                ));
            }
        }
        let result = match method {
            "initialize" => {
                let requested = params.get("protocolVersion").and_then(Value::as_str);
                if requested.is_none()
                    || !params["capabilities"].is_object()
                    || !params["clientInfo"]["name"].is_string()
                    || !params["clientInfo"]["version"].is_string()
                {
                    return Some(error(
                        id,
                        -32602,
                        "initialize requires protocolVersion, capabilities and clientInfo",
                    ));
                }
                if self.initialized {
                    return Some(error(id, -32600, "connection is already initialized"));
                }
                self.initialized = true;
                self.ready = false;
                let version = if LEGACY.contains(&requested.unwrap()) {
                    requested.unwrap()
                } else {
                    LEGACY[0]
                };
                json!({"protocolVersion":version,"capabilities":capabilities(),"serverInfo":identity(),"instructions":"Use tools/list for exact supported arguments. Supply kernels explicitly; calculations run fixed local backends with no CSPICE runtime. Read geocalc://manual for units and scope."})
            }
            "server/discover" => {
                json!({"supportedVersions":VERSIONS,"capabilities":capabilities(),"_meta":{"io.modelcontextprotocol/serverInfo":identity()},"instructions":"Local exact-source geometry and GF tools; explicit kernel paths per call."})
            }
            "ping" => json!({}),
            _ if !modern && !self.ready => {
                return Some(error(
                    id,
                    -32600,
                    "initialize and send notifications/initialized before operations",
                ))
            }
            "tools/list" => {
                if params.get("cursor").is_some() {
                    return Some(error(id, -32602, "pagination cursors are not supported"));
                }
                json!({"tools":catalog::definitions()})
            }
            "tools/call" => {
                let Some(name) = params.get("name").and_then(Value::as_str) else {
                    return Some(error(id, -32602, "tools/call requires name"));
                };
                if name != "calculate_csv"
                    && !catalog::specifications().iter().any(|s| s.name == name)
                {
                    return Some(error(id, -32602, "unknown tool"));
                }
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                if let Err(message) = backend::prepare(name, &args) {
                    return Some(error(id, -32602, &message));
                }
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    backend::call(name, &args)
                })) {
                    Ok(Ok(data)) => {
                        json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":false})
                    }
                    Ok(Err(message)) => {
                        json!({"content":[{"type":"text","text":message}],"isError":true})
                    }
                    Err(_) => {
                        json!({"content":[{"type":"text","text":"calculation adapter panicked"}],"isError":true})
                    }
                }
            }
            "resources/list" => {
                json!({"resources":[{"uri":"geocalc://manual","name":"STKCraft Geocalc MCP manual","mimeType":"text/markdown"}]})
            }
            "resources/templates/list" => json!({"resourceTemplates":[]}),
            "resources/read" => {
                if params["uri"] != "geocalc://manual" {
                    return Some(error(id, -32602, "unknown resource"));
                }
                json!({"contents":[{"uri":"geocalc://manual","mimeType":"text/markdown","text":include_str!("../../docs/MCP.md")} ]})
            }
            _ => return Some(error(id, -32601, "method not found")),
        };
        let mut result = result;
        if modern {
            result["resultType"] = json!("complete");
        }
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}
/// Newline-delimited UTF-8 transport with a bounded per-message reader.
pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut out = stdout.lock();
    let mut server = Server::default();
    loop {
        let mut bytes = Vec::new();
        let mut oversized = false;
        let mut any = false;
        loop {
            let available = input.fill_buf()?;
            if available.is_empty() {
                break;
            }
            any = true;
            let n = available
                .iter()
                .position(|b| *b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(available.len());
            if bytes.len() + n > MAX_MESSAGE + 1 {
                oversized = true;
            } else if !oversized {
                bytes.extend_from_slice(&available[..n]);
            }
            let done = available[n - 1] == b'\n';
            input.consume(n);
            if done {
                break;
            }
        }
        if !any {
            break;
        }
        let response = if oversized {
            Some(error(Value::Null, -32600, "message exceeds1MiB").to_string())
        } else {
            match std::str::from_utf8(&bytes) {
                Ok(line) if line.trim().is_empty() => None,
                Ok(line) => server.handle_message(line.trim_end_matches(['\r', '\n'])),
                Err(_) => Some(error(Value::Null, -32700, "message is not UTF-8").to_string()),
            }
        };
        if let Some(response) = response {
            writeln!(out, "{response}")?;
            out.flush()?;
        }
    }
    Ok(())
}
