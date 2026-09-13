# Grok Build Desktop Design

**Date:** 2026-09-13
**Status:** Draft pending user review
**Repo:** `/Users/venkatasatish/Documents/GitHub/grok-desktop`

## Goal

Ship a macOS app that is a Codex-style coding command center for Grok. The user opens a folder, talks to one live Grok agent, watches tool calls, approves writes, and reviews git diffs. The app does not reimplement the agent. It starts the installed `grok` binary in agent mode and speaks ACP.

This is V1 of **Grok Build**, the desktop shell around Grok Build TUI.

## Non-goals (V1)

- Parallel agents and a multi-thread board
- A Cursor-class editor (Grok Bot.app already exists)
- General grok.com chat, Cowork, computer use, in-app browser, voice
- Stage, discard, or commit from the diff pane
- Always-approve / yolo as a default
- Sharing one live session with the TUI through the Grok leader process
- Notarized distribution, Intel Macs, Windows, Linux
- Image attachments, slash-command palette, MCP installer UI

## Product

| Item | Value |
|------|--------|
| Display name | Grok Build |
| Bundle id | `ai.x.grok.build` |
| Platforms | macOS 14+, Apple Silicon |
| Stack | Tauri 2, Rust host, React + TypeScript UI |
| Agent | `grok agent stdio --no-leader` |
| Auth | Existing `$GROK_HOME/auth.json` (default `~/.grok/auth.json`) / `grok login` |
| Sessions | Existing `$GROK_HOME/sessions/` |
| Recents | `~/Library/Application Support/Grok Build/state.json` |
| Windows | One window. V1 does not open a second workspace window. |

V1 screens: Setup (binary or auth missing), Home (recent folders + Open Folder), Workspace (three panes). All `$GROK_HOME` paths use the env var when set, otherwise `~/.grok`.

## Architecture

```
Grok Build.app (Tauri 2)
  React UI  <-->  Rust host  <-->  grok agent stdio --no-leader
                                      |
                                      +-- ~/.grok/auth.json
                                      +-- ~/.grok/sessions/
                                      +-- ~/.grok/config.toml
```

The UI never talks to the child process. The Rust host owns stdio, JSON-RPC ids, and the permission pending map. The UI calls Tauri commands and listens to Tauri events.

### Host modules

| Module | Does | Depends on |
|--------|------|------------|
| `grok_bin` | Resolve the `grok` executable | env, homedir, PATH |
| `auth` | Detect signed-in state; start login | `grok_bin`, `~/.grok/auth.json` |
| `agent` | Spawn child, ACP initialize, sessions, prompts, cancel, permissions | `grok_bin` |
| `sessions` | List/resume metadata from `$GROK_HOME/sessions/<encoded-cwd>/` | filesystem |
| `diffs` | Request `x.ai/git/diffs` through `agent` | `agent` |
| `recents` | Read/write recent folders | app support dir |

Each module has a small public API. The UI can be swapped without changing ACP framing. The mock agent in tests can replace the real child without changing the UI.

### Binary lookup

1. `GROK_BIN` if it points to an executable
2. `~/.grok/bin/grok` (honor `GROK_HOME` when set)
3. `grok` on `PATH`

The app does not bundle a Grok binary.

### ACP client capabilities

On `initialize`, the host advertises:

- `fs.readTextFile`: true (so the agent can ask the client to read files)
- `fs.writeTextFile`: false (writes go through Grok tools, then permission prompts)
- `terminal`: true

The host does not pass `_meta.yoloMode` or `--always-approve`.

## UI

Dark near-black chrome, white text, native traffic lights. No cream Claude theme. No ChatGPT green.

### Title bar (workspace)

- App name + current folder name
- Model picker from the session `configOptions` entry with `configId = model`
- Effort picker from `configId = reasoning_effort` when present; hidden when the model does not advertise it
- Worktree toggle: **new sessions only**. When on, the host calls `x.ai/git/worktree/create`, then `session/new` with that path as `cwd`. If the method is missing, the toggle is disabled.
- Stop: enabled while a prompt turn is running; sends `session/cancel`

### Left pane: sessions

- `+ New session`
- Search by title
- Rows from `summary.json` for the open folder: title, state (Working / Needs input / Idle / Done / Failed), relative time
- Click a row: if a turn is running, cancel it; then `session/load` that id. One live session in the UI.
- Footer: full folder path + Change folder (native picker)

State for the live session comes from ACP. State for other rows comes from `summary.json` plus mtime. V1 does not poll other processes' live dashboards.

### Center pane: chat

- User text
- Agent markdown
- Thought stream in a collapsed block
- Tool cards: title, path or target, status (running / done / error). Expand shows input/output already present on the ACP event. V1 does not fetch extra logs.
- Plan updates as a checklist when `sessionUpdate = plan`
- Permission cards (see Permissions)
- Composer: textarea, Send, Stop. Text only. Raw text is sent as one ACP text block. A line that starts with `/` is still a prompt, not a command palette.

While streaming, the view sticks to the bottom unless the user has scrolled up.

### Right pane: diffs

- File list with add/delete counts from `x.ai/git/diffs`
- Selected file: unified diff, read-only
- Empty: "No local changes"
- Fetch error: "Could not load diffs" (chat still works)
- No stage, discard, or commit in V1

### Home

Recent folders (path, last opened). Open Folder. Clicking a recent opens the workspace and lists sessions for that path.

### Setup

- Missing binary: show `curl -fsSL https://x.ai/cli/install.sh | bash` and a Recheck button
- Missing or unreadable `auth.json`: Sign in button

## Data flow

### Launch

1. Resolve `grok` binary
2. Check `auth.json` exists and is a readable file (do not display secrets)
3. Setup if either check fails, else Home

### Sign in

1. Run `grok login` as a child of the app (Grok opens the system browser)
2. Wait until `auth.json` exists or the child exits non-zero
3. If `grok login` cannot run, spawn the agent and use `x.ai/auth/get_url` plus `x.ai/auth/submit_code`
4. Never copy tokens into `state.json`

### Open folder

Native directory picker. Persist the path at the front of recents (cap 20). Encode the path the same way Grok does for `~/.grok/sessions/<encoded-cwd>/` and list `summary.json` files one level down. If the encoded name would exceed 255 bytes, also look for group dirs that contain a `.cwd` file matching the path (Grok's documented fallback).

### Agent process

Spawn on first New or Resume in this window:

```
<grok> agent stdio --no-leader
```

One child for the single window. On `initialize`, send the protocol version the agent expects in the handshake (do not invent a private dialect). Then `session/new` or `session/load`. Kill the child on quit after `session/cancel` if a turn is running.

### New session

`session/new` with `{ cwd, mcpServers: [] }` and no yolo meta. Store `sessionId`. Apply model/effort from the title bar with `session/set_config_option` when those options exist.

### Prompt

`session/prompt` with `{ sessionId, prompt: [{ type: "text", text }] }`.

Map `session/update.sessionUpdate`:

| Value | UI |
|-------|----|
| `agent_message_chunk` | Append agent markdown |
| `agent_thought_chunk` | Append to collapsed thoughts |
| `tool_call` | New tool card |
| `tool_call_update` | Patch that card; refresh diffs after a completed write |
| `plan` | Replace plan checklist |

### Permissions

The agent sends a JSON-RPC **request** (not a notification) for tool approval. Use the ACP method name the agent advertises (commonly `session/request_permission`). The host holds the request id.

UI rules:

- Render every option the agent sent as a button
- Always offer Deny, which rejects the request
- When option kinds map cleanly, label them Allow once / Allow session / Deny
- Do not add Allow session unless the agent offered an equivalent option
- Closing the window, killing the child, or leaving the session without an answer **rejects** the request
- There is no timeout that auto-allows

Unfocused window: macOS notification "Grok needs approval". Completing a turn while unfocused: notification "Grok finished".

### Diffs

After a tool card reaches done, and on `x.ai/session_notification` that mentions review/diff, call `x.ai/git/diffs` for the live session cwd. If the method is missing, the pane stays on "Could not load diffs".

### Switch session

1. If a turn is running, `session/cancel` and wait for that RPC to finish or the child to error
2. `session/load` the chosen id with the folder cwd
3. Rebuild the transcript from the load result. If the load result has no history, read `updates.jsonl` for that session id and render it read-only until the next prompt
4. Refresh diffs for the loaded cwd

### Quit

Cancel in-flight turn if any, then kill the child. Session files stay on disk. Recents and last window size go to `state.json`.

## Error handling

| Case | Behavior |
|------|----------|
| No `grok` binary | Setup only. Workspace does not open. |
| Auth missing or expired mid-session | Freeze composer, banner, Sign in. After success, `session/load` the current id. |
| Child exits | Banner + Retry. Retry respawns the child and `session/load`s. Chat keeps the partial stream. |
| ACP error on prompt | Error card in the thread. Composer unlocks. No auto-retry. |
| Permission with no answer | Deny. |
| Diff RPC fails | Right pane error string. Chat continues. |
| `session/load` fails | Keep the previous live session if the child is still up. Else empty composer and mark the row failed. |
| Folder unreadable | Do not open. Offer Remove from recents. |
| TUI and app on the same session | No file lock in V1. Document: do not drive one live session from both. |

## Testing

Must stay green:

1. **Rust unit:** binary lookup order; `GROK_HOME`; session index from a fixture tree (`summary.json`, long-path `.cwd` groups); recents cap 20; permission default-deny helper
2. **Mock ACP child:** a small executable that speaks JSON-RPC on stdio. Cases: initialize, `session/new`, streamed updates, permission allow, permission deny, `session/load`, child exit mid-turn
3. **UI (Vitest + Testing Library):** session rows, tool cards, permission buttons including deny-only, diff empty/error/list, composer send/stop

Not in CI: a logged-in live Grok account.

**Manual checklist** before calling V1 done:

1. Open a folder
2. New session
3. Prompt that reads a file
4. Prompt that writes a file; Allow once; diff pane shows the file
5. Deny a later write; file unchanged
6. Switch to another session and back
7. Quit, reopen, resume
8. Quit while a turn is running; confirm the tool did not keep going as allow

## Repo layout

```
grok-desktop/
  README.md
  docs/superpowers/specs/2026-09-13-grok-build-desktop-design.md
  package.json
  src/
    main.tsx
    App.tsx
    screens/Setup.tsx
    screens/Home.tsx
    screens/Workspace.tsx
    panes/SessionList.tsx
    panes/Chat.tsx
    panes/Diffs.tsx
    components/ToolCard.tsx
    components/PermissionCard.tsx
    components/Composer.tsx
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/
      lib.rs
      grok_bin.rs
      auth.rs
      agent.rs
      sessions.rs
      diffs.rs
      recents.rs
    tests/
      mock_agent.rs
```

## Success

V1 is done when a signed-in user on this machine can open `obsidian`, start a session, get a file write approved, see the diff, resume that session after quit, and the mock ACP tests pass.

## Later (not this spec)

Parallel agents and worktree board. Diff stage/discard. Leader process sharing with the TUI. Chat tab. Notarization. Intel builds.
