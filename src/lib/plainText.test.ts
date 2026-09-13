import { describe, expect, it } from "vitest";
import { toPlainText } from "./plainText";

describe("toPlainText", () => {
  it("strips bold, code, headings, and tables", () => {
    const src = `This is **Vikramaditya**.

## Layout

| Area | What it is |
|:-----|:-----------|
| Entry | \`vikramaditya.py\` |
`;
    const out = toPlainText(src);
    expect(out).toContain("This is Vikramaditya.");
    expect(out).not.toContain("**");
    expect(out).not.toContain("| Area");
    expect(out).not.toContain("##");
    expect(out).toContain("Area: Entry. What it is: vikramaditya.py");
  });
});
