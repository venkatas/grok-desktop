import type { PermissionOption } from "../lib/types";

export function PermissionCard({
  title,
  options,
  onChoose,
}: {
  title: string;
  options: PermissionOption[];
  onChoose: (optionId: string | null) => void;
}) {
  const hasDeny = options.some(
    (o) => o.kind === "reject_once" || o.kind === "reject_always" || o.name === "Deny",
  );
  return (
    <div className="perm-card">
      <b>{title}</b>
      <div className="perm-btns">
        {options.map((o) => (
          <button
            key={o.optionId}
            className={o.kind.startsWith("allow") ? "primary" : ""}
            onClick={() => onChoose(o.optionId)}
          >
            {o.name}
          </button>
        ))}
        {!hasDeny ? (
          <button onClick={() => onChoose(null)}>Deny</button>
        ) : null}
      </div>
    </div>
  );
}
