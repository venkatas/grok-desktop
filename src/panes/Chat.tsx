import { Composer } from "../components/Composer";
import { PermissionCard } from "../components/PermissionCard";
import { ToolCard } from "../components/ToolCard";
import type { ChatItem } from "../lib/types";

export function Chat({
  items,
  draft,
  running,
  frozen,
  onDraft,
  onSend,
  onStop,
  onPermission,
}: {
  items: ChatItem[];
  draft: string;
  running: boolean;
  frozen?: boolean;
  onDraft: (v: string) => void;
  onSend: () => void;
  onStop: () => void;
  onPermission: (optionId: string | null) => void;
}) {
  return (
    <section className="chat">
      <div className="msgs">
        {items.length === 0 ? (
          <p className="muted">Ask Grok about this folder. + New session starts a fresh thread.</p>
        ) : null}
        {items.map((item, i) => {
          if (item.kind === "user") return <div key={i} className="u">{item.text}</div>;
          if (item.kind === "agent") return <div key={i} className="a">{item.text}</div>;
          if (item.kind === "thought") {
            return (
              <details key={i} className="thought">
                <summary>Thoughts</summary>
                <div>{item.text}</div>
              </details>
            );
          }
          if (item.kind === "tool") {
            return (
              <ToolCard
                key={item.id + i}
                title={item.title}
                status={item.status}
                detail={item.detail}
              />
            );
          }
          if (item.kind === "plan") {
            return (
              <ul key={i} className="plan">
                {item.entries.map((e, j) => (
                  <li key={j}>
                    {e.status} · {e.content}
                  </li>
                ))}
              </ul>
            );
          }
          if (item.kind === "permission") {
            return (
              <PermissionCard
                key={i}
                title={item.title}
                options={item.options}
                onChoose={onPermission}
              />
            );
          }
          return <div key={i} className="err">{item.text}</div>;
        })}
      </div>
      <Composer
        value={draft}
        running={running}
        disabled={frozen}
        onChange={onDraft}
        onSend={onSend}
        onStop={onStop}
      />
    </section>
  );
}
