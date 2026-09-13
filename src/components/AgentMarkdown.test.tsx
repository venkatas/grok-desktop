import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AgentMarkdown } from "./AgentMarkdown";

describe("AgentMarkdown", () => {
  it("renders bold and table cells, not source marks", () => {
    render(
      <AgentMarkdown text={"This is **Vikramaditya**.\n\n| Area | What |\n|:-----|:-----|\n| Entry | `hunt.py` |\n"} />,
    );
    expect(screen.getByText("Vikramaditya").tagName).toBe("STRONG");
    expect(screen.getByText("hunt.py").tagName).toBe("CODE");
    expect(screen.queryByText(/\*\*Vikramaditya\*\*/)).toBeNull();
    expect(screen.getByText("Area").tagName).toBe("TH");
  });
});
