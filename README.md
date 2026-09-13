# Grok Build

Mac desktop command center for the Grok coding agent. It wraps the local `grok` CLI over ACP. It is not a second IDE and not a grok.com chat wrapper.

## Requirements

- macOS 14+ on Apple Silicon
- [Grok CLI](https://x.ai/cli) installed and signed in (`grok login`)
- Rust (rustup) and Node.js 20+

## Dev

```bash
npm install
npm run tauri dev
```

## Tests

```bash
cd src-tauri && cargo test
npm test
```

## Build the app

```bash
npm run tauri build
```

The `.app` lands in `src-tauri/target/release/bundle/macos/Grok Build.app`.

## Notes

- One live session at a time. Do not drive the same live session from the TUI and this app together.
- Permissions default to deny if you close the window or switch sessions without answering.
- Diffs are view-only in V1.

See [docs/superpowers/specs/2026-09-13-grok-build-desktop-design.md](docs/superpowers/specs/2026-09-13-grok-build-desktop-design.md).
