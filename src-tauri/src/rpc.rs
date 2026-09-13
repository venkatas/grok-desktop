use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Response {
        id: Value,
        result: Option<Value>,
        error: Option<Value>,
    },
    Notification {
        method: String,
        params: Value,
    },
    Request {
        id: Value,
        method: String,
        params: Value,
    },
}

pub fn parse_line(line: &str) -> Result<Incoming, String> {
    let v: Value = serde_json::from_str(line.trim()).map_err(|e| e.to_string())?;
    let method = v.get("method").and_then(|m| m.as_str()).map(|s| s.to_string());
    let id = v.get("id").cloned();
    if let Some(method) = method {
        let params = v.get("params").cloned().unwrap_or(json!({}));
        if let Some(id) = id {
            return Ok(Incoming::Request { id, method, params });
        }
        return Ok(Incoming::Notification { method, params });
    }
    if let Some(id) = id {
        return Ok(Incoming::Response {
            id,
            result: v.get("result").cloned(),
            error: v.get("error").cloned(),
        });
    }
    Err("not a json-rpc message".into())
}

pub fn request_line(id: u64, method: &str, params: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
    .to_string()
        + "\n"
}

pub fn notify_line(method: &str, params: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
    .to_string()
        + "\n"
}

pub fn response_line(id: &Value, result: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
    .to_string()
        + "\n"
}

pub fn error_line(id: &Value, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32000, "message": message }
    })
    .to_string()
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_request_notification_response() {
        let req = parse_line(r#"{"jsonrpc":"2.0","id":5,"method":"session/request_permission","params":{}}"#)
            .unwrap();
        match req {
            Incoming::Request { method, .. } => assert_eq!(method, "session/request_permission"),
            _ => panic!("expected request"),
        }
        let n = parse_line(r#"{"jsonrpc":"2.0","method":"session/update","params":{"x":1}}"#).unwrap();
        match n {
            Incoming::Notification { method, .. } => assert_eq!(method, "session/update"),
            _ => panic!("expected notification"),
        }
        let r = parse_line(r#"{"jsonrpc":"2.0","id":1,"result":{"sessionId":"s"}}"#).unwrap();
        match r {
            Incoming::Response { result, .. } => {
                assert_eq!(result.unwrap()["sessionId"], "s");
            }
            _ => panic!("expected response"),
        }
    }
}
