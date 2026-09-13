use crate::permission::{selected_result, unanswered_deny_result};
use crate::rpc::{notify_line, parse_line, request_line, response_line, Incoming};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub enum HostEvent {
    Notification { method: String, params: Value },
    Request { id: Value, method: String, params: Value },
    Exit,
}

pub type EventFn = Arc<dyn Fn(HostEvent) + Send + Sync>;

struct Pending {
    tx: Sender<Result<Value, String>>,
}

pub struct AgentHost {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    next_id: Mutex<u64>,
    pending: Arc<Mutex<HashMap<u64, Pending>>>,
    child: Mutex<Option<Child>>,
}

pub fn agent_stdio_args() -> &'static [&'static str] {
    // --no-leader is an option of `grok agent`, not of `stdio`.
    &["agent", "--no-leader", "stdio"]
}

impl AgentHost {
    pub fn spawn(bin: &Path, on_event: EventFn) -> Result<Self, String> {
        let mut child = Command::new(bin)
            .args(agent_stdio_args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to start grok: {e}"))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let stderr = child.stderr.take();
        let stderr_buf = Arc::new(Mutex::new(String::new()));
        if let Some(err) = stderr {
            let buf = stderr_buf.clone();
            thread::spawn(move || {
                let reader = BufReader::new(err);
                for line in reader.lines().flatten() {
                    if let Ok(mut s) = buf.lock() {
                        if s.len() < 16_384 {
                            s.push_str(&line);
                            s.push('\n');
                        }
                    }
                }
            });
        }
        Self::from_rw(
            Box::new(stdin),
            BufReader::new(stdout),
            Some(child),
            on_event,
            stderr_buf,
        )
    }

    pub fn from_rw<R: BufRead + Send + 'static>(
        writer: Box<dyn Write + Send>,
        reader: R,
        child: Option<Child>,
        on_event: EventFn,
        stderr_buf: Arc<Mutex<String>>,
    ) -> Result<Self, String> {
        let pending: Arc<Mutex<HashMap<u64, Pending>>> = Arc::new(Mutex::new(HashMap::new()));
        let pending_r = pending.clone();
        thread::spawn(move || read_loop(reader, pending_r, on_event, stderr_buf));
        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            next_id: Mutex::new(1),
            pending,
            child: Mutex::new(child),
        })
    }

    fn write_raw(&self, line: &str) -> Result<(), String> {
        let mut w = self.writer.lock().map_err(|e| e.to_string())?;
        w.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())
    }

    fn request(&self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let id = {
            let mut n = self.next_id.lock().map_err(|e| e.to_string())?;
            let id = *n;
            *n += 1;
            id
        };
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id, Pending { tx });
        self.write_raw(&request_line(id, method, params))?;
        rx.recv_timeout(timeout)
            .map_err(|_| format!("timeout waiting for {method}"))?
    }

    pub fn initialize(&self) -> Result<Value, String> {
        self.request(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {
                    "fs": { "readTextFile": true, "writeTextFile": false },
                    "terminal": true
                },
                "clientInfo": {
                    "name": "grok-build",
                    "title": "Grok Build",
                    "version": "0.1.0"
                }
            }),
            Duration::from_secs(15),
        )
    }

    pub fn session_new(&self, cwd: &str) -> Result<Value, String> {
        let result = self.request(
            "session/new",
            json!({ "cwd": cwd, "mcpServers": [] }),
            Duration::from_secs(30),
        )?;
        if result.get("sessionId").and_then(|s| s.as_str()).is_none() {
            return Err("session/new missing sessionId".into());
        }
        Ok(result)
    }

    pub fn session_load(&self, cwd: &str, id: &str) -> Result<Value, String> {
        self.request(
            "session/load",
            json!({ "cwd": cwd, "sessionId": id }),
            Duration::from_secs(30),
        )
    }

    pub fn session_prompt(&self, session_id: &str, text: &str) -> Result<Value, String> {
        self.request(
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": [{ "type": "text", "text": text }]
            }),
            Duration::from_secs(600),
        )
    }

    pub fn session_cancel(&self, session_id: &str) -> Result<(), String> {
        self.write_raw(&notify_line(
            "session/cancel",
            json!({ "sessionId": session_id }),
        ))
    }

    pub fn set_config_option(
        &self,
        session_id: &str,
        config_id: &str,
        value: &str,
    ) -> Result<Value, String> {
        self.request(
            "session/set_config_option",
            json!({
                "sessionId": session_id,
                "configId": config_id,
                "value": { "value": value }
            }),
            Duration::from_secs(15),
        )
    }

    pub fn git_diffs(&self) -> Result<Value, String> {
        self.request("x.ai/git/diffs", json!({}), Duration::from_secs(15))
    }

    pub fn create_worktree(&self, cwd: &str) -> Result<Value, String> {
        self.request(
            "x.ai/git/worktree/create",
            json!({ "cwd": cwd }),
            Duration::from_secs(60),
        )
    }

    pub fn respond_permission(&self, rpc_id: &Value, option_id: Option<&str>) -> Result<(), String> {
        let result = match option_id {
            Some(id) => selected_result(id),
            None => unanswered_deny_result(),
        };
        self.write_raw(&response_line(rpc_id, result))
    }

    pub fn respond_fs_read(&self, rpc_id: &Value, path: &Path) -> Result<(), String> {
        match std::fs::read_to_string(path) {
            Ok(content) => self.write_raw(&response_line(rpc_id, json!({ "content": content }))),
            Err(e) => self.write_raw(&crate::rpc::error_line(rpc_id, &e.to_string())),
        }
    }

    pub fn respond_error(&self, rpc_id: &Value, message: &str) -> Result<(), String> {
        self.write_raw(&crate::rpc::error_line(rpc_id, message))
    }

    pub fn kill(&self) {
        if let Ok(mut child) = self.child.lock() {
            if let Some(c) = child.as_mut() {
                let _ = c.kill();
                let _ = c.wait();
            }
            *child = None;
        }
    }
}

fn id_as_u64(id: &Value) -> Option<u64> {
    id.as_u64()
        .or_else(|| id.as_i64().map(|i| i as u64))
        .or_else(|| id.as_str().and_then(|s| s.parse().ok()))
}

fn read_loop<R: BufRead>(
    reader: R,
    pending: Arc<Mutex<HashMap<u64, Pending>>>,
    events: EventFn,
    stderr_buf: Arc<Mutex<String>>,
) {
    let mut lines = reader.lines();
    while let Some(Ok(line)) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(&line) {
            Ok(Incoming::Response { id, result, error }) => {
                if let Some(n) = id_as_u64(&id) {
                    if let Ok(mut map) = pending.lock() {
                        if let Some(p) = map.remove(&n) {
                            let msg = if let Some(err) = error {
                                Err(err.to_string())
                            } else {
                                Ok(result.unwrap_or(Value::Null))
                            };
                            let _ = p.tx.send(msg);
                        }
                    }
                }
            }
            Ok(Incoming::Notification { method, params }) => {
                events(HostEvent::Notification { method, params });
            }
            Ok(Incoming::Request { id, method, params }) => {
                events(HostEvent::Request { id, method, params });
            }
            Err(_) => {}
        }
    }
    let detail = stderr_buf
        .lock()
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| format!("agent process exited: {s}"))
        .unwrap_or_else(|| "agent process exited".to_string());
    if let Ok(mut map) = pending.lock() {
        for (_, p) in map.drain() {
            let _ = p.tx.send(Err(detail.clone()));
        }
    }
    events(HostEvent::Exit);
}

pub fn normalize_diffs(value: &Value) -> Vec<Value> {
    if let Some(files) = value.get("files").and_then(|f| f.as_array()) {
        return files.clone();
    }
    if let Some(diffs) = value.get("diffs").and_then(|f| f.as_array()) {
        return diffs.clone();
    }
    if let Some(arr) = value.as_array() {
        return arr.clone();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;

    fn run_mock(stream: UnixStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut line = String::new();
        let mut prompt_id: Option<Value> = None;
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let v: Value = match serde_json::from_str(line.trim()) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let method = v.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = v.get("id").cloned();
            let params = v.get("params").cloned().unwrap_or(json!({}));
            match method {
                "initialize" => {
                    let _ = write!(
                        writer,
                        "{}",
                        response_line(
                            &id.unwrap(),
                            json!({
                                "protocolVersion": 1,
                                "agentCapabilities": { "loadSession": true }
                            })
                        )
                    );
                }
                "session/new" => {
                    let _ = write!(
                        writer,
                        "{}",
                        response_line(&id.unwrap(), json!({ "sessionId": "sess-1" }))
                    );
                }
                "session/load" => {
                    let sid = params.get("sessionId").cloned().unwrap_or(json!("sess-1"));
                    let _ = write!(
                        writer,
                        "{}",
                        response_line(&id.unwrap(), json!({ "sessionId": sid }))
                    );
                }
                "session/prompt" => {
                    prompt_id = id.clone();
                    let text = params
                        .pointer("/prompt/0/text")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    if text == "DIE" {
                        break;
                    }
                    if text == "NEED_PERM" {
                        let _ = write!(
                            writer,
                            "{}\n",
                            json!({
                                "jsonrpc": "2.0",
                                "id": 99,
                                "method": "session/request_permission",
                                "params": {
                                    "sessionId": "sess-1",
                                    "toolCall": { "toolCallId": "call_1", "title": "write a.rs" },
                                    "options": [
                                        { "optionId": "allow-once", "name": "Allow once", "kind": "allow_once" },
                                        { "optionId": "reject-once", "name": "Reject", "kind": "reject_once" }
                                    ]
                                }
                            })
                        );
                        continue;
                    }
                    let _ = write!(
                        writer,
                        "{}",
                        crate::rpc::notify_line(
                            "session/update",
                            json!({
                                "sessionId": "sess-1",
                                "update": {
                                    "sessionUpdate": "agent_message_chunk",
                                    "content": { "type": "text", "text": "hello" }
                                }
                            })
                        )
                    );
                    let _ = write!(
                        writer,
                        "{}",
                        response_line(&id.unwrap(), json!({ "stopReason": "end_turn" }))
                    );
                }
                "session/cancel" => {
                    if let Some(pid) = prompt_id.take() {
                        let _ = write!(
                            writer,
                            "{}",
                            response_line(&pid, json!({ "stopReason": "cancelled" }))
                        );
                    }
                }
                "x.ai/git/diffs" => {
                    let _ = write!(
                        writer,
                        "{}",
                        response_line(
                            &id.unwrap(),
                            json!({
                                "files": [{ "path": "a.rs", "added": 1, "removed": 0, "patch": "+x" }]
                            })
                        )
                    );
                }
                _ => {
                    if v.get("result").is_some() {
                        // permission response from client
                        if let Some(pid) = prompt_id.take() {
                            let cancelled = v
                                .pointer("/result/outcome/outcome")
                                .and_then(|o| o.as_str())
                                == Some("cancelled");
                            let reason = if cancelled { "cancelled" } else { "end_turn" };
                            let _ = write!(
                                writer,
                                "{}",
                                response_line(&pid, json!({ "stopReason": reason }))
                            );
                        }
                    }
                }
            }
            let _ = writer.flush();
        }
    }

    struct Pair {
        host: AgentHost,
        events: mpsc::Receiver<HostEvent>,
    }

    fn with_host() -> Pair {
        let (a, b) = UnixStream::pair().unwrap();
        thread::spawn(move || run_mock(b));
        let reader = BufReader::new(a.try_clone().unwrap());
        let (tx, rx) = mpsc::channel();
        let host = AgentHost::from_rw(
            Box::new(a),
            reader,
            None,
            Arc::new(move |e| {
                let _ = tx.send(e);
            }),
            Arc::new(Mutex::new(String::new())),
        )
        .unwrap();
        Pair { host, events: rx }
    }

    fn recv_timeout(rx: &mpsc::Receiver<HostEvent>, dur: Duration) -> Option<HostEvent> {
        rx.recv_timeout(dur).ok()
    }

    #[test]
    fn stdio_args_put_no_leader_before_subcommand() {
        assert_eq!(agent_stdio_args(), ["agent", "--no-leader", "stdio"]);
    }

    #[test]
    fn initialize_and_new_session() {
        let p = with_host();
        let init = p.host.initialize().unwrap();
        assert_eq!(init["protocolVersion"], 1);
        let created = p.host.session_new("/tmp/proj").unwrap();
        assert_eq!(created["sessionId"], "sess-1");
    }

    #[test]
    fn prompt_streams_then_completes() {
        let p = with_host();
        p.host.initialize().unwrap();
        p.host.session_new("/tmp/proj").unwrap();
        let done = thread::scope(|s| {
            let result = s.spawn(|| p.host.session_prompt("sess-1", "hi"));
            let mut saw = false;
            for _ in 0..20 {
                if let Some(HostEvent::Notification { method, params }) =
                    recv_timeout(&p.events, Duration::from_millis(200))
                {
                    if method == "session/update"
                        && params.pointer("/update/content/text").and_then(|t| t.as_str())
                            == Some("hello")
                    {
                        saw = true;
                    }
                }
            }
            let stop = result.join().unwrap().unwrap();
            (saw, stop)
        });
        assert!(done.0);
        assert_eq!(done.1["stopReason"], "end_turn");
    }

    #[test]
    fn permission_deny_cancels() {
        let p = with_host();
        p.host.initialize().unwrap();
        p.host.session_new("/tmp/proj").unwrap();
        let stop = thread::scope(|s| {
            let result = s.spawn(|| p.host.session_prompt("sess-1", "NEED_PERM"));
            let mut rpc_id = None;
            for _ in 0..20 {
                if let Some(HostEvent::Request { id, method, .. }) =
                    recv_timeout(&p.events, Duration::from_millis(200))
                {
                    if method == "session/request_permission" {
                        rpc_id = Some(id);
                        break;
                    }
                }
            }
            p.host.respond_permission(&rpc_id.unwrap(), None).unwrap();
            result.join().unwrap().unwrap()
        });
        assert_eq!(stop["stopReason"], "cancelled");
    }

    #[test]
    fn permission_allow_completes() {
        let p = with_host();
        p.host.initialize().unwrap();
        p.host.session_new("/tmp/proj").unwrap();
        let stop = thread::scope(|s| {
            let result = s.spawn(|| p.host.session_prompt("sess-1", "NEED_PERM"));
            let mut rpc_id = None;
            for _ in 0..20 {
                if let Some(HostEvent::Request { id, method, .. }) =
                    recv_timeout(&p.events, Duration::from_millis(200))
                {
                    if method == "session/request_permission" {
                        rpc_id = Some(id);
                        break;
                    }
                }
            }
            p.host
                .respond_permission(&rpc_id.unwrap(), Some("allow-once"))
                .unwrap();
            result.join().unwrap().unwrap()
        });
        assert_eq!(stop["stopReason"], "end_turn");
    }

    #[test]
    fn session_load_and_diffs() {
        let p = with_host();
        p.host.initialize().unwrap();
        let loaded = p.host.session_load("/tmp/proj", "sess-9").unwrap();
        assert_eq!(loaded["sessionId"], "sess-9");
        let diffs = p.host.git_diffs().unwrap();
        assert_eq!(normalize_diffs(&diffs)[0]["path"], "a.rs");
    }

    #[test]
    fn child_exit_emits_event() {
        let p = with_host();
        p.host.initialize().unwrap();
        p.host.session_new("/tmp/proj").unwrap();
        thread::scope(|s| {
            s.spawn(|| {
                let _ = p.host.session_prompt("sess-1", "DIE");
            });
            let mut saw_exit = false;
            for _ in 0..20 {
                if matches!(
                    recv_timeout(&p.events, Duration::from_millis(200)),
                    Some(HostEvent::Exit)
                ) {
                    saw_exit = true;
                    break;
                }
            }
            assert!(saw_exit);
        });
    }
}
