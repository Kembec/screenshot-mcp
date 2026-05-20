use serde_json::{json, Value};

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "screenshot-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

pub fn ok(id: Value, result: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

pub fn err(id: Value, code: i32, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
    .to_string()
}

pub async fn handle_line(line: &str) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Some(err(Value::Null, PARSE_ERROR, "invalid JSON")),
    };

    let jsonrpc = req.get("jsonrpc").and_then(|v| v.as_str()).unwrap_or("");
    if jsonrpc != "2.0" {
        return Some(err(
            req.get("id").cloned().unwrap_or(Value::Null),
            INVALID_REQUEST,
            "jsonrpc must be \"2.0\"",
        ));
    }

    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");

    match method {
        "initialize" => Some(ok(id, initialize_result())),
        "initialized" | "notifications/initialized" => None,
        "ping" => Some(ok(id, json!({}))),
        "tools/list" => Some(ok(id, crate::tools::tools_list())),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(json!({}));
            let name = params.get("name").and_then(|v| v.as_str());
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            match name {
                None => Some(err(id, INVALID_PARAMS, "missing tool name")),
                Some(n) => match crate::tools::call(n, arguments).await {
                    Ok(result) => Some(ok(id, result)),
                    Err(e) => Some(err(id, INTERNAL_ERROR, &format!("{e}"))),
                },
            }
        }
        other => {
            if id == Value::Null {
                None
            } else {
                Some(err(
                    id,
                    METHOD_NOT_FOUND,
                    &format!("unknown method `{other}`"),
                ))
            }
        }
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
        "capabilities": {"tools": {"listChanged": false}}
    })
}
