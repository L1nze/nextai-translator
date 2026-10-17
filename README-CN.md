# NextAI Translator（个人定制版）

基于 [nextai-translator](https://github.com/nextai-translator/nextai-translator)
的个人定制 fork，仅构建 Windows 桌面端，自用。

主要定制：新增多个 LLM 供应商与免费翻译接口、润色/总结独立供应商配置、
翻译快捷键、无边框窗口与自定义标题栏、自动更新指向本仓库；移除了上游的
推广横幅与遥测，不再构建浏览器扩展等多端产物。

## 开发

```bash
pnpm install
pnpm dev            # Tauri 开发模式
pnpm test           # 单元测试
pnpm release <版本号> [说明]   # 签名构建安装包到 release/
```

自动更新签名私钥位于 `src-tauri/keys/updater.key`——切勿提交到仓库，
也不要弄丢，丢失后将无法再发布自动更新。
