import type { SessionRow } from "../lib/types";
import { relativeTime } from "../lib/relativeTime";

export function SessionList({
  sessions,
  liveId,
  liveState,
  query,
  folder,
  onQuery,
  onNew,
  onSelect,
  onChangeFolder,
}: {
  sessions: SessionRow[];
  liveId: string | null;
  liveState: string;
  query: string;
  folder: string;
  onQuery: (q: string) => void;
  onNew: () => void;
  onSelect: (id: string) => void;
  onChangeFolder: () => void;
}) {
  const q = query.toLowerCase();
  const rows = sessions.filter((s) => s.title.toLowerCase().includes(q) || s.id.includes(q));
  return (
    <aside className="side">
      <button className="new-btn" onClick={onNew}>
        + New session
      </button>
      <input
        className="search"
        placeholder="Search sessions"
        value={query}
        onChange={(e) => onQuery(e.target.value)}
      />
      <div className="session-rows">
        {rows.map((s) => {
          const on = s.id === liveId;
          return (
            <button key={s.id} className={on ? "sess on" : "sess"} onClick={() => onSelect(s.id)}>
              <span className="t">{s.title}</span>
              <span className="s">
                {on ? liveState : "Idle"} · {relativeTime(s.updated_at)}
              </span>
            </button>
          );
        })}
      </div>
      <div className="side-foot">
        <div className="path" title={folder}>
          {folder}
        </div>
        <button onClick={onChangeFolder}>Change folder</button>
      </div>
    </aside>
  );
}
