export type SessionRow = {
  id: string;
  title: string;
  updated_at: string;
  model?: string | null;
};

export type DiffFile = {
  path: string;
  added: number;
  removed: number;
  patch: string;
};

export type AppStatus = {
  grok_bin: string | null;
  signed_in: boolean;
  grok_home: string;
};

export type PermissionOption = {
  optionId: string;
  name: string;
  kind: string;
};

export type ChatItem =
  | { kind: "user"; text: string }
  | { kind: "agent"; text: string }
  | { kind: "thought"; text: string }
  | {
      kind: "tool";
      id: string;
      title: string;
      status: string;
      detail?: string;
    }
  | { kind: "plan"; entries: { content: string; status: string }[] }
  | {
      kind: "permission";
      options: PermissionOption[];
      title: string;
    }
  | { kind: "error"; text: string };

export type Screen = "setup" | "home" | "workspace";
