import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Chat } from "./panes/Chat";
import { Diffs } from "./panes/Diffs";
import { SessionList } from "./panes/SessionList";
import { Home } from "./screens/Home";
import { Setup } from "./screens/Setup";
import type { AppStatus, ChatItem, DiffFile, PermissionOption, Screen, SessionRow } from "./lib/types";
import { formatToolDetail } from "./lib/toolDetail";
import "./App.css";

type ConfigOpt = { configId: string; values?: { value: string; name?: string }[] };

function applyUpdate(items: ChatItem[], params: Record<string, unknown>): ChatItem[] {
  const update = (params.update ?? params) as Record<string, unknown>;
  const kind = String(update.sessionUpdate ?? "");
  if (kind === "agent_message_chunk") {
    const text = String((update.content as { text?: string } | undefined)?.text ?? "");
    const last = items[items.length - 1];
    if (last?.kind === "agent") {
      return [...items.slice(0, -1), { kind: "agent", text: last.text + text }];
    }
    return [...items, { kind: "agent", text }];
  }
  if (kind === "agent_thought_chunk") {
    const text = String((update.content as { text?: string } | undefined)?.text ?? "");
    const last = items[items.length - 1];
    if (last?.kind === "thought") {
      return [...items.slice(0, -1), { kind: "thought", text: last.text + text }];
    }
    return [...items, { kind: "thought", text }];
  }
  if (kind === "tool_call") {
    return [
      ...items,
      {
        kind: "tool",
        id: String(update.toolCallId ?? items.length),
        title: String(update.title ?? "tool"),
        status: String(update.status ?? "pending"),
      },
    ];
  }
  if (kind === "tool_call_update") {
    const id = String(update.toolCallId ?? "");
    return items.map((it) =>
      it.kind === "tool" && it.id === id
        ? {
            ...it,
            status: String(update.status ?? it.status),
            detail: formatToolDetail(update.content ?? update.rawOutput) || it.detail,
          }
        : it,
    );
  }
  if (kind === "plan") {
    const entries = Array.isArray(update.entries) ? (update.entries as { content: string; status: string }[]) : [];
    return [...items.filter((i) => i.kind !== "plan"), { kind: "plan", entries }];
  }
  return items;
}

export default function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [screen, setScreen] = useState<Screen>("setup");
  const [folders, setFolders] = useState<string[]>([]);
  const [folder, setFolder] = useState<string | null>(null);
  const [sessions, setSessions] = useState<SessionRow[]>([]);
  const [liveId, setLiveId] = useState<string | null>(null);
  const [items, setItems] = useState<ChatItem[]>([]);
  const [draft, setDraft] = useState("");
  const [running, setRunning] = useState(false);
  const [query, setQuery] = useState("");
  const [diffs, setDiffs] = useState<DiffFile[]>([]);
  const [diffError, setDiffError] = useState<string | null>(null);
  const [diffSel, setDiffSel] = useState<string | null>(null);
  const [banner, setBanner] = useState<string | null>(null);
  const [setupError, setSetupError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [worktree, setWorktree] = useState(false);
  const [worktreeOk, setWorktreeOk] = useState(true);
  const [model, setModel] = useState<string>("");
  const [effort, setEffort] = useState<string>("");
  const [models, setModels] = useState<string[]>([]);
  const [efforts, setEfforts] = useState<string[]>([]);
  const [authFrozen, setAuthFrozen] = useState(false);

  const liveState = running ? "Working" : items.some((i) => i.kind === "permission") ? "Needs input" : "Idle";

  async function refreshStatus() {
    const s = await invoke<AppStatus>("app_status");
    setStatus(s);
    if (!s.grok_bin || !s.signed_in) setScreen("setup");
    else if (screen === "setup") setScreen("home");
    return s;
  }

  async function refreshRecents() {
    const r = await invoke<{ folders: string[] }>("list_recents");
    setFolders(r.folders ?? []);
  }

  async function loadDiffs() {
    try {
      const files = await invoke<DiffFile[]>("git_diffs");
      setDiffs(files);
      setDiffError(null);
    } catch {
      setDiffError("Could not load diffs");
    }
  }

  useEffect(() => {
    refreshStatus().then(() => refreshRecents());
    const unsubs: Array<() => void> = [];
    listen<Record<string, unknown>>("agent://update", (e) => {
      setItems((cur) => applyUpdate(cur, e.payload));
    }).then((u) => unsubs.push(u));
    listen<{ options: PermissionOption[]; toolCall?: { title?: string } }>("agent://permission", (e) => {
      setItems((cur) => [
        ...cur,
        {
          kind: "permission",
          options: e.payload.options ?? [],
          title: e.payload.toolCall?.title ?? "Permission needed",
        },
      ]);
    }).then((u) => unsubs.push(u));
    listen<{ ok: boolean; error?: string }>("agent://prompt-done", (e) => {
      setRunning(false);
      if (!e.payload.ok) {
        setItems((cur) => [...cur, { kind: "error", text: e.payload.error ?? "Prompt failed" }]);
      }
    }).then((u) => unsubs.push(u));
    listen("agent://exit", () => {
      setBanner("Agent process died");
      setRunning(false);
    }).then((u) => unsubs.push(u));
    listen("agent://refresh-diffs", () => {
      loadDiffs();
    }).then((u) => unsubs.push(u));
    return () => unsubs.forEach((u) => u());
  }, []);

  async function pickFolder() {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") await openFolder(dir);
  }

  async function openFolder(path: string) {
    try {
      const rows = await invoke<SessionRow[]>("open_folder", { path });
      setFolder(path);
      setSessions(rows);
      setScreen("workspace");
      setItems([]);
      setLiveId(null);
      setBanner(null);
      await startSession(path);
    } catch (e) {
      setBanner(String(e));
    }
  }

  function applyConfig(raw: unknown) {
    const opts = (raw as ConfigOpt[] | undefined) ?? [];
    const modelOpt = opts.find((o) => o.configId === "model");
    const effortOpt = opts.find((o) => o.configId === "reasoning_effort");
    setModels((modelOpt?.values ?? []).map((v) => v.value));
    setEfforts((effortOpt?.values ?? []).map((v) => v.value));
    if (modelOpt?.values?.[0]) setModel(modelOpt.values[0].value);
    if (effortOpt?.values?.[0]) setEffort(effortOpt.values[0].value);
  }

  async function startSession(cwd: string): Promise<string | null> {
    try {
      const res = await invoke<{ sessionId: string; configOptions?: unknown }>("session_new", {
        cwd,
        worktree,
      });
      setLiveId(res.sessionId);
      setItems([]);
      applyConfig(res.configOptions);
      setBanner(null);
      const rows = await invoke<SessionRow[]>("list_sessions_cmd", { cwd });
      setSessions(rows);
      return res.sessionId;
    } catch (e) {
      const msg = String(e);
      if (msg.toLowerCase().includes("worktree")) setWorktreeOk(false);
      setBanner(msg);
      return null;
    }
  }

  async function newSession() {
    if (!folder) return;
    await startSession(folder);
  }

  async function selectSession(id: string) {
    if (!folder) return;
    try {
      if (running) await invoke("session_cancel");
      const res = await invoke<{ updates?: unknown[] }>("session_load", { cwd: folder, id });
      setLiveId(id);
      let next: ChatItem[] = [];
      for (const u of res.updates ?? []) {
        next = applyUpdate(next, u as Record<string, unknown>);
      }
      setItems(next);
      loadDiffs();
    } catch (e) {
      setBanner(String(e));
    }
  }

  async function send() {
    const text = draft.trim();
    if (!text || !folder) return;
    if (!liveId) {
      const id = await startSession(folder);
      if (!id) return;
    }
    setDraft("");
    setItems((cur) => [...cur, { kind: "user", text }]);
    setRunning(true);
    try {
      await invoke("session_prompt", { text });
    } catch (e) {
      setRunning(false);
      setItems((cur) => [...cur, { kind: "error", text: String(e) }]);
    }
  }

  const folderName = useMemo(() => (folder ? folder.split("/").pop() ?? folder : ""), [folder]);

  if (!status || screen === "setup") {
    return (
      <Setup
        status={status ?? { grok_bin: null, signed_in: false, grok_home: "" }}
        busy={busy}
        error={setupError}
        onRecheck={() => refreshStatus()}
        onSignIn={async () => {
          setBusy(true);
          setSetupError(null);
          try {
            await invoke("sign_in");
            await refreshStatus();
            await refreshRecents();
            setAuthFrozen(false);
          } catch (e) {
            setSetupError(String(e));
          } finally {
            setBusy(false);
          }
        }}
      />
    );
  }

  if (screen === "home" || !folder) {
    return (
      <Home
        folders={folders}
        onOpen={openFolder}
        onPick={pickFolder}
        onRemove={async (p) => {
          const r = await invoke<{ folders: string[] }>("remove_recent", { path: p });
          setFolders(r.folders ?? []);
        }}
      />
    );
  }

  return (
    <div className="workspace">
      <div className="titlebar">
        <span className="name">Grok Build</span>
        <span className="meta">{folderName}</span>
        <div className="right">
          {models.length > 0 ? (
            <select className="chip" value={model} onChange={(e) => {
              setModel(e.target.value);
              invoke("set_config_option", { configId: "model", value: e.target.value });
            }}>
              {models.map((m) => <option key={m}>{m}</option>)}
            </select>
          ) : (
            <span className="chip">grok</span>
          )}
          {efforts.length > 0 ? (
            <select className="chip" value={effort} onChange={(e) => {
              setEffort(e.target.value);
              invoke("set_config_option", { configId: "reasoning_effort", value: e.target.value });
            }}>
              {efforts.map((m) => <option key={m}>{m}</option>)}
            </select>
          ) : null}
          <label className="chip">
            <input
              type="checkbox"
              disabled={!worktreeOk}
              checked={worktree}
              onChange={(e) => setWorktree(e.target.checked)}
            />{" "}
            worktree {worktreeOk ? (worktree ? "on" : "off") : "unavailable"}
          </label>
          {running ? (
            <button onClick={() => invoke("session_cancel")}>Stop</button>
          ) : null}
        </div>
      </div>
      {banner ? (
        <div className="banner">
          <span>{banner}</span>
          <button onClick={() => invoke("retry_agent").then(() => setBanner(null))}>Retry</button>
        </div>
      ) : null}
      <div className="body">
        <SessionList
          sessions={sessions}
          liveId={liveId}
          liveState={liveState}
          query={query}
          folder={folder}
          onQuery={setQuery}
          onNew={newSession}
          onSelect={selectSession}
          onChangeFolder={pickFolder}
        />
        <Chat
          items={items}
          draft={draft}
          running={running}
          frozen={authFrozen}
          onDraft={setDraft}
          onSend={send}
          onStop={() => invoke("session_cancel")}
          onPermission={(id) => {
            invoke("permission_respond", { optionId: id });
            setItems((cur) => cur.filter((i) => i.kind !== "permission"));
          }}
        />
        <Diffs files={diffs} error={diffError} selected={diffSel} onSelect={setDiffSel} />
      </div>
    </div>
  );
}
