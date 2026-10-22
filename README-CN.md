# NextAI Translator

Windows 桌面翻译工具。在任意应用中选中文本，按全局快捷键即可原地翻译。

## 功能

-   全局快捷键翻译选中文本（默认 `Ctrl+T`）
-   划词图标：点击图标翻译选中内容
-   快速翻译浮层与行内查词模式
-   截图 OCR 翻译
-   写作模式：输入时把译文写入任意输入框
-   朗读播放
-   支持多家 LLM 服务商与免费机器翻译引擎
-   自动更新与 NSIS 安装包

## 开发

```bash
pnpm install
pnpm dev        # Tauri 开发壳，带热更新
pnpm test       # 单元测试
pnpm build      # 打包，安装包输出到 release/
pnpm release <version> [notes]   # 签名发布产物输出到 release/v<version>/
```

需要 Node 18+、pnpm 9、Rust 及 Tauri 2 系统依赖。

## 更新

应用从本仓库的 releases 检查更新。更新签名私钥位于 `src-tauri/keys/updater.key`，请勿提交、务必备份——丢失后将无法再发布自动更新。

## 许可证

AGPL-3.0
