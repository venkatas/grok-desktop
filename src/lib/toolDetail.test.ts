import { describe, expect, it } from "vitest";
import { formatToolDetail } from "./toolDetail";

describe("formatToolDetail", () => {
  it("pulls list_dir tree text out of ACP JSON", () => {
    const raw = {
      Content: {
        absolute_root_path: "/tmp/proj",
        content: "- /tmp/proj/\n  - README.md\n",
      },
      type: "ListDir",
    };
    expect(formatToolDetail(raw)).toContain("README.md");
    expect(formatToolDetail(raw)).not.toContain("absolute_root_path");
  });

  it("pulls text from content blocks", () => {
    const raw = [
      { type: "content", content: { type: "text", text: "List files, git status" } },
    ];
    expect(formatToolDetail(raw)).toBe("List files, git status");
  });
});
