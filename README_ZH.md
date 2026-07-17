<div align="center">

<img src="public/agentdock-logo.svg" width="104" height="104" alt="AgentDock Logo">

# AgentDock

### 面向普通用户的 AI 编程客户端桌面中枢

[![Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml/badge.svg)](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-4f6f68)](https://github.com/Cailiang/AgentDock/actions)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-2f5f55)](LICENSE)

[English](README.md) | 简体中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md)

</div>

AgentDock 用一个原生桌面程序完成 AI 编程客户端、供应商、Skills 和 MCP 服务器的安装与管理。它面向希望直接使用 Codex、Claude Code、Grok 等智能代理，但不想手动安装运行时或编辑 JSON、TOML、环境变量文件的用户。

> AgentDock `0.1.15` 仍处于早期预览阶段。使用供应商切换或 MCP 同步前，请保留重要客户端配置的备份。

## 为什么选择 AgentDock？

不同 AI 编程客户端使用不同的安装方式、配置格式、模型协议和 MCP 结构。专业开发者可以手动处理这些差异，但普通用户第一次使用时会遇到很高的门槛。

AgentDock 把新手流程放在首位：

1. 检测电脑上已经安装的客户端。
2. 一键安装或更新客户端。
3. 添加官方登录、预设供应商或自定义兼容 API。
4. 测试连接、检查生成的配置，然后启动客户端。

最终用户不需要另外安装 Node.js、npm、Python，也不需要手动编辑配置。客户端需要运行时时，AgentDock 会在自己的数据目录中自动准备托管环境。

## 核心功能

### 客户端全生命周期

- 检测系统安装和 AgentDock 托管安装。
- 安装、更新、启动和卸载 AgentDock 托管的客户端。
- 优先尝试国内可用的 npm/PyPI 镜像，失败后回退官方源。
- 自动选择当前操作系统和 CPU 架构对应的安装包。
- 当来源提供摘要或 npm integrity 信息时校验安装包完整性。

### 供应商管理

- 按客户端独立管理供应商。
- 支持预设供应商、官方登录和完全自定义的兼容接口。
- 自动获取供应商模型列表，并通过下拉列表选择默认模型。
- 根据客户端支持 OpenAI Responses、Chat Completions、Anthropic Messages 和 Gemini 兼容协议。
- 支持连接测试、生成配置预览与编辑、供应商切换，并在写入前备份已有文件。

### Skills 与 MCP

- 安装和卸载 Skills，按客户端启用并同步到真实客户端目录。
- 通过预设或原始配置添加 MCP 服务器。
- 从支持的客户端导入已有 MCP 配置。
- 在多个客户端间同步 `stdio`、HTTP 和 SSE 服务，同时保留无关配置。
- 连接 MCP 服务查看工具名称、说明、注解以及输入输出参数 Schema。

### 通用设置

- 界面支持简体中文、繁体中文、英语、日语和德语。
- 外观可选择浅色、深色或跟随系统。
- 配置开机启动、静默启动和关闭到系统托盘。
- 在 macOS 上后台检查 GitHub Release，并可从连接状态区升级后自动重启。
- 选择启动命令行客户端时使用的首选终端。
- 控制客户端列表中显示的客户端及其排列顺序。
- Skills 可存储在 AgentDock 数据目录或 `~/.agents/skills`，并使用复制文件或符号链接同步。

### 统计与诊断

- 读取本机 Codex、Claude Code、OpenCode 和 Grok 会话记录。
- 展示 Token、请求次数、可计算成本和 7/30/90 天趋势。
- 按客户端、供应商或模型拆分统计。
- 检查目录权限、客户端安装与更新、供应商连接、MCP 配置和统计数据源。
- 导出不包含已配置密钥值的脱敏诊断报告。

## 支持的客户端

| 客户端 | 检测 | 安装 / 更新 | 供应商 | MCP |
| --- | :---: | :---: | :---: | :---: |
| Codex | 是 | 是 | 是 | 是 |
| Claude Code | 是 | 是 | 是 | 是 |
| Antigravity CLI (Agy) | 是 | 是 | 是 | 是 |
| Grok | 是 | 是 | 是 | 是 |
| OpenCode | 是 | 是 | 是 | 是 |
| OpenClaw | 是 | 是 | 是 | 是 |
| Hermes Agent | 是 | 是 | 是 | 是 |
| Claude Desktop | 是 | 否 | 是 | 是 |

AgentDock 可以检测 Claude Desktop，并向它同步供应商或 MCP 配置，但目前不会下载或卸载 Claude Desktop 本体。

## 下载与安装

Windows、macOS 和 Linux 的版本化预览安装包会以预发布版本形式发布到 [Releases](https://github.com/Cailiang/AgentDock/releases) 页面。成功的 [Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml) 工作流也会保留对应构建产物。

请选择与系统匹配的格式：

- **Windows：** `.msi` 或 `.exe`
- **macOS：** `.dmg` 或 `.app`
- **Linux：** `.deb`、`.rpm` 或 `.AppImage`

预览包可能尚未签名或公证，因此会触发系统安全提示。正式分发需要配置各平台签名证书，不应要求最终用户关闭系统安全功能。

## 数据与安全

- 供应商 API Key 保存在操作系统的 AgentDock 本地配置目录，不会提交到本仓库。
- Unix 系统会限制密钥文件权限；当前预览版尚未接入系统钥匙串或凭据保险库。
- 用量统计直接从本机客户端会话计算，不会由 AgentDock 上传。
- 网络请求仅用于软件元数据与下载、供应商测试与模型发现，以及用户配置的 MCP 连接。
- 诊断导出会移除 API Key、URL 凭据、MCP 环境变量值和请求头值，但仍可能包含系统版本和本机路径，分享前请人工检查。

安全问题报告方式见 [SECURITY.md](SECURITY.md)。

## 本地开发

依赖：

- Node.js 20.19 或更高版本
- Rust stable toolchain
- 当前平台所需的 [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
npm ci
npm run dev
```

构建桌面安装包：

```bash
npm run build
```

运行开发检查：

```bash
npm run build:ui
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

桌面产物位于 `src-tauri/target/release/bundle/`。

## 常见问题

<details>
<summary><strong>用户需要安装 Node.js、npm、Python 或 Rust 吗？</strong></summary>

不需要。这些是开发依赖，不是最终用户依赖。客户端需要运行时时，AgentDock 会下载原生安装包，或在自己的数据目录中准备托管运行时。

</details>

<details>
<summary><strong>为什么不能卸载系统中检测到的客户端？</strong></summary>

AgentDock 只删除安装在自身托管目录中的客户端。系统已有安装不会被修改，避免误删其他安装器或用户管理的软件与文件。

</details>

<details>
<summary><strong>AgentDock 的数据保存在哪里？</strong></summary>

数据保存在当前平台的 AgentDock 应用数据目录和配置目录中。打开软件的 **诊断** 页面，点击 **打开数据目录** 即可定位当前机器实际使用的目录。

</details>

<details>
<summary><strong>AgentDock 会上传 API Key 或使用记录吗？</strong></summary>

不会。当前没有针对这些数据的遥测或上传逻辑。只有在测试或使用供应商时，API Key 才会发送到用户选择的供应商地址。

</details>

## 致谢

AgentDock 的供应商与 MCP 工作流参考了开源项目 [cc-switch](https://github.com/farion1231/cc-switch)。其 MIT 声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

## 许可证

AgentDock 自有源码与资产采用 [MIT](LICENSE) 许可证，Copyright (c) 2026 Cailiang。

第三方客户端名称、Logo 和商标仅用于说明兼容性，不包含在 AgentDock 的 MIT 授权中。详见 [ASSET_NOTICES.md](ASSET_NOTICES.md)。
