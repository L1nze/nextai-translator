# NextAI Translator

A Windows desktop translation tool. Select text in any app and press the
global hotkey to translate it in place.

## Features

-   Global-hotkey translation of selected text (`Ctrl+T` by default)
-   Selection icon: click it to translate the highlighted text
-   Quick Translator overlay and compact inline-lookup mode
-   Screenshot OCR translation
-   Writing mode: translate text into any input box while typing
-   Text-to-speech playback
-   Multiple LLM providers plus free machine-translation engines
-   Auto-updater and NSIS installer

## Development

```bash
pnpm install
pnpm dev        # Tauri dev shell with HMR
pnpm test       # unit tests
pnpm build      # package; installer lands in release/
pnpm release <version> [notes]   # signed release artifacts in release/v<version>/
```

Requires Node 18+, pnpm 9, Rust, and the Tauri 2 system prerequisites.

## Updates

The app checks this repository's releases for updates. The updater signing
key lives at `src-tauri/keys/updater.key` — never commit it, and keep a
backup: losing it breaks all future auto-updates.

## License

AGPL-3.0
