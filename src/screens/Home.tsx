export function Home({
  folders,
  onOpen,
  onPick,
  onRemove,
}: {
  folders: string[];
  onOpen: (path: string) => void;
  onPick: () => void;
  onRemove: (path: string) => void;
}) {
  return (
    <div className="home">
      <div className="home-top">
        <h1>Grok Build</h1>
        <button className="primary" onClick={onPick}>
          Open Folder
        </button>
      </div>
      <h2>Recent</h2>
      {folders.length === 0 ? (
        <p className="muted">No recent folders.</p>
      ) : (
        <ul className="recents">
          {folders.map((f) => (
            <li key={f}>
              <button className="recent" onClick={() => onOpen(f)}>
                {f}
              </button>
              <button className="link" onClick={() => onRemove(f)}>
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
