function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());
}

function isTableRow(line: string): boolean {
  const t = line.trim();
  return t.includes("|") && !/^\|?\s*[:\-| ]+\|/.test(t);
}

function isSep(line: string): boolean {
  return /^\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(line.trim());
}

function convertTables(src: string): string {
  const lines = src.split("\n");
  const out: string[] = [];
  let i = 0;
  while (i < lines.length) {
    if (isTableRow(lines[i]) && i + 1 < lines.length && isSep(lines[i + 1])) {
      const headers = cells(lines[i]);
      i += 2;
      while (i < lines.length && isTableRow(lines[i])) {
        const row = cells(lines[i]);
        out.push(
          headers
            .map((h, idx) => `${h}: ${row[idx] ?? ""}`.replace(/\s+/g, " ").trim())
            .join(". "),
        );
        i += 1;
      }
      continue;
    }
    out.push(lines[i]);
    i += 1;
  }
  return out.join("\n");
}

export function toPlainText(src: string): string {
  let s = src.replace(/\r\n/g, "\n");
  s = s.replace(/```[\w-]*\n?([\s\S]*?)```/g, "$1");
  s = convertTables(s);
  s = s.replace(/^#{1,6}\s+/gm, "");
  s = s.replace(/\*\*([^*]+)\*\*/g, "$1");
  s = s.replace(/__([^_]+)__/g, "$1");
  s = s.replace(/\*([^*]+)\*/g, "$1");
  s = s.replace(/`([^`]+)`/g, "$1");
  s = s.replace(/\[([^\]]+)\]\([^)]+\)/g, "$1");
  s = s.replace(/^>\s?/gm, "");
  s = s.replace(/^\|?\s*[:\-| ]+\|\s*[:\-| ]+.*$/gm, "");
  s = s.replace(/\n{3,}/g, "\n\n");
  return s.trim();
}
