<div align="center">

<img src="public/agentdock-logo.svg" width="104" height="104" alt="AgentDock ロゴ">

# AgentDock

### AI コーディングクライアントを簡単に管理するデスクトップハブ

[![Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml/badge.svg)](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-4f6f68)](https://github.com/Cailiang/AgentDock/actions)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-2f5f55)](LICENSE)

[English](README.md) | [简体中文](README_ZH.md) | 日本語 | [Deutsch](README_DE.md)

</div>

AgentDock は、AI コーディングクライアント、プロバイダー、Skills、MCP サーバーのインストールと管理を 1 つのネイティブデスクトップアプリにまとめます。Codex、Claude Code、Grok などを使い始めるために、ランタイムを手動で導入したり、JSON、TOML、環境変数ファイルを編集したりしたくないユーザー向けです。

> AgentDock `0.1.18` は早期プレビュー版です。プロバイダー切り替えや MCP 同期を使用する前に、重要なクライアント設定をバックアップしてください。

## AgentDock を選ぶ理由

AI コーディングクライアントごとに、インストーラー、設定形式、モデルプロトコル、MCP の構造が異なります。経験豊富な開発者には扱えても、初めて使うユーザーには大きな障壁になります。

AgentDock は初心者向けの流れを優先します。

1. すでにインストールされているクライアントを検出します。
2. クライアントをワンクリックでインストールまたは更新します。
3. 公式ログイン、プリセットプロバイダー、またはカスタム互換 API を追加します。
4. 接続をテストし、生成された設定を確認してからクライアントを起動します。

エンドユーザーが Node.js、npm、Python を別途インストールしたり、設定を手動編集したりする必要はありません。必要なランタイムは AgentDock のデータディレクトリ内に用意されます。

## 主な機能

### クライアント管理

- システムインストールと AgentDock 管理インストールを検出します。
- 管理対象クライアントのインストール、更新、起動、アンインストールに対応します。
- 中国本土から利用しやすい npm/PyPI ミラーを優先し、失敗時は公式ソースへフォールバックします。
- OS と CPU アーキテクチャに合うパッケージを自動選択します。
- 配布元がダイジェストまたは npm integrity を公開している場合は整合性を検証します。

### プロバイダー管理

- 対応クライアントごとにプロバイダーを個別管理します。
- プリセット、公式ログイン、完全なカスタムエンドポイントに対応します。
- モデル一覧を取得し、ドロップダウンから既定モデルを選択できます。
- クライアントに応じて OpenAI Responses、Chat Completions、Anthropic Messages、Gemini 互換プロトコルを扱います。
- 接続テスト、設定のプレビューと編集、切り替え、書き込み前のバックアップに対応します。

### Skills と MCP

- Skills のインストールと削除、クライアント別の有効化、実ディレクトリへの同期に対応します。
- プリセットまたは生の設定から MCP サーバーを追加できます。
- 対応クライアントの既存 MCP 設定をインポートできます。
- `stdio`、HTTP、SSE サーバーを、無関係な設定を維持したまま複数クライアントへ同期します。
- MCP サーバーへ接続し、ツール名、説明、注釈、入出力 Schema を確認できます。

### 一般設定

- 簡体字中国語、繁体字中国語、英語、日本語、ドイツ語の表示言語を切り替えます。
- ライト、ダーク、またはシステムに合わせた外観を選択します。
- ログイン時の起動、サイレント起動、閉じたときのトレイ格納を設定します。
- macOS では GitHub Releases をバックグラウンドで確認し、接続ステータスから更新して自動再起動できます。
- コマンドラインクライアントを起動する優先ターミナルを選択します。
- クライアント一覧に表示する項目と並び順を設定します。
- Skills を AgentDock データディレクトリまたは `~/.agents/skills` に保存し、ファイルコピーまたはシンボリックリンクで同期します。

### 使用量と診断

- ローカルの Codex、Claude Code、OpenCode、Grok セッションを読み取ります。
- Token、リクエスト数、計算可能なコスト、7/30/90 日の推移を表示します。
- クライアント、プロバイダー、モデル別に集計できます。
- ディレクトリ権限、インストール、更新、プロバイダー接続、MCP 設定、使用量データを診断します。
- 設定済みシークレット値を除外した診断レポートを出力します。

## 対応クライアント

| クライアント | 検出 | インストール / 更新 | プロバイダー | MCP |
| --- | :---: | :---: | :---: | :---: |
| Codex | はい | はい | はい | はい |
| Claude Code | はい | はい | はい | はい |
| Antigravity CLI (Agy) | はい | はい | はい | はい |
| Grok | はい | はい | はい | はい |
| OpenCode | はい | はい | はい | はい |
| OpenClaw | はい | はい | はい | はい |
| Hermes Agent | はい | はい | はい | はい |
| Claude Desktop | はい | いいえ | はい | はい |

Claude Desktop は検出とプロバイダー/MCP 設定の同期に対応しますが、アプリ本体のダウンロードやアンインストールは行いません。

## ダウンロードとインストール

Windows、macOS、Linux 向けのバージョン付きプレビュー版は、[Releases](https://github.com/Cailiang/AgentDock/releases) ページでプレリリースとして公開されます。成功した [Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml) でも各ビルド成果物を保持します。

- **Windows:** `.msi` または `.exe`
- **macOS:** `.dmg` または `.app`
- **Linux:** `.deb`、`.rpm` または `.AppImage`

プレビュー版は未署名または未公証の場合があり、OS のセキュリティ警告が表示されることがあります。正式配布では各プラットフォームの署名証明書が必要です。

## データとセキュリティ

- API Key はローカルの AgentDock 設定ディレクトリに保存され、このリポジトリには含まれません。
- Unix ではシークレットファイルの権限を制限します。現在のプレビュー版は OS のキーチェーンや資格情報保管庫をまだ使用しません。
- 使用量はローカルのセッションデータから計算され、AgentDock によってアップロードされません。
- ネットワークはソフトウェア情報とダウンロード、プロバイダーテストとモデル取得、ユーザー設定の MCP 接続に使用されます。
- 診断出力は API Key、URL 認証情報、MCP 環境変数値、ヘッダー値を除外しますが、共有前に内容を確認してください。

脆弱性の報告方法は [SECURITY.md](SECURITY.md) を参照してください。

## 開発

必要環境：

- Node.js 20.19 以降
- Rust stable toolchain
- 各プラットフォーム向けの [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
npm ci
npm run dev
```

デスクトップパッケージをビルドします。

```bash
npm run build
```

開発時のチェック：

```bash
npm run build:ui
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

成果物は `src-tauri/target/release/bundle/` に生成されます。

## FAQ

<details>
<summary><strong>Node.js、npm、Python、Rust をユーザーがインストールする必要はありますか？</strong></summary>

ありません。これらは開発時の依存関係です。クライアントに必要なランタイムは AgentDock のデータディレクトリ内に管理されます。

</details>

<details>
<summary><strong>システムで検出されたクライアントをアンインストールできないのはなぜですか？</strong></summary>

AgentDock は自身の管理ディレクトリにインストールしたクライアントだけを削除します。他のインストーラーやユーザーが管理するソフトウェアを誤って削除しないためです。

</details>

<details>
<summary><strong>AgentDock のデータはどこに保存されますか？</strong></summary>

各 OS の AgentDock アプリデータおよび設定ディレクトリです。アプリの **診断** 画面から **データディレクトリを開く** を選ぶと確認できます。

</details>

<details>
<summary><strong>API Key や使用履歴はアップロードされますか？</strong></summary>

されません。これらを収集するテレメトリやアップロード処理は実装されていません。API Key は、ユーザーが選択したプロバイダーのテストまたは利用時にのみ、そのエンドポイントへ送信されます。

</details>

## 謝辞

AgentDock のプロバイダーおよび MCP ワークフローは [cc-switch](https://github.com/farion1231/cc-switch) を参考にしています。MIT 表示は [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。

## ライセンス

AgentDock が所有するソースコードとアセットは [MIT License](LICENSE) で提供されます。Copyright (c) 2026 Cailiang.

第三者クライアントの名称、ロゴ、商標は互換性の表示にのみ使用され、AgentDock の MIT ライセンスには含まれません。詳細は [ASSET_NOTICES.md](ASSET_NOTICES.md) を参照してください。
