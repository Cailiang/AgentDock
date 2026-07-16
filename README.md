# AgentDock

<p align="center">
  <img src="public/agentdock-logo.svg" width="96" height="96" alt="AgentDock logo">
</p>

AgentDock 是面向普通用户的 AI 编程客户端桌面管理工具。它把客户端安装、供应商配置、Skills、MCP、用量统计和本机诊断集中到一个原生桌面程序中，支持 Windows、macOS 和 Linux。

> 当前版本为 `0.1.8`，仍处于早期预览阶段。使用供应商切换和 MCP 同步前，建议保留重要客户端配置的备份。

## 功能

- **客户端管理**：检测、安装、更新、启动和卸载 AgentDock 托管的客户端。
- **国内网络适配**：优先尝试 npm/PyPI 国内镜像，失败后回退官方源；对提供完整性信息或官方摘要的安装包执行校验。
- **供应商配置**：按客户端管理官方登录、预设供应商和自定义兼容接口，支持模型列表获取、连接测试、配置预览与编辑、切换和备份。
- **Skills 管理**：安装、卸载并按客户端启用 Skill，再同步到真实客户端目录。
- **MCP 管理**：新增、编辑、删除、启停、导入已有配置并同步到多个客户端；支持 `stdio`、HTTP 和 SSE；可以连接服务器查看工具、说明和参数 Schema。
- **本地统计**：读取本机 Codex、Claude Code、OpenCode 和 Grok 会话记录，按 7/30/90 天展示 Token、请求、可计算成本和趋势，并按客户端、供应商或模型拆分。
- **诊断中心**：主动检查目录权限、客户端版本与配置、供应商连接、MCP 配置和统计数据，可导出已脱敏的诊断包。

## 支持的客户端

| 客户端 | 检测 | 一键安装/更新 | 供应商配置 | MCP |
| --- | :---: | :---: | :---: | :---: |
| Codex | 是 | 是 | 是 | 是 |
| Claude Code | 是 | 是 | 是 | 是 |
| Antigravity CLI (Agy) | 是 | 是 | 是 | 是 |
| Grok | 是 | 是 | 是 | 是 |
| OpenCode | 是 | 是 | 是 | 是 |
| OpenClaw | 是 | 是 | 是 | 是 |
| Hermes Agent | 是 | 是 | 是 | 是 |
| Claude Desktop | 是 | 否 | 是 | 是 |

自动安装会选择与当前操作系统和 CPU 架构匹配的客户端包。Claude Desktop 目前只检测系统中已有的安装，不由 AgentDock 下载或卸载。

## 获取安装包

稳定安装包将发布在仓库的 [Releases](https://github.com/Cailiang/AgentDock/releases) 页面。每次推送到 `main` 后，GitHub Actions 也会分别构建 Windows、macOS 和 Linux 安装包，可从对应的 `Desktop Build` 工作流运行记录中下载构建产物。

未签名或未公证的预览包可能触发系统安全提示。正式分发时应配置平台签名证书，不建议要求最终用户关闭系统安全功能。

## 本地开发

依赖：

- Node.js 20 或更高版本
- Rust stable toolchain
- 当前系统所需的 [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
npm ci
npm run dev
```

只构建前端：

```bash
npm run build:ui
```

构建桌面安装包：

```bash
npm run build
```

产物位于 `src-tauri/target/release/bundle/`。CI 配置见 [`.github/workflows/desktop-build.yml`](.github/workflows/desktop-build.yml)。

## 数据与安全

- AgentDock 不要求用户在项目源码或环境变量中填写 API Key。
- 供应商 API Key 保存在操作系统的 AgentDock 本地配置目录，并在 Unix 系统上限制文件权限；当前版本尚未接入系统钥匙串或凭据保险库，请保护好本机账户和备份。
- 用量统计直接读取本机客户端会话数据，不会由 AgentDock 上传到远端服务。
- 网络请求仅用于客户端/软件版本查询与下载、供应商连接测试、模型列表读取，以及用户配置的 MCP 服务连接。
- 诊断导出会移除 API Key、URL 凭据、MCP 环境变量值和请求头值，但可能包含操作系统、版本和本机文件路径。对外分享前仍应人工检查。
- 仓库不包含本地配置、密钥、内部设计稿、测试截图、第三方客户端安装包或构建产物。

发现安全问题请阅读 [`SECURITY.md`](SECURITY.md)，不要在公开 Issue 中提交密钥或可直接利用的漏洞细节。

## 致谢

AgentDock 的供应商与 MCP 管理工作流参考了开源项目 [cc-switch](https://github.com/farion1231/cc-switch)。相关许可证声明见 [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)。

## License

AgentDock 自有源码与资产采用 [MIT](LICENSE) 许可证，Copyright (c) 2026 Cailiang。

第三方客户端名称、Logo 和商标不包含在 AgentDock 的 MIT 授权中，仅用于说明兼容性，权利归各自所有者。详见 [`ASSET_NOTICES.md`](ASSET_NOTICES.md)。
