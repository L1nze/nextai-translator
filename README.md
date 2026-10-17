# NextAI Translator (personal fork)

A personal, Windows-only customization of
[nextai-translator](https://github.com/nextai-translator/nextai-translator),
maintained for my own use. See [README-CN.md](README-CN.md) for Chinese.

Customizations include: extra LLM providers and free machine-translation
engines, a separate provider for polishing/summarizing, a translation
hotkey, frameless windows with a custom titlebar, and an auto-updater
pointed at this fork's releases. Upstream promotion banners and telemetry
are removed, and only the Windows desktop target is built.

## Development

```bash
pnpm install
pnpm dev            # Tauri dev shell
pnpm test           # unit tests
pnpm release <version> [notes]   # signed installer into release/
```

The updater signing key lives at `src-tauri/keys/updater.key` — never
commit it and never lose it; losing it breaks all future auto-updates.
