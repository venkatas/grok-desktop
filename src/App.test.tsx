import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Composer } from "./components/Composer";
import { PermissionCard } from "./components/PermissionCard";
import { ToolCard } from "./components/ToolCard";
import { Chat } from "./panes/Chat";
import { Diffs } from "./panes/Diffs";
import { SessionList } from "./panes/SessionList";

describe("session list", () => {
  it("renders titles", () => {
    render(
      <SessionList
        sessions={[{ id: "1", title: "Fix login CSRF", updated_at: new Date().toISOString() }]}
        liveId="1"
        liveState="Working"
        query=""
        folder="/tmp/proj"
        onQuery={() => {}}
        onNew={() => {}}
        onSelect={() => {}}
        onChangeFolder={() => {}}
      />,
    );
    expect(screen.getByText("Fix login CSRF")).toBeTruthy();
    expect(screen.getByText("+ New session")).toBeTruthy();
  });
});

describe("tool card", () => {
  it("shows status", () => {
    render(<ToolCard title="read_file auth_utils.py" status="completed" />);
    expect(screen.getByText("completed")).toBeTruthy();
  });
});

describe("permission card", () => {
  it("adds Deny when agent omitted it", () => {
    render(
      <PermissionCard
        title="write a.rs"
        options={[{ optionId: "allow-once", name: "Allow once", kind: "allow_once" }]}
        onChoose={() => {}}
      />,
    );
    expect(screen.getByText("Allow once")).toBeTruthy();
    expect(screen.getByText("Deny")).toBeTruthy();
  });
});

describe("diffs", () => {
  it("empty state", () => {
    render(<Diffs files={[]} error={null} selected={null} onSelect={() => {}} />);
    expect(screen.getByText("No local changes")).toBeTruthy();
  });
  it("error state", () => {
    render(<Diffs files={[]} error="fail" selected={null} onSelect={() => {}} />);
    expect(screen.getByText("Could not load diffs")).toBeTruthy();
  });
  it("lists files", () => {
    render(
      <Diffs
        files={[{ path: "a.rs", added: 1, removed: 0, patch: "+x" }]}
        error={null}
        selected={null}
        onSelect={() => {}}
      />,
    );
    expect(screen.getByText(/a.rs/)).toBeTruthy();
  });
});

describe("chat empty state", () => {
  it("tells you how to start", () => {
    render(
      <Chat
        items={[]}
        draft=""
        running={false}
        onDraft={() => {}}
        onSend={() => {}}
        onStop={() => {}}
        onPermission={() => {}}
      />,
    );
    expect(screen.getByText(/Ask Grok about this folder/)).toBeTruthy();
  });
});

describe("composer", () => {
  it("send vs stop", () => {
    const { rerender } = render(
      <Composer value="hi" running={false} onChange={() => {}} onSend={() => {}} onStop={() => {}} />,
    );
    expect(screen.getByText("Send")).toBeTruthy();
    rerender(
      <Composer value="hi" running={true} onChange={() => {}} onSend={() => {}} onStop={() => {}} />,
    );
    expect(screen.getByText("Stop")).toBeTruthy();
  });
});
