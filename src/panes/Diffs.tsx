import type { DiffFile } from "../lib/types";

export function Diffs({
  files,
  error,
  selected,
  onSelect,
}: {
  files: DiffFile[];
  error: string | null;
  selected: string | null;
  onSelect: (path: string) => void;
}) {
  const file = files.find((f) => f.path === selected) ?? files[0];
  if (error) {
    return (
      <aside className="diff">
        <div className="label">Changes</div>
        <p className="muted">Could not load diffs</p>
      </aside>
    );
  }
  if (files.length === 0) {
    return (
      <aside className="diff">
        <div className="label">Changes</div>
        <p className="muted">No local changes</p>
      </aside>
    );
  }
  return (
    <aside className="diff">
      <div className="label">Changes · {files.length} files</div>
      {files.map((f) => (
        <button
          key={f.path}
          className={file?.path === f.path ? "df on" : "df"}
          onClick={() => onSelect(f.path)}
        >
          {f.path.split("/").pop()}{" "}
          <span className="add">+{f.added}</span>
        </button>
      ))}
      {file ? <pre className="patch">{file.patch || "(no patch text)"}</pre> : null}
    </aside>
  );
}
