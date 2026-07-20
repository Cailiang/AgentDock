<div align="center">

<img src="public/agentdock-logo.svg" width="104" height="104" alt="AgentDock logo">

# AgentDock

### The beginner-friendly desktop hub for AI coding clients

[![Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml/badge.svg)](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-4f6f68)](https://github.com/Cailiang/AgentDock/actions)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-2f5f55)](LICENSE)

English | [简体中文](README_ZH.md) | [日本語](README_JA.md) | [Deutsch](README_DE.md)

</div>

AgentDock installs and manages AI coding clients, providers, Skills, and MCP servers from one native desktop app. It is designed for users who want to start with Codex, Claude Code, Grok, or other agents without manually installing runtimes or editing JSON, TOML, and environment files.

> AgentDock `0.1.20` is an early preview. Keep a backup of important client configuration before using provider switching or MCP synchronization.

## Why AgentDock?

AI coding clients use different installers, configuration formats, model protocols, and MCP layouts. This is manageable for experienced developers, but it creates a steep first-run experience for everyone else.

AgentDock puts the beginner workflow first:

1. Detect what is already installed.
2. Install or update a client with one click.
3. Add an official login, a preset provider, or a custom compatible API.
4. Test the connection, review the generated configuration, and launch the client.

No separate Node.js, npm, Python, or manual configuration is required for end users. AgentDock provisions managed runtimes when a client needs them.

## Core Features

### Client Lifecycle

- Detect system and AgentDock-managed installations.
- Install, update, launch, and uninstall managed clients.
- Prefer mainland-friendly npm/PyPI mirrors, then fall back to official sources.
- Select the package for the current operating system and CPU architecture.
- Verify package integrity when the source publishes a digest or npm integrity value.

### Provider Management

- Manage providers separately for each supported client.
- Start from curated presets, official login, or a fully custom endpoint.
- Fetch provider model lists and choose a default model from a dropdown.
- Support OpenAI Responses, Chat Completions, Anthropic Messages, and Gemini-compatible protocols where applicable.
- Test connectivity, preview and edit generated configuration, switch providers, and back up existing files before writing.

### Skills and MCP

- Install and uninstall Skills, enable them per client, and synchronize real client directories.
- Add MCP servers from presets or raw configuration.
- Import existing MCP configuration from supported clients.
- Synchronize `stdio`, HTTP, and SSE servers across clients without replacing unrelated settings.
- Connect to an MCP server to inspect its tools, descriptions, annotations, and input/output schemas.

### General Settings

- Switch between Simplified Chinese, Traditional Chinese, English, Japanese, and German interface languages.
- Use a light, dark, or system-matched appearance.
- Configure launch at login, silent startup, and minimize-to-tray behavior.
- On macOS, check GitHub Releases in the background, then update and restart from the connection status area.
- Select the preferred terminal used to launch command-line clients.
- Choose which clients appear in the client list and arrange their order.
- Store Skills in AgentDock's data directory or `~/.agents/skills`, then synchronize with file copies or symbolic links.

### Usage and Diagnostics

- Read local Codex, Claude Code, OpenCode, and Grok sessions.
- Show tokens, request counts, calculable cost, and 7/30/90-day trends.
- Break usage down by client, provider, or model.
- Diagnose directory permissions, installations, updates, provider connectivity, MCP configuration, and usage sources.
- Export a sanitized diagnostic report that excludes configured secret values.

## Supported Clients

| Client | Detect | Install / Update | Providers | MCP |
| --- | :---: | :---: | :---: | :---: |
| Codex | Yes | Yes | Yes | Yes |
| Claude Code | Yes | Yes | Yes | Yes |
| Antigravity CLI (Agy) | Yes | Yes | Yes | Yes |
| Grok | Yes | Yes | Yes | Yes |
| OpenCode | Yes | Yes | Yes | Yes |
| OpenClaw | Yes | Yes | Yes | Yes |
| Hermes Agent | Yes | Yes | Yes | Yes |
| Claude Desktop | Yes | No | Yes | Yes |

Claude Desktop is detected and can receive provider or MCP configuration, but AgentDock does not download or uninstall the desktop application itself.

## Download and Installation

Versioned preview packages for Windows, macOS, and Linux are published as prereleases on the [Releases](https://github.com/Cailiang/AgentDock/releases) page. Successful [Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml) runs also retain their build artifacts.

Choose the package for your system:

- **Windows:** `.msi` or `.exe`
- **macOS:** `.dmg` or `.app`
- **Linux:** `.deb`, `.rpm`, or `.AppImage`

Preview builds may be unsigned or not notarized and can trigger an operating-system security warning. Production distribution should use platform signing certificates; users should not be asked to disable system security features.

## Data and Security

- Provider API keys are stored in the local AgentDock configuration directory and are never committed to this repository.
- Secret files receive restrictive permissions on Unix systems. The current preview does not yet use the operating-system keychain or credential vault.
- Usage statistics are calculated from local client session data and are not uploaded by AgentDock.
- Network access is used for software metadata and downloads, provider tests and model discovery, and user-configured MCP connections.
- Diagnostic exports remove API keys, URL credentials, MCP environment values, and header values. They can still contain system versions and local paths, so review them before sharing.

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## Development

Requirements:

- Node.js 20.19 or later
- Rust stable toolchain
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for the current platform

```bash
npm ci
npm run dev
```

Build the desktop packages:

```bash
npm run build
```

Run the checks used during development:

```bash
npm run build:ui
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Desktop bundles are written to `src-tauri/target/release/bundle/`.

## FAQ

<details>
<summary><strong>Do users need to install Node.js, npm, Python, or Rust?</strong></summary>

No. They are development dependencies, not end-user requirements. AgentDock downloads native packages or provisions managed runtimes inside its own data directory when required by a client.

</details>

<details>
<summary><strong>Why can AgentDock not uninstall a client it detected on my system?</strong></summary>

AgentDock only removes clients installed inside its managed directory. Existing system installations are left untouched to avoid deleting software or files owned by another installer.

</details>

<details>
<summary><strong>Where is AgentDock data stored?</strong></summary>

Data is stored in the platform-specific AgentDock application data and configuration directories. Open **Diagnostics** and use **Open data directory** to locate the active directory on the current machine.

</details>

<details>
<summary><strong>Does AgentDock upload API keys or usage history?</strong></summary>

No telemetry or upload path is implemented for those values. A key is sent only to the provider endpoint selected by the user when testing or using that provider.

</details>

## Acknowledgements

AgentDock's provider and MCP workflows were informed by [cc-switch](https://github.com/farion1231/cc-switch). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for its MIT notice.

## License

AgentDock-owned source code and assets are available under the [MIT License](LICENSE), Copyright (c) 2026 Cailiang.

Third-party client names, logos, and trademarks are used only to identify compatibility and are not licensed under AgentDock's MIT License. See [ASSET_NOTICES.md](ASSET_NOTICES.md).
