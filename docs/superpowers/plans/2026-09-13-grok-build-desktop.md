# Grok Build Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a macOS Tauri app named Grok Build that wraps `grok agent stdio --no-leader` over ACP, with setup/home/workspace screens, one live session, fail-closed permissions, and view-only diffs.

**Architecture:** React UI talks only to a Rust host. The host owns the grok child, JSON-RPC ACP framing, permission ids, session index, and recents. A mock stdio agent drives integration tests without a live Grok account.

**Tech Stack:** Tauri 2, Rust, React 18, TypeScript, Vite, Vitest, Testing Library, `serde_json` JSON-RPC (no private ACP dialect). Apple Silicon, macOS 14+.

**Spec:** `docs/superpowers/specs/2026-09-13-grok-build-desktop-design.md`

## Global Constraints

- Display name Grok Build, bundle id `ai.x.grok.build`
- Agent spawn: `<grok> agent stdio --no-leader` (never `--always-approve`, never `_meta.yoloMode`)
- `$GROK_HOME` when set, else `~/.grok`
- Binary lookup: `GROK_BIN` executable, then `$GROK_HOME/bin/grok`, then `grok` on PATH
- One window, one live session
- Unanswered permission is Deny (`outcome: cancelled`)
- Diff pane is view-only
- Recents: `~/Library/Application Support/Grok Build/state.json`, cap 20
- Dark near-black UI, white text, native traffic lights
- Do not bundle a grok binary
- `fs.writeTextFile` capability is false; `fs.readTextFile` true; `terminal` true

## File map

| File | Responsibility |
|------|----------------|
| `src-tauri/src/paths.rs` | `grok_home()`, app support dir |
| `src-tauri/src/grok_bin.rs` | Resolve grok executable |
| `src-tauri/src/auth.rs` | `auth.json` readable; spawn `grok login` |
| `src-tauri/src/sessions.rs` | Encode cwd, list `summary.json`, read `updates.jsonl` |
| `src-tauri/src/recents.rs` | Load/save recents cap 20 |
| `src-tauri/src/permission.rs` | Label map, ensure Deny, unanswered = cancelled |
| `src-tauri/src/rpc.rs` | JSON-RPC framing over a `BufRead`/`Write` pair |
| `src-tauri/src/agent.rs` | Spawn child, initialize, session/new/load/prompt/cancel, permission, diffs |
| `src-tauri/src/lib.rs` | Tauri commands and events |
| `src-tauri/tests/mock_agent.rs` | Stdio mock agent + process tests |
| `src/screens/*.tsx` `src/panes/*.tsx` `src/components/*.tsx` | UI |
| `src/lib/types.ts` `src/lib/host.ts` | Shared TS types and Tauri wrappers |

## Shared types (lock these names)

```rust
// paths.rs
pub fn grok_home() -> PathBuf;

// grok_bin.rs
pub fn resolve_grok_bin() -> Option<PathBuf>;
pub fn resolve_grok_bin_in(env_bin: Option<PathBuf>, grok_home: &Path, path_dirs: &[PathBuf]) -> Option<PathBuf>;

// auth.rs
pub fn auth_json_path() -> PathBuf;
pub fn is_signed_in() -> bool; // file exists and is readable; do not parse secrets into UI

// sessions.rs
pub fn encode_cwd(cwd: &Path) -> String; // percent-encode the full path, '/' -> "%2F"
pub fn session_group_dir(cwd: &Path) -> PathBuf;
pub struct SessionRow { pub id: String, pub title: String, pub updated_at: String, pub model: Option<String> }
pub fn list_sessions(cwd: &Path) -> Vec<SessionRow>;

// recents.rs
pub struct RecentsState { pub folders: Vec<String>, pub window_width: Option<f64>, pub window_height: Option<f64> }
pub fn load_recents(path: &Path) -> RecentsState;
pub fn push_recent(state: RecentsState, folder: String) -> RecentsState; // front, unique, cap 20

// permission.rs
pub struct PermissionOption { pub option_id: String, pub name: String, pub kind: String }
pub fn display_label(kind: &str, name: &str) -> String; // allow_once -> Allow once, allow_always -> Allow session, else name
pub fn unanswered_deny_result() -> serde_json::Value; // {"outcome":{"outcome":"cancelled"}}

// agent.rs
pub struct AgentHost { /* child, pending map, next id */ }
impl AgentHost {
    pub fn spawn(bin: &Path) -> Result<Self, String>;
    pub fn initialize(&mut self) -> Result<serde_json::Value, String>;
    pub fn session_new(&mut self, cwd: &str) -> Result<String, String>;
    pub fn session_load(&mut self, cwd: &str, id: &str) -> Result<serde_json::Value, String>;
    pub fn session_prompt(&mut self, session_id: &str, text: &str) -> Result<(), String>;
    pub fn session_cancel(&mut self, session_id: &str) -> Result<(), String>;
    pub fn respond_permission(&mut self, rpc_id: serde_json::Value, option_id: Option<String>) -> Result<(), String>;
    pub fn git_diffs(&mut self) -> Result<serde_json::Value, String>;
    pub fn kill(&mut self);
}
```

Tauri events: `agent://update`, `agent://permission`, `agent://error`, `agent://exit`, `agent://prompt-done`.

Tauri commands: `app_status`, `sign_in`, `list_recents`, `open_folder`, `list_sessions`, `session_new`, `session_load`, `session_prompt`, `session_cancel`, `permission_respond`, `git_diffs`, `set_config_option`, `retry_agent`.

---

### Task 1: Scaffold Tauri 2 + React TS

**Files:** create `package.json`, `src-tauri/`, `src/`, Vite, keep existing spec/README.

- [ ] **Step 1:** Create the app in the repo with `create-tauri-app` react-ts, identifier `ai.x.grok.build`, npm, Tauri 2, force into the existing directory. Keep `docs/` and `.gitignore` entries for `.superpowers/`, `node_modules/`, `dist/`, `src-tauri/target/`.
- [ ] **Step 2:** Set productName `Grok Build`, identifier `ai.x.grok.build`, windows: one, macOS minimum 14. Native decorations on. CSP default.
- [ ] **Step 3:** `npm install` and `cargo check` in `src-tauri`. Confirm they succeed.
- [ ] **Step 4:** Commit `chore: scaffold Tauri 2 React app`.

### Task 2: paths + grok_bin

**Test:** `src-tauri/src/grok_bin.rs` unit tests (or `src-tauri/tests/grok_bin.rs`).

- [ ] **Step 1:** Write failing tests: `GROK_BIN` wins if executable; skipped if not executable; `$GROK_HOME/bin/grok` next; PATH last; missing returns None.
- [ ] **Step 2:** Run tests, expect fail (module missing).
- [ ] **Step 3:** Implement `grok_home` and `resolve_grok_bin_in`.
- [ ] **Step 4:** Tests pass. Commit `feat: resolve grok binary and GROK_HOME`.

### Task 3: sessions index

- [ ] **Step 1:** Fixture tree with encoded cwd group plus a `.cwd` fallback group. Tests: encode `/Users/a/b` == `%2FUsers%2Fa%2Fb`; list reads `generated_title`/`session_summary` and `info.id`; long-path `.cwd` match included.
- [ ] **Step 2:** Fail, then implement `encode_cwd` + `list_sessions`.
- [ ] **Step 3:** Commit `feat: index grok sessions by encoded cwd`.

### Task 4: recents + auth detect + permission helper

- [ ] **Step 1:** Tests: push unique to front, cap 20, load missing file as empty; `is_signed_in` true only when file readable; `unanswered_deny_result` is cancelled; `display_label("allow_always", "x")` == `Allow session`.
- [ ] **Step 2:** Implement. Commit `feat: recents, auth detect, permission deny default`.

### Task 5: JSON-RPC + mock ACP agent

**Test:** `src-tauri/tests/mock_agent.rs` using a thread or subprocess that speaks newline JSON-RPC.

Mock behavior:
- `initialize` -> protocolVersion 1, loadSession true
- `session/new` -> `{ sessionId: "sess-1" }`
- `session/load` -> `{ sessionId }`
- `session/prompt` -> notify `agent_message_chunk` "hello", then result `{ stopReason: "end_turn" }`
- `session/request_permission` (on prompt text `NEED_PERM`) -> wait for response; if cancelled, stopReason cancelled
- `x.ai/git/diffs` -> `{ files: [{ path: "a.rs", added: 1, removed: 0, patch: "+x" }] }`
- `session/cancel` -> complete prompt with cancelled
- special prompt `DIE` -> exit process

- [ ] **Step 1:** Failing tests for initialize, new, streamed update, permission allow, permission deny, load, child exit mid-turn.
- [ ] **Step 2:** Implement `rpc.rs` + `agent.rs` against the mock.
- [ ] **Step 3:** Tests pass. Commit `feat: ACP host with mock agent tests`.

### Task 6: Tauri commands and events

- [ ] **Step 1:** Wire `app_status` `{ grok_bin: Option<String>, signed_in: bool }`. `open_folder` uses native dialog, pushes recents. `session_new` spawns agent if needed. Permission requests emit `agent://permission`. Unanswered on window close calls `unanswered_deny_result`.
- [ ] **Step 2:** Handle inbound `fs/read_text_file` from agent: read file, return text. Reject `fs/write_text_file`.
- [ ] **Step 3:** Notifications when unfocused: "Grok needs approval", "Grok finished".
- [ ] **Step 4:** Commit `feat: tauri commands for status, sessions, agent`.

### Task 7: React UI

Screens: Setup, Home, Workspace. Panes: SessionList, Chat, Diffs. Components: ToolCard, PermissionCard, Composer.

- [ ] **Step 1:** Vitest tests for session rows, tool card statuses, permission buttons (agent options + Deny), diff empty/error/list, composer send/stop.
- [ ] **Step 2:** Implement dark three-pane workspace per spec anatomy.
- [ ] **Step 3:** Tests pass. Commit `feat: desktop UI for setup, home, workspace`.

### Task 8: README + build

- [ ] **Step 1:** README: how to `npm install`, `npm run tauri dev`, `npm run tauri build`. Note: do not drive the same live session from TUI and the app.
- [ ] **Step 2:** `npm run tauri build` produces `Grok Build.app` for arm64.
- [ ] **Step 3:** Commit `docs: usage and local build`.

## Spec coverage

Setup, home, workspace, binary lookup, auth, sessions, recents, ACP spawn, prompt stream, permission fail-closed, diffs view-only, worktree toggle (new session via `x.ai/git/worktree/create` or disable), model/effort configOptions, errors (child die, load fail, diff fail), mock tests, manual checklist in README. Not in this plan: parallel agents, leader, notarization.
