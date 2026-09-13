export function formatToolDetail(raw: unknown): string {
  if (raw == null || raw === "") return "";
  if (typeof raw === "string") {
    const trimmed = raw.trim();
    if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
      try {
        return formatToolDetail(JSON.parse(trimmed));
      } catch {
        return raw;
      }
    }
    return raw;
  }
  if (Array.isArray(raw)) {
    return raw.map(formatToolDetail).filter(Boolean).join("\n");
  }
  if (typeof raw !== "object") return String(raw);

  const o = raw as Record<string, unknown>;
  const listed = o.Content ?? o.content;
  if (listed && typeof listed === "object") {
    const inner = listed as Record<string, unknown>;
    if (typeof inner.content === "string") return inner.content;
    if (typeof inner.text === "string") return inner.text;
  }
  if (typeof listed === "string") return listed;
  if (typeof o.text === "string") return o.text;
  if (o.type === "content" || o.type === "text") return formatToolDetail(o.content ?? o.text);
  if (typeof o.path === "string" && (o.newText != null || o.oldText != null)) {
    return `${o.path}\n${String(o.newText ?? o.oldText ?? "")}`;
  }
  return "";
}
