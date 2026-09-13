export function ToolCard({
  title,
  status,
  detail,
}: {
  title: string;
  status: string;
  detail?: string;
}) {
  return (
    <div className="tool-card">
      <div className="tool-head">
        <b>{title}</b>
        <span className={`tool-status status-${status}`}>{status}</span>
      </div>
      {detail ? <pre className="tool-detail">{detail}</pre> : null}
    </div>
  );
}
