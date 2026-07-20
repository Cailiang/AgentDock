use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use directories::ProjectDirs;
use flate2::read::GzDecoder;
use futures::future::join_all;
use rmcp::{
    model::Tool,
    transport::{
        sse_client::SseClientConfig, streamable_http_client::StreamableHttpClientTransportConfig,
        SseClientTransport, StreamableHttpClientTransport, TokioChildProcess,
    },
    ServiceExt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, Cursor, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    time::Instant,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);
static CLIENT_DETECTION_CACHE: OnceLock<Mutex<Option<(Instant, Vec<ClientStatus>)>>> =
    OnceLock::new();
const MANAGED_CLI_MARKER: &str = "AgentDock managed CLI shim";
const MANAGED_PATH_BLOCK_START: &str = "# >>> AgentDock CLI >>>";
const MANAGED_PATH_BLOCK_END: &str = "# <<< AgentDock CLI <<<";
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    app_version: String,
    platform: String,
    data_dir: String,
    config_dir: String,
    managed_runtime_ready: bool,
    clients: Vec<ClientStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    supported: bool,
    available: bool,
    current_version: String,
    latest_version: Option<String>,
    download_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAppRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GithubAppAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAppAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Debug, Clone)]
struct SelectedAppUpdate {
    version: String,
    asset: GithubAppAsset,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    language: String,
    theme: String,
    launch_on_startup: bool,
    silent_startup: bool,
    minimize_to_tray_on_close: bool,
    preferred_terminal: String,
    visible_clients: Vec<String>,
    client_order: Vec<String>,
    current_working_directory: String,
    recent_working_directories: Vec<String>,
    skill_storage_location: String,
    skill_sync_method: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let clients = supported_provider_apps()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        Self {
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            launch_on_startup: false,
            silent_startup: false,
            minimize_to_tray_on_close: false,
            preferred_terminal: default_terminal().to_string(),
            visible_clients: clients.clone(),
            client_order: clients,
            current_working_directory: String::new(),
            recent_working_directories: Vec::new(),
            skill_storage_location: "agentdock".to_string(),
            skill_sync_method: "copy".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    id: String,
    name: String,
    installed: bool,
    version: Option<String>,
    executable: Option<String>,
    config_path: Option<String>,
    managed_by_agentdock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareCatalogItem {
    id: String,
    client_id: String,
    name: String,
    description: String,
    publisher: String,
    website_url: String,
    category: String,
    recommended: bool,
    installed: bool,
    current_version: Option<String>,
    latest_version: Option<String>,
    update_available: bool,
    install_supported: bool,
    managed_by_agentdock: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SoftwareCatalogSeed {
    id: String,
    client_id: String,
    name: String,
    description: String,
    publisher: String,
    website_url: String,
    category: String,
    #[serde(default)]
    recommended: bool,
    #[serde(default)]
    install_supported: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteSoftwareCatalog {
    items: Vec<SoftwareCatalogSeed>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelsResult {
    models: Vec<String>,
    source: String,
    provider_supplied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderModelProtocol {
    OpenAi,
    Gemini,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedProviderConfig {
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    api_format: Option<String>,
    source_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    id: String,
    name: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    website_url: String,
    #[serde(default)]
    preset_id: String,
    provider_type: String,
    base_url: String,
    api_format: String,
    #[serde(default)]
    settings_config: String,
    enabled_apps: Vec<String>,
    codex_model: String,
    #[serde(default = "default_gemini_model")]
    gemini_model: String,
    claude_sonnet_model: String,
    claude_haiku_model: String,
    claude_opus_model: String,
    active: bool,
    #[serde(default)]
    active_apps: Vec<String>,
    #[serde(default)]
    api_key_configured: bool,
    #[serde(default)]
    activation_reviewed: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    id: Option<String>,
    name: String,
    notes: Option<String>,
    website_url: Option<String>,
    preset_id: Option<String>,
    provider_type: String,
    base_url: String,
    api_format: Option<String>,
    settings_config: Option<String>,
    enabled_apps: Option<Vec<String>>,
    codex_model: Option<String>,
    gemini_model: Option<String>,
    claude_sonnet_model: Option<String>,
    claude_haiku_model: Option<String>,
    claude_opus_model: Option<String>,
    active: Option<bool>,
    active_apps: Option<Vec<String>>,
    api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSwitchImportDetection {
    available: bool,
    source_path: String,
    source_kind: String,
    fingerprint: String,
    provider_count: usize,
    app_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CcSwitchImportResult {
    imported: usize,
    updated: usize,
    skipped: usize,
    app_counts: BTreeMap<String, usize>,
    errors: Vec<String>,
    backup_dir: String,
}

#[derive(Debug, Clone)]
struct CcSwitchProviderCandidate {
    source_id: String,
    source_app: String,
    app_id: String,
    name: String,
    settings_config: serde_json::Value,
    website_url: String,
    category: String,
    notes: String,
    meta: serde_json::Value,
    is_current: bool,
}

#[derive(Debug, Clone)]
struct PreparedCcSwitchProvider {
    provider: ProviderProfile,
    api_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyCheck {
    score: u8,
    blockers: Vec<String>,
    warnings: Vec<String>,
    clients_ready: usize,
    providers_ready: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPreview {
    codex_toml: String,
    claude_env_json: String,
    gemini_env_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedClientRecord {
    id: String,
    name: String,
    installed: bool,
    version: String,
    install_dir: String,
    launcher_path: String,
    config_dir: String,
    installed_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallClientResult {
    client: ManagedClientRecord,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResult {
    ok: bool,
    latency_ms: u128,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProviderResult {
    provider_id: String,
    backup_dir: String,
    written_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    id: String,
    category: String,
    status: String,
    title: String,
    detail: String,
    action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    generated_at: String,
    score: u8,
    passed: usize,
    warnings: usize,
    failed: usize,
    checks: Vec<DiagnosticCheck>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchClientResult {
    launched: bool,
    message: String,
    client_id: String,
    working_directory: Option<String>,
    request_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    written_files: Vec<String>,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    id: String,
    name: String,
    description: String,
    source: String,
    installed: bool,
    apps: Vec<String>,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstallInput {
    id: String,
    name: Option<String>,
    description: Option<String>,
    source: Option<String>,
    apps: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRecord {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    docs: String,
    #[serde(default)]
    tags: Vec<String>,
    transport: String,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    extra: BTreeMap<String, serde_json::Value>,
    apps: Vec<String>,
    enabled: bool,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInput {
    id: String,
    name: Option<String>,
    description: Option<String>,
    homepage: Option<String>,
    docs: Option<String>,
    tags: Option<Vec<String>>,
    transport: Option<String>,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<BTreeMap<String, String>>,
    headers: Option<BTreeMap<String, String>>,
    cwd: Option<String>,
    extra: Option<BTreeMap<String, serde_json::Value>>,
    apps: Option<Vec<String>>,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImportResult {
    imported: usize,
    linked: usize,
    scanned_apps: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    name: String,
    title: Option<String>,
    description: String,
    input_schema: serde_json::Value,
    output_schema: Option<serde_json::Value>,
    annotations: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolsResult {
    server_id: String,
    server_name: String,
    transport: String,
    tools: Vec<McpToolInfo>,
    latency_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    total_tokens: u64,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    requests: u64,
    cost_usd: f64,
    unpriced_requests: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTrendPoint {
    date: String,
    total_tokens: u64,
    requests: u64,
    cost_usd: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageBreakdownItem {
    id: String,
    name: String,
    total_tokens: u64,
    requests: u64,
    cost_usd: f64,
    share: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    days: u32,
    from: String,
    to: String,
    summary: UsageSummary,
    trend: Vec<UsageTrendPoint>,
    by_client: Vec<UsageBreakdownItem>,
    by_provider: Vec<UsageBreakdownItem>,
    by_model: Vec<UsageBreakdownItem>,
    sources: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct UsageRecord {
    timestamp: OffsetDateTime,
    client: String,
    provider: String,
    model: String,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
    cost_usd: Option<f64>,
}

fn desktop_status() -> Result<DesktopStatus, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;

    Ok(DesktopStatus {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: env::consts::OS.to_string(),
        data_dir: dirs.data_dir.display().to_string(),
        config_dir: dirs.config_dir.display().to_string(),
        managed_runtime_ready: dirs.runtime_dir.exists(),
        clients: refresh_client_detection(),
    })
}

#[tauri::command]
async fn get_desktop_status() -> Result<DesktopStatus, String> {
    tauri::async_runtime::spawn_blocking(desktop_status)
        .await
        .map_err(|error| format!("客户端检测任务失败: {}", error))?
}

#[tauri::command]
async fn check_app_update() -> Result<AppUpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    if env::consts::OS != "macos" {
        return Ok(AppUpdateInfo {
            supported: false,
            available: false,
            current_version,
            latest_version: None,
            download_size: None,
        });
    }

    let update = fetch_available_app_update(&current_version, env::consts::ARCH).await?;
    Ok(AppUpdateInfo {
        supported: true,
        available: update.is_some(),
        current_version,
        latest_version: update.as_ref().map(|item| item.version.clone()),
        download_size: update.as_ref().map(|item| item.asset.size),
    })
}

#[tauri::command]
async fn install_app_update(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("AgentDock 自动升级目前仅支持 macOS".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let current_version = env!("CARGO_PKG_VERSION");
        let update = fetch_available_app_update(current_version, env::consts::ARCH)
            .await?
            .ok_or_else(|| "当前已经是最新版本".to_string())?;
        let app_path = current_app_bundle_path()?;
        ensure_app_parent_writable(&app_path)?;

        let client = app_update_http_client()?;
        let bytes = download_bytes(&client, &update.asset.browser_download_url).await?;
        if update.asset.size > 0 && bytes.len() as u64 != update.asset.size {
            return Err("升级包大小与 GitHub 发布信息不一致，已停止升级".to_string());
        }
        verify_sha256(&bytes, &update.sha256)?;

        let dmg_path = app_update_temp_path("dmg");
        write_private_file(&dmg_path, &bytes)?;
        let helper_path = app_update_temp_path("sh");
        if let Err(error) = write_private_file(&helper_path, app_update_helper_script().as_bytes())
        {
            let _ = fs::remove_file(&dmg_path);
            return Err(error);
        }
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(&helper_path, fs::Permissions::from_mode(0o700)) {
            let _ = fs::remove_file(&dmg_path);
            let _ = fs::remove_file(&helper_path);
            return Err(format!("无法设置升级程序权限: {}", error));
        }

        let spawn_result = Command::new("/bin/sh")
            .arg(&helper_path)
            .arg(std::process::id().to_string())
            .arg(&dmg_path)
            .arg(&app_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        if let Err(error) = spawn_result {
            let _ = fs::remove_file(&dmg_path);
            let _ = fs::remove_file(&helper_path);
            return Err(format!("无法启动升级程序: {}", error));
        }

        app.exit(0);
        Ok(())
    }
}

fn app_update_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(180))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("无法创建升级请求: {}", error))
}

async fn fetch_available_app_update(
    current_version: &str,
    architecture: &str,
) -> Result<Option<SelectedAppUpdate>, String> {
    let client = app_update_http_client()?;
    let releases = client
        .get("https://api.github.com/repos/Cailiang/AgentDock/releases?per_page=20")
        .send()
        .await
        .map_err(|error| format!("检查 AgentDock 更新失败: {}", error))?
        .error_for_status()
        .map_err(|error| format!("检查 AgentDock 更新失败: {}", error))?
        .json::<Vec<GithubAppRelease>>()
        .await
        .map_err(|error| format!("读取 AgentDock 发布信息失败: {}", error))?;
    select_app_update(&releases, current_version, "macos", architecture)
}

fn select_app_update(
    releases: &[GithubAppRelease],
    current_version: &str,
    operating_system: &str,
    architecture: &str,
) -> Result<Option<SelectedAppUpdate>, String> {
    let release = releases
        .iter()
        .filter(|release| !release.draft)
        .filter(|release| version_is_newer(&release.tag_name, current_version))
        .max_by(|left, right| {
            version_numbers(&left.tag_name).cmp(&version_numbers(&right.tag_name))
        });
    let Some(release) = release else {
        return Ok(None);
    };

    let version = normalized_release_version(&release.tag_name);
    let expected_asset = app_update_asset_name(&version, operating_system, architecture)?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == expected_asset)
        .cloned()
        .ok_or_else(|| format!("新版本 {} 缺少当前架构的安装包 {}", version, expected_asset))?;
    let sha256 = github_asset_sha256(&asset)?.to_string();
    Ok(Some(SelectedAppUpdate {
        version,
        asset,
        sha256,
    }))
}

fn normalized_release_version(tag: &str) -> String {
    tag.trim()
        .trim_start_matches(|character| character == 'v' || character == 'V')
        .to_string()
}

fn app_update_asset_name(
    version: &str,
    operating_system: &str,
    architecture: &str,
) -> Result<String, String> {
    let suffix = match (operating_system, architecture) {
        ("macos", "aarch64" | "x86_64") => "universal.dmg",
        (os, arch) => {
            return Err(format!(
                "AgentDock 自动升级暂不支持当前平台: {} {}",
                os, arch
            ))
        }
    };
    Ok(format!("AgentDock_{}_{}", version, suffix))
}

fn github_asset_sha256(asset: &GithubAppAsset) -> Result<&str, String> {
    let digest = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .ok_or_else(|| format!("安装包 {} 缺少 GitHub SHA-256 摘要", asset.name))?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("安装包 {} 的 SHA-256 摘要无效", asset.name));
    }
    Ok(digest)
}

#[cfg(target_os = "macos")]
fn current_app_bundle_path() -> Result<PathBuf, String> {
    let executable =
        env::current_exe().map_err(|error| format!("无法定位 AgentDock: {}", error))?;
    executable
        .ancestors()
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("app"))
        .map(Path::to_path_buf)
        .ok_or_else(|| "请先将 AgentDock.app 安装到“应用程序”目录后再自动升级".to_string())
}

#[cfg(target_os = "macos")]
fn ensure_app_parent_writable(app_path: &Path) -> Result<(), String> {
    let parent = app_path
        .parent()
        .ok_or_else(|| "无法定位 AgentDock 所在目录".to_string())?;
    let probe = parent.join(format!(
        ".agentdock-update-write-test-{}-{}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            Ok(())
        }
        Err(error) => Err(format!(
            "AgentDock 所在目录不可写，无法自动升级。请确认当前用户可以修改 {}：{}",
            parent.display(),
            error
        )),
    }
}

#[cfg(target_os = "macos")]
fn app_update_temp_path(extension: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "agentdock-update-{}-{}.{}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos(),
        extension
    ))
}

#[cfg(target_os = "macos")]
fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("无法创建升级临时文件 {}: {}", path.display(), error))?;
    file.write_all(bytes)
        .map_err(|error| format!("无法写入升级临时文件 {}: {}", path.display(), error))?;
    file.sync_all()
        .map_err(|error| format!("无法保存升级临时文件 {}: {}", path.display(), error))
}

#[cfg(any(target_os = "macos", test))]
fn app_update_helper_script() -> &'static str {
    r#"#!/bin/sh
set -u

APP_PID="$1"
DMG_PATH="$2"
APP_PATH="$3"
SCRIPT_PATH="$0"
MOUNT_POINT="$(/usr/bin/mktemp -d /tmp/agentdock-update-mount.XXXXXX)" || exit 1
STAGE_PATH="${APP_PATH}.update.$$"
BACKUP_PATH="${APP_PATH}.backup.$$"
LOG_PATH="${TMPDIR:-/tmp}/agentdock-updater.log"
MOUNTED=0

exec >>"$LOG_PATH" 2>&1

cleanup() {
  STATUS=$?
  trap - EXIT HUP INT TERM
  if [ "$MOUNTED" -eq 1 ]; then
    /usr/bin/hdiutil detach "$MOUNT_POINT" -quiet || true
  fi
  /bin/rm -rf "$MOUNT_POINT" "$STAGE_PATH"
  /bin/rm -f "$DMG_PATH" "$SCRIPT_PATH"
  exit "$STATUS"
}

reopen_or_rollback() {
  MESSAGE="$1"
  echo "$MESSAGE"
  if [ ! -d "$APP_PATH" ] && [ -d "$BACKUP_PATH" ]; then
    /bin/mv "$BACKUP_PATH" "$APP_PATH" || true
  fi
  if [ -d "$APP_PATH" ]; then
    /usr/bin/open "$APP_PATH" || true
  fi
  exit 1
}

trap cleanup EXIT HUP INT TERM

ATTEMPT=0
while /bin/kill -0 "$APP_PID" 2>/dev/null; do
  ATTEMPT=$((ATTEMPT + 1))
  if [ "$ATTEMPT" -gt 600 ]; then
    reopen_or_rollback "Timed out waiting for AgentDock to exit"
  fi
  /bin/sleep 0.2
done

if ! /usr/bin/hdiutil attach "$DMG_PATH" -nobrowse -readonly -mountpoint "$MOUNT_POINT"; then
  reopen_or_rollback "Could not mount the AgentDock update"
fi
MOUNTED=1

SOURCE_APP="$(/usr/bin/find "$MOUNT_POINT" -maxdepth 2 -type d -name AgentDock.app -print -quit)"
if [ -z "$SOURCE_APP" ]; then
  reopen_or_rollback "The update does not contain AgentDock.app"
fi
if ! /usr/bin/ditto "$SOURCE_APP" "$STAGE_PATH"; then
  reopen_or_rollback "Could not stage the AgentDock update"
fi
/usr/bin/xattr -dr com.apple.quarantine "$STAGE_PATH" 2>/dev/null || true

if ! /bin/mv "$APP_PATH" "$BACKUP_PATH"; then
  reopen_or_rollback "Could not back up the current AgentDock app"
fi
if ! /bin/mv "$STAGE_PATH" "$APP_PATH"; then
  reopen_or_rollback "Could not install the new AgentDock app"
fi
if ! /usr/bin/open "$APP_PATH"; then
  /bin/rm -rf "$APP_PATH"
  /bin/mv "$BACKUP_PATH" "$APP_PATH" || true
  /usr/bin/open "$APP_PATH" || true
  exit 1
fi

/bin/rm -rf "$BACKUP_PATH"
exit 0
"#
}

#[tauri::command]
fn get_app_settings() -> Result<AppSettings, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    read_app_settings(&dirs)
}

#[tauri::command]
fn save_app_settings(settings: AppSettings) -> Result<AppSettings, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let previous = read_app_settings(&dirs)?;
    let settings = normalize_app_settings(settings);

    if previous.skill_storage_location != settings.skill_storage_location {
        migrate_skill_storage(&dirs, &previous, &settings)?;
    }
    if previous.launch_on_startup != settings.launch_on_startup {
        set_auto_launch_enabled(settings.launch_on_startup)?;
    }

    write_json(&app_settings_path(&dirs), &settings)?;
    Ok(settings)
}

#[tauri::command]
async fn list_software_catalog() -> Result<Vec<SoftwareCatalogItem>, String> {
    let statuses = cached_client_detection();
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建软件目录请求失败: {}", err))?;
    let seeds = load_software_catalog_seeds(&client).await;

    let checks = seeds.into_iter().map(|seed| {
        let client = client.clone();
        let status = statuses
            .iter()
            .find(|status| status.id == seed.client_id)
            .cloned();
        async move {
            let latest_version = latest_client_version(&client, &seed.client_id)
                .await
                .ok()
                .flatten();
            let current_version = status.as_ref().and_then(|status| status.version.clone());
            let update_available = match (&latest_version, &current_version) {
                (Some(latest), Some(current)) => version_is_newer(latest, current),
                _ => false,
            };
            SoftwareCatalogItem {
                id: seed.id,
                client_id: seed.client_id,
                name: seed.name,
                description: seed.description,
                publisher: seed.publisher,
                website_url: seed.website_url,
                category: seed.category,
                recommended: seed.recommended,
                installed: status
                    .as_ref()
                    .map(|status| status.installed)
                    .unwrap_or(false),
                current_version,
                latest_version,
                update_available,
                install_supported: seed.install_supported,
                managed_by_agentdock: status
                    .as_ref()
                    .map(|status| status.managed_by_agentdock)
                    .unwrap_or(false),
            }
        }
    });

    Ok(join_all(checks).await)
}

fn strip_config_code_fence(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let Some(first_newline) = trimmed.find('\n') else {
        return trimmed.to_string();
    };
    let body = &trimmed[first_newline + 1..];
    body.strip_suffix("```").unwrap_or(body).trim().to_string()
}

fn clean_imported_value(value: &str) -> Option<String> {
    let value = value
        .trim()
        .trim_end_matches(',')
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '`'))
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn clean_imported_api_key(value: &str) -> Option<String> {
    let value = clean_imported_value(value)?;
    let lowered = value.to_ascii_lowercase();
    if value.contains("${")
        || value.starts_with('$')
        || lowered.contains("your_api_key")
        || lowered.contains("replace_me")
        || lowered == "api-key"
    {
        return None;
    }
    Some(value)
}

fn normalized_config_key(key: &str) -> String {
    key.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalized_imported_api_format(value: &str) -> Option<String> {
    let normalized = normalized_config_key(value);
    let format = match normalized.as_str() {
        "responses" | "openairesponses" => "responses",
        "chatcompletions" | "openaicompletions" | "openai" => "chat-completions",
        "messages" | "anthropic" | "anthropicmessages" => "anthropic",
        "gemini" | "googlegemini" | "gemininative" => "gemini",
        _ => return None,
    };
    Some(format.to_string())
}

fn apply_imported_entry(result: &mut ParsedProviderConfig, key: &str, value: &str) {
    let key = normalized_config_key(key);
    match key.as_str() {
        "baseurl"
        | "apibase"
        | "apiurl"
        | "endpoint"
        | "anthropicbaseurl"
        | "openaibaseurl"
        | "xaibaseurl"
        | "geminibaseurl"
        | "googlegeminibaseurl" => {
            if result.base_url.is_none() {
                let value = clean_imported_value(value);
                if value.as_deref().is_some_and(|value| {
                    value.starts_with("http://") || value.starts_with("https://")
                }) {
                    result.base_url = value;
                }
            }
        }
        "apikey" | "openaikey" | "openaiapikey" | "anthropicauthtoken" | "anthropicapikey"
        | "geminiapikey" | "googleapikey" | "xaiapikey" | "authtoken" | "accesstoken" => {
            if result.api_key.is_none() {
                result.api_key = clean_imported_api_key(value);
            }
        }
        "model" | "defaultmodel" | "anthropicmodel" | "geminimodel" | "grokdefaultmodel" => {
            if result.model.is_none() {
                result.model = clean_imported_value(value);
            }
        }
        "apibackend" | "wireapi" | "apiformat" | "apimode" | "api" => {
            if result.api_format.is_none() {
                result.api_format = normalized_imported_api_format(value);
            }
        }
        _ => {}
    }
}

fn json_string_at<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn apply_json_paths(
    result: &mut ParsedProviderConfig,
    value: &serde_json::Value,
    paths: &[(&str, &[&str])],
) {
    for (key, path) in paths {
        if let Some(value) = json_string_at(value, path) {
            apply_imported_entry(result, key, value);
        }
    }
}

fn walk_json_config(value: &serde_json::Value, result: &mut ParsedProviderConfig) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if let Some(value) = value.as_str() {
                    apply_imported_entry(result, key, value);
                }
                walk_json_config(value, result);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                walk_json_config(value, result);
            }
        }
        _ => {}
    }
}

fn merge_parsed_config(result: &mut ParsedProviderConfig, parsed: ParsedProviderConfig) {
    if result.base_url.is_none() {
        result.base_url = parsed.base_url;
    }
    if result.api_key.is_none() {
        result.api_key = parsed.api_key;
    }
    if result.model.is_none() {
        result.model = parsed.model;
    }
    if result.api_format.is_none()
        && (result.base_url.is_some() || result.api_key.is_some() || result.model.is_some())
    {
        result.api_format = parsed.api_format;
    }
}

fn parse_json_provider_config(
    app_id: &str,
    value: &serde_json::Value,
    result: &mut ParsedProviderConfig,
) {
    match app_id {
        "claude-code" | "claude-desktop" => apply_json_paths(
            result,
            value,
            &[
                ("base_url", &["env", "ANTHROPIC_BASE_URL"]),
                ("api_key", &["env", "ANTHROPIC_AUTH_TOKEN"]),
                ("api_key", &["env", "ANTHROPIC_API_KEY"]),
                ("model", &["env", "ANTHROPIC_MODEL"]),
            ],
        ),
        "antigravity" => apply_json_paths(
            result,
            value,
            &[
                ("base_url", &["env", "GOOGLE_GEMINI_BASE_URL"]),
                ("api_key", &["env", "GEMINI_API_KEY"]),
                ("api_key", &["env", "GOOGLE_API_KEY"]),
                ("model", &["env", "GEMINI_MODEL"]),
            ],
        ),
        "codex" => {
            apply_json_paths(
                result,
                value,
                &[
                    ("api_key", &["OPENAI_API_KEY"]),
                    ("api_key", &["auth", "OPENAI_API_KEY"]),
                ],
            );
            if let Some(config) = json_string_at(value, &["config"]) {
                let mut nested = ParsedProviderConfig::default();
                if parse_toml_provider_config(app_id, config, &mut nested) {
                    merge_parsed_config(result, nested);
                }
            }
        }
        "grok" => {
            apply_json_paths(
                result,
                value,
                &[
                    ("api_key", &["env", "XAI_API_KEY"]),
                    ("api_key", &["XAI_API_KEY"]),
                ],
            );
            if let Some(config) = json_string_at(value, &["config"]) {
                let mut nested = ParsedProviderConfig::default();
                if parse_toml_provider_config(app_id, config, &mut nested) {
                    merge_parsed_config(result, nested);
                }
            }
        }
        "opencode" => {
            if let Some(model) = json_string_at(value, &["model"]) {
                result.model = clean_imported_value(model.rsplit('/').next().unwrap_or(model));
            }
            if let Some(providers) = value.get("provider").and_then(serde_json::Value::as_object) {
                let selected_id = json_string_at(value, &["model"])
                    .and_then(|model| model.split('/').next())
                    .filter(|provider| providers.contains_key(*provider));
                let selected = selected_id
                    .and_then(|provider| providers.get(provider))
                    .or_else(|| providers.values().next());
                if let Some(selected) = selected {
                    apply_json_paths(
                        result,
                        selected,
                        &[
                            ("base_url", &["options", "baseURL"]),
                            ("base_url", &["options", "baseUrl"]),
                            ("api_key", &["options", "apiKey"]),
                        ],
                    );
                    if result.model.is_none() {
                        result.model = selected
                            .get("models")
                            .and_then(serde_json::Value::as_object)
                            .and_then(|models| models.keys().next().cloned());
                    }
                }
            }
        }
        "openclaw" => {
            apply_json_paths(
                result,
                value,
                &[
                    ("base_url", &["baseUrl"]),
                    ("api_key", &["apiKey"]),
                    ("api", &["api"]),
                ],
            );
            if let Some(model) = value
                .get("models")
                .and_then(serde_json::Value::as_array)
                .and_then(|models| models.first())
                .and_then(|model| model.get("id"))
                .and_then(serde_json::Value::as_str)
            {
                result.model = clean_imported_value(model);
            }
            if let Some(providers) = value
                .pointer("/models/providers")
                .and_then(serde_json::Value::as_object)
            {
                let primary = value
                    .pointer("/agents/defaults/model/primary")
                    .and_then(serde_json::Value::as_str);
                let provider_id = primary
                    .and_then(|model| model.split('/').next())
                    .filter(|provider| providers.contains_key(*provider));
                let selected = provider_id
                    .and_then(|provider| providers.get(provider))
                    .or_else(|| providers.values().next());
                if let Some(selected) = selected {
                    apply_json_paths(
                        result,
                        selected,
                        &[
                            ("base_url", &["baseUrl"]),
                            ("api_key", &["apiKey"]),
                            ("api", &["api"]),
                        ],
                    );
                    if let Some(model) =
                        primary.and_then(|model| model.split_once('/').map(|(_, model)| model))
                    {
                        result.model = clean_imported_value(model);
                    }
                }
            }
        }
        "hermes" => apply_json_paths(
            result,
            value,
            &[
                ("base_url", &["base_url"]),
                ("api_key", &["api_key"]),
                ("model", &["model"]),
                ("api_mode", &["api_mode"]),
            ],
        ),
        _ => {}
    }
    walk_json_config(value, result);
}

fn walk_toml_config(value: &toml::Value, result: &mut ParsedProviderConfig) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                if let Some(value) = value.as_str() {
                    apply_imported_entry(result, key, value);
                }
                walk_toml_config(value, result);
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                walk_toml_config(value, result);
            }
        }
        _ => {}
    }
}

fn parse_toml_provider_config(
    app_id: &str,
    content: &str,
    result: &mut ParsedProviderConfig,
) -> bool {
    let Ok(root) = content.parse::<toml::Table>() else {
        return false;
    };
    if root.is_empty() {
        return false;
    }

    if app_id == "grok" {
        let alias = root
            .get("models")
            .and_then(toml::Value::as_table)
            .and_then(|models| models.get("default"))
            .and_then(toml::Value::as_str);
        if let Some(profile) = alias
            .and_then(|alias| {
                root.get("model")
                    .and_then(toml::Value::as_table)
                    .and_then(|models| models.get(alias))
            })
            .and_then(toml::Value::as_table)
        {
            for key in ["base_url", "api_key", "model", "api_backend"] {
                if let Some(value) = profile.get(key).and_then(toml::Value::as_str) {
                    apply_imported_entry(result, key, value);
                }
            }
        }
    }

    if app_id == "codex" {
        if let Some(model) = root.get("model").and_then(toml::Value::as_str) {
            apply_imported_entry(result, "model", model);
        }
        let provider_id = root.get("model_provider").and_then(toml::Value::as_str);
        if let Some(provider) = provider_id
            .and_then(|provider| {
                root.get("model_providers")
                    .and_then(toml::Value::as_table)
                    .and_then(|providers| providers.get(provider))
            })
            .and_then(toml::Value::as_table)
        {
            for key in ["base_url", "api_key", "wire_api"] {
                if let Some(value) = provider.get(key).and_then(toml::Value::as_str) {
                    apply_imported_entry(result, key, value);
                }
            }
        }
    }

    walk_toml_config(&toml::Value::Table(root), result);
    true
}

fn parse_assignment_config(content: &str, result: &mut ParsedProviderConfig) -> bool {
    let mut recognized = false;
    for line in content.lines() {
        let line = line.trim().trim_start_matches("export ").trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let pair = line.split_once('=').or_else(|| line.split_once(':'));
        let Some((key, value)) = pair else {
            continue;
        };
        let before = (
            result.base_url.is_some(),
            result.api_key.is_some(),
            result.model.is_some(),
            result.api_format.is_some(),
        );
        apply_imported_entry(result, key, value);
        let after = (
            result.base_url.is_some(),
            result.api_key.is_some(),
            result.model.is_some(),
            result.api_format.is_some(),
        );
        recognized |= before != after;
    }
    recognized
}

fn parse_provider_config_text(app_id: &str, content: &str) -> Result<ParsedProviderConfig, String> {
    if !supported_provider_apps().contains(&app_id) {
        return Err("不支持这个客户端的供应商配置".to_string());
    }
    if content.len() > 1_000_000 {
        return Err("配置内容过大，请粘贴单个客户端配置文件".to_string());
    }
    let content = strip_config_code_fence(content);
    if content.is_empty() {
        return Err("配置内容不能为空".to_string());
    }

    let mut result = ParsedProviderConfig::default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
        if value.is_object() {
            parse_json_provider_config(app_id, &value, &mut result);
            result.source_format = "JSON".to_string();
        }
    }
    if result.source_format.is_empty() && parse_toml_provider_config(app_id, &content, &mut result)
    {
        result.source_format = "TOML".to_string();
    }
    if result.source_format.is_empty() {
        if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Ok(value) = serde_json::to_value(value) {
                if value.is_object() {
                    parse_json_provider_config(app_id, &value, &mut result);
                    result.source_format = "YAML".to_string();
                }
            }
        }
    }
    let assignment_recognized = parse_assignment_config(&content, &mut result);
    if result.source_format.is_empty() && assignment_recognized {
        result.source_format = "环境变量".to_string();
    }
    if result.api_format.is_none() {
        result.api_format = match app_id {
            "claude-code" | "claude-desktop" => Some("anthropic".to_string()),
            "antigravity" => Some("gemini".to_string()),
            _ => None,
        };
    }
    if result.base_url.is_none()
        && result.api_key.is_none()
        && result.model.is_none()
        && result.api_format.is_none()
    {
        return Err("没有识别到请求地址、API Key、模型或 API 协议".to_string());
    }
    Ok(result)
}

#[tauri::command]
fn parse_provider_config(app_id: String, content: String) -> Result<ParsedProviderConfig, String> {
    parse_provider_config_text(&app_id, &content)
}

fn cc_switch_app_id(source_app: &str) -> Option<&'static str> {
    match source_app {
        "claude" => Some("claude-code"),
        "claude-desktop" => Some("claude-desktop"),
        "codex" => Some("codex"),
        "gemini" => Some("antigravity"),
        "grokbuild" | "grok" => Some("grok"),
        "opencode" => Some("opencode"),
        "openclaw" => Some("openclaw"),
        "hermes" => Some("hermes"),
        _ => None,
    }
}

fn cc_switch_source() -> Option<(PathBuf, String)> {
    let config_dir = dirs_home()?.join(".cc-switch");
    let database = config_dir.join("cc-switch.db");
    if database.is_file() {
        return Some((database, "SQLite".to_string()));
    }
    let legacy = config_dir.join("config.json");
    legacy.is_file().then(|| (legacy, "JSON".to_string()))
}

fn cc_switch_source_fingerprint(path: &Path) -> Result<String, String> {
    let mut paths = vec![path.to_path_buf()];
    if path.extension().and_then(|value| value.to_str()) == Some("db") {
        paths.push(path.with_extension("db-wal"));
    }
    let mut identity = String::new();
    for item in paths {
        let Ok(metadata) = fs::metadata(&item) else {
            continue;
        };
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        identity.push_str(&format!(
            "{}:{}:{};",
            item.display(),
            metadata.len(),
            modified
        ));
    }
    if identity.is_empty() {
        return Err("读取 cc-switch 配置状态失败".to_string());
    }
    Ok(format!("{:x}", Sha256::digest(identity.as_bytes())))
}

fn cc_switch_display_path(path: &Path) -> String {
    path.file_name()
        .map(|name| format!("~/.cc-switch/{}", name.to_string_lossy()))
        .unwrap_or_else(|| "~/.cc-switch".to_string())
}

fn load_cc_switch_database(path: &Path) -> Result<Vec<CcSwitchProviderCandidate>, String> {
    use rusqlite::{Connection, OpenFlags};

    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| format!("只读打开 cc-switch 数据库失败: {}", err))?;
    let mut statement = connection
        .prepare(
            "SELECT id, app_type, name, settings_config, website_url, category, notes, meta, is_current
             FROM providers
             ORDER BY app_type, COALESCE(sort_index, 999999), created_at, id",
        )
        .map_err(|err| format!("读取 cc-switch 供应商表失败: {}", err))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                row.get::<_, String>(7).unwrap_or_else(|_| "{}".to_string()),
                row.get::<_, bool>(8)?,
            ))
        })
        .map_err(|err| format!("查询 cc-switch 供应商失败: {}", err))?;

    let mut candidates = Vec::new();
    for row in rows {
        let (source_id, source_app, name, settings, website_url, category, notes, meta, is_current) =
            row.map_err(|err| format!("读取 cc-switch 供应商记录失败: {}", err))?;
        let Some(app_id) = cc_switch_app_id(&source_app) else {
            continue;
        };
        let settings_config = serde_json::from_str(&settings)
            .map_err(|err| format!("{} 的配置不是有效 JSON: {}", name, err))?;
        let meta = serde_json::from_str(&meta).unwrap_or_else(|_| serde_json::json!({}));
        candidates.push(CcSwitchProviderCandidate {
            source_id,
            source_app,
            app_id: app_id.to_string(),
            name,
            settings_config,
            website_url,
            category,
            notes,
            meta,
            is_current,
        });
    }
    Ok(candidates)
}

fn load_cc_switch_legacy_json(path: &Path) -> Result<Vec<CcSwitchProviderCandidate>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取 cc-switch config.json 失败: {}", err))?;
    if raw.len() > 20_000_000 {
        return Err("cc-switch config.json 过大，已停止导入".to_string());
    }
    let root: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|err| format!("解析 cc-switch config.json 失败: {}", err))?;
    let apps = root.get("apps").unwrap_or(&root);
    let mut candidates = Vec::new();
    for source_app in [
        "claude",
        "claude-desktop",
        "codex",
        "gemini",
        "grokbuild",
        "opencode",
        "openclaw",
        "hermes",
    ] {
        let Some(manager) = apps.get(source_app) else {
            continue;
        };
        let current = manager
            .get("current")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let Some(providers) = manager
            .get("providers")
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        for (source_id, value) in providers {
            let name = value
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(source_id)
                .to_string();
            candidates.push(CcSwitchProviderCandidate {
                source_id: source_id.clone(),
                source_app: source_app.to_string(),
                app_id: cc_switch_app_id(source_app)
                    .unwrap_or(source_app)
                    .to_string(),
                name,
                settings_config: value
                    .get("settingsConfig")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                website_url: value
                    .get("websiteUrl")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                category: value
                    .get("category")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                notes: value
                    .get("notes")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                meta: value
                    .get("meta")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                is_current: current == source_id,
            });
        }
    }
    Ok(candidates)
}

fn load_cc_switch_candidates(
) -> Result<Option<(PathBuf, String, Vec<CcSwitchProviderCandidate>)>, String> {
    let Some((path, source_kind)) = cc_switch_source() else {
        return Ok(None);
    };
    let candidates = if source_kind == "SQLite" {
        load_cc_switch_database(&path)?
    } else {
        load_cc_switch_legacy_json(&path)?
    };
    Ok(Some((path, source_kind, candidates)))
}

fn cc_switch_api_format(value: Option<&str>) -> Option<String> {
    let normalized = value.map(normalized_config_key)?;
    match normalized.as_str() {
        "openairesponses" | "responses" => Some("responses".to_string()),
        "openaichat" | "chatcompletions" | "openai" => Some("chat-completions".to_string()),
        "anthropic" | "anthropicmessages" | "messages" => Some("anthropic".to_string()),
        "gemini" | "gemininative" => Some("gemini".to_string()),
        _ => None,
    }
}

fn is_cc_switch_secret_key(key: &str) -> bool {
    matches!(
        normalized_config_key(key).as_str(),
        "apikey"
            | "openaiapikey"
            | "anthropicauthtoken"
            | "anthropicapikey"
            | "geminiapikey"
            | "googleapikey"
            | "xaiapikey"
            | "authtoken"
            | "accesstoken"
            | "secretaccesskey"
    )
}

fn collect_cc_switch_secrets(value: &serde_json::Value, secrets: &mut HashSet<String>) {
    match value {
        serde_json::Value::Object(entries) => {
            for (key, value) in entries {
                if is_cc_switch_secret_key(key) {
                    if let Some(secret) = value.as_str().and_then(clean_imported_api_key) {
                        secrets.insert(secret);
                    }
                }
                collect_cc_switch_secrets(value, secrets);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_cc_switch_secrets(item, secrets);
            }
        }
        _ => {}
    }
}

fn redact_cc_switch_settings(value: &mut serde_json::Value, secrets: &[String]) {
    match value {
        serde_json::Value::String(text) => {
            for secret in secrets {
                *text = text.replace(secret, "${AGENTDOCK_API_KEY}");
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_cc_switch_settings(item, secrets);
            }
        }
        serde_json::Value::Object(entries) => {
            for (key, value) in entries {
                if is_cc_switch_secret_key(key)
                    && value.as_str().and_then(clean_imported_api_key).is_some()
                {
                    *value = serde_json::Value::String("${AGENTDOCK_API_KEY}".to_string());
                } else {
                    redact_cc_switch_settings(value, secrets);
                }
            }
        }
        _ => {}
    }
}

fn cc_switch_string_at(value: &serde_json::Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| json_string_at(value, path))
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

fn cc_switch_model(value: &serde_json::Value) -> Option<String> {
    cc_switch_string_at(
        value,
        &[
            &["env", "ANTHROPIC_MODEL"],
            &["env", "GEMINI_MODEL"],
            &["model"],
        ],
    )
    .or_else(|| {
        value
            .pointer("/models/0/id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    })
    .or_else(|| {
        value
            .get("models")
            .and_then(serde_json::Value::as_object)
            .and_then(|models| models.keys().next().cloned())
    })
    .filter(|value| !value.trim().is_empty())
}

fn cc_switch_preset_id(name: &str, base_url: &str) -> String {
    let name = name.to_ascii_lowercase();
    let mappings = [
        ("agentplan", "volcengine-agentplan"),
        ("doubao", "doubao"),
        ("deepseek", "deepseek"),
        ("openrouter", "openrouter"),
        ("packy", "packycode"),
        ("subrouter", "subrouter"),
        ("qiniu", "qiniu"),
        ("zhipu", "zhipu-glm"),
        ("minimax", "minimax"),
        ("modelscope", "modelscope"),
    ];
    if base_url.contains("volces.com/api/coding") {
        return "volcengine-agentplan".to_string();
    }
    mappings
        .iter()
        .find(|(needle, _)| name.contains(needle))
        .map(|(_, id)| (*id).to_string())
        .unwrap_or_default()
}

fn cc_switch_provider_id(candidate: &CcSwitchProviderCandidate) -> String {
    let identity = format!("{}:{}", candidate.source_app, candidate.source_id);
    let digest = format!("{:x}", Sha256::digest(identity.as_bytes()));
    let source_slug = slugify(&candidate.source_id);
    let source_slug = if source_slug.starts_with("provider-") {
        "provider".to_string()
    } else {
        source_slug
    };
    format!(
        "cc-switch-{}-{}-{}",
        candidate.app_id,
        source_slug,
        &digest[..10]
    )
}

fn prepare_cc_switch_provider(
    candidate: &CcSwitchProviderCandidate,
) -> Result<PreparedCcSwitchProvider, String> {
    let raw = serde_json::to_string(&candidate.settings_config)
        .map_err(|err| format!("读取配置失败: {}", err))?;
    let parsed = parse_provider_config_text(&candidate.app_id, &raw).unwrap_or_default();
    let base_url = normalize_base_url(parsed.base_url.as_deref().unwrap_or_default());
    let is_official = candidate.category.eq_ignore_ascii_case("official")
        || (base_url.is_empty() && candidate.name.to_ascii_lowercase().contains("official"));
    if !is_official && !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
        return Err("没有识别到有效请求地址".to_string());
    }

    let mut secrets = HashSet::new();
    collect_cc_switch_secrets(&candidate.settings_config, &mut secrets);
    if let Some(secret) = parsed
        .api_key
        .as_ref()
        .and_then(|value| clean_imported_api_key(value))
    {
        secrets.insert(secret);
    }
    if secrets.len() > 1 {
        return Err("包含多组凭据，暂不支持自动合并".to_string());
    }
    let api_key = secrets.iter().next().cloned();
    let mut sanitized_settings = candidate.settings_config.clone();
    let secret_values = secrets.into_iter().collect::<Vec<_>>();
    redact_cc_switch_settings(&mut sanitized_settings, &secret_values);

    let meta_format = candidate
        .meta
        .get("apiFormat")
        .and_then(serde_json::Value::as_str);
    let default_format = match candidate.app_id.as_str() {
        "claude-code" | "claude-desktop" => "anthropic",
        "antigravity" => "gemini",
        "grok" | "opencode" | "openclaw" | "hermes" => "chat-completions",
        _ => "responses",
    };
    let api_format = cc_switch_api_format(meta_format)
        .or_else(|| {
            parsed
                .api_format
                .and_then(|value| cc_switch_api_format(Some(&value)))
        })
        .unwrap_or_else(|| default_format.to_string());

    let model = parsed.model.unwrap_or_else(|| {
        cc_switch_model(&candidate.settings_config).unwrap_or_else(|| {
            match candidate.app_id.as_str() {
                "claude-code" | "claude-desktop" => "claude-sonnet-5".to_string(),
                "antigravity" => default_gemini_model(),
                _ => default_codex_model(),
            }
        })
    });
    let claude_sonnet_model = cc_switch_string_at(
        &candidate.settings_config,
        &[
            &["env", "ANTHROPIC_DEFAULT_SONNET_MODEL"],
            &["env", "ANTHROPIC_MODEL"],
        ],
    )
    .unwrap_or_else(|| model.clone());
    let claude_haiku_model = cc_switch_string_at(
        &candidate.settings_config,
        &[
            &["env", "ANTHROPIC_DEFAULT_HAIKU_MODEL"],
            &["env", "ANTHROPIC_MODEL"],
        ],
    )
    .unwrap_or_else(|| model.clone());
    let claude_opus_model = cc_switch_string_at(
        &candidate.settings_config,
        &[
            &["env", "ANTHROPIC_DEFAULT_OPUS_MODEL"],
            &["env", "ANTHROPIC_MODEL"],
        ],
    )
    .unwrap_or_else(|| model.clone());
    let settings_config = if is_official {
        String::new()
    } else {
        let settings = serde_json::to_string_pretty(&sanitized_settings)
            .map_err(|err| format!("生成脱敏配置失败: {}", err))?;
        validate_provider_settings_config(&candidate.app_id, &settings)?
    };
    let now = now_rfc3339();
    let active_apps = candidate
        .is_current
        .then(|| vec![candidate.app_id.clone()])
        .unwrap_or_default();
    let note = if candidate.notes.trim().is_empty() {
        "从 cc-switch 导入".to_string()
    } else {
        format!("{} · 从 cc-switch 导入", candidate.notes.trim())
    };
    let provider_type = if is_official {
        "official"
    } else {
        match candidate.app_id.as_str() {
            "claude-code" | "claude-desktop" => "anthropic",
            "antigravity" => "gemini",
            _ => "openai",
        }
    };
    Ok(PreparedCcSwitchProvider {
        provider: ProviderProfile {
            id: cc_switch_provider_id(candidate),
            name: candidate.name.trim().to_string(),
            notes: note,
            website_url: candidate.website_url.trim().to_string(),
            preset_id: cc_switch_preset_id(&candidate.name, &base_url),
            provider_type: provider_type.to_string(),
            base_url,
            api_format,
            settings_config,
            enabled_apps: vec![candidate.app_id.clone()],
            codex_model: model.clone(),
            gemini_model: if candidate.app_id == "antigravity" {
                model.clone()
            } else {
                default_gemini_model()
            },
            claude_sonnet_model,
            claude_haiku_model,
            claude_opus_model,
            active: !active_apps.is_empty(),
            active_apps,
            api_key_configured: api_key.is_some(),
            activation_reviewed: true,
            created_at: now.clone(),
            updated_at: now,
        },
        api_key,
    })
}

#[tauri::command]
fn detect_cc_switch_config() -> Result<CcSwitchImportDetection, String> {
    let Some((path, source_kind, candidates)) = load_cc_switch_candidates()? else {
        return Ok(CcSwitchImportDetection {
            available: false,
            source_path: String::new(),
            source_kind: String::new(),
            fingerprint: String::new(),
            provider_count: 0,
            app_counts: BTreeMap::new(),
        });
    };
    let mut app_counts = BTreeMap::new();
    let mut provider_count = 0;
    for candidate in &candidates {
        if prepare_cc_switch_provider(candidate).is_ok() {
            provider_count += 1;
            *app_counts.entry(candidate.app_id.clone()).or_insert(0) += 1;
        }
    }
    Ok(CcSwitchImportDetection {
        available: provider_count > 0,
        source_path: cc_switch_display_path(&path),
        source_kind,
        fingerprint: cc_switch_source_fingerprint(&path)?,
        provider_count,
        app_counts,
    })
}

#[tauri::command]
fn import_cc_switch_config() -> Result<CcSwitchImportResult, String> {
    let Some((_source_path, _source_kind, candidates)) = load_cc_switch_candidates()? else {
        return Err("没有检测到 cc-switch 配置".to_string());
    };
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut prepared = Vec::new();
    let mut errors = Vec::new();
    for candidate in &candidates {
        match prepare_cc_switch_provider(candidate) {
            Ok(provider) => prepared.push(provider),
            Err(error) => errors.push(format!(
                "{} / {}: {}",
                candidate.app_id, candidate.name, error
            )),
        }
    }
    if prepared.is_empty() {
        return Ok(CcSwitchImportResult {
            imported: 0,
            updated: 0,
            skipped: errors.len(),
            app_counts: BTreeMap::new(),
            errors,
            backup_dir: String::new(),
        });
    }

    let providers_file = providers_path(&dirs);
    let secrets_file = provider_secrets_path(&dirs);
    let old_providers = fs::read(&providers_file).ok();
    let old_secrets = fs::read(&secrets_file).ok();
    let backup_dir = dirs.backups_dir.join(format!(
        "cc-switch-import-{}",
        OffsetDateTime::now_utc().unix_timestamp()
    ));
    fs::create_dir_all(&backup_dir)
        .map_err(|err| format!("创建 cc-switch 导入备份失败: {}", err))?;
    if providers_file.exists() {
        fs::copy(&providers_file, backup_dir.join("providers.json"))
            .map_err(|err| format!("备份供应商配置失败: {}", err))?;
    }
    if secrets_file.exists() {
        let backup = backup_dir.join("provider-secrets.json");
        fs::copy(&secrets_file, &backup).map_err(|err| format!("备份供应商密钥失败: {}", err))?;
        protect_secret_file(&backup)?;
    }

    let mut providers = list_providers()?;
    let mut secrets = read_provider_secrets(&dirs)?;
    let existing_ids = providers
        .iter()
        .map(|provider| provider.id.clone())
        .collect::<HashSet<_>>();
    let current_apps = prepared
        .iter()
        .flat_map(|item| item.provider.active_apps.iter().cloned())
        .collect::<HashSet<_>>();
    for provider in &mut providers {
        provider
            .active_apps
            .retain(|app| !current_apps.contains(app));
        provider.active = !provider.active_apps.is_empty();
    }

    let mut imported = 0;
    let mut updated = 0;
    let mut app_counts = BTreeMap::new();
    let mut imported_profiles = Vec::new();
    for mut item in prepared {
        if let Some(existing) = providers
            .iter()
            .find(|provider| provider.id == item.provider.id)
        {
            item.provider.created_at = existing.created_at.clone();
            updated += 1;
        } else if existing_ids.contains(&item.provider.id) {
            updated += 1;
        } else {
            imported += 1;
        }
        if let Some(api_key) = item.api_key {
            secrets.insert(item.provider.id.clone(), api_key);
        }
        item.provider.api_key_configured = secrets.contains_key(&item.provider.id);
        *app_counts
            .entry(item.provider.enabled_apps[0].clone())
            .or_insert(0) += 1;
        providers.retain(|provider| provider.id != item.provider.id);
        imported_profiles.push(item.provider);
    }
    imported_profiles.extend(providers);

    let write_result = (|| {
        write_provider_secrets(&dirs, &secrets)?;
        write_providers(&dirs, &imported_profiles)
    })();
    if let Err(error) = write_result {
        restore_file_snapshot(&secrets_file, old_secrets.as_deref());
        restore_file_snapshot(&providers_file, old_providers.as_deref());
        return Err(error);
    }

    Ok(CcSwitchImportResult {
        imported,
        updated,
        skipped: errors.len(),
        app_counts,
        errors,
        backup_dir: backup_dir.display().to_string(),
    })
}

#[tauri::command]
async fn fetch_provider_models(
    app_id: String,
    base_url: String,
    api_key: Option<String>,
    api_format: Option<String>,
) -> Result<ProviderModelsResult, String> {
    let fallback = fallback_models(&app_id);
    let normalized_url = normalize_base_url(&base_url);
    if normalized_url.is_empty() {
        return Ok(ProviderModelsResult {
            models: fallback,
            source: "客户端推荐".to_string(),
            provider_supplied: false,
        });
    }
    if !(normalized_url.starts_with("https://") || normalized_url.starts_with("http://")) {
        return Err("请求地址必须以 http:// 或 https:// 开头".to_string());
    }
    if api_format.as_deref() == Some("anthropic") || app_id.starts_with("claude") {
        return Ok(ProviderModelsResult {
            models: fallback,
            source: "客户端推荐（Anthropic 协议不提供模型列表）".to_string(),
            provider_supplied: false,
        });
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建模型请求失败: {}", err))?;
    let key = api_key.unwrap_or_default();
    let protocol = if app_id == "antigravity" || api_format.as_deref() == Some("gemini") {
        ProviderModelProtocol::Gemini
    } else {
        ProviderModelProtocol::OpenAi
    };
    let endpoints = provider_model_endpoints(&normalized_url, protocol);
    let mut failures = Vec::new();

    for endpoint in endpoints {
        match fetch_models_from_endpoint(&client, &endpoint, key.trim(), protocol).await {
            Ok(models) => {
                return Ok(ProviderModelsResult {
                    models,
                    source: "供应商实际返回".to_string(),
                    provider_supplied: true,
                });
            }
            Err(error) => failures.push(error),
        }
    }

    Err(format!("读取供应商模型失败：{}", failures.join("；")))
}

fn provider_model_endpoints(base_url: &str, protocol: ProviderModelProtocol) -> Vec<String> {
    let base_url = base_url.trim_end_matches('/');
    if base_url.ends_with("/models") {
        return vec![base_url.to_string()];
    }
    let path = reqwest::Url::parse(base_url)
        .ok()
        .map(|url| url.path().trim_matches('/').to_string())
        .unwrap_or_default();
    let mut endpoints = match protocol {
        ProviderModelProtocol::OpenAi if path.is_empty() => {
            vec![
                format!("{}/v1/models", base_url),
                format!("{}/models", base_url),
            ]
        }
        ProviderModelProtocol::OpenAi => vec![format!("{}/models", base_url)],
        ProviderModelProtocol::Gemini if path.ends_with("v1beta") || path.ends_with("v1") => {
            vec![format!("{}/models", base_url)]
        }
        ProviderModelProtocol::Gemini if path.is_empty() => vec![
            format!("{}/v1beta/models", base_url),
            format!("{}/v1/models", base_url),
            format!("{}/models", base_url),
        ],
        ProviderModelProtocol::Gemini => vec![
            format!("{}/v1beta/models", base_url),
            format!("{}/models", base_url),
        ],
    };
    endpoints.dedup();
    endpoints
}

fn provider_model_request(
    client: &reqwest::Client,
    url: reqwest::Url,
    api_key: &str,
    protocol: ProviderModelProtocol,
) -> reqwest::RequestBuilder {
    let request = client.get(url).header("accept", "application/json");
    if api_key.is_empty() {
        return request;
    }
    match protocol {
        ProviderModelProtocol::OpenAi => request.bearer_auth(api_key),
        ProviderModelProtocol::Gemini => request.header("x-goog-api-key", api_key),
    }
}

async fn fetch_models_from_endpoint(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    protocol: ProviderModelProtocol,
) -> Result<Vec<String>, String> {
    let mut models = Vec::new();
    let mut page_token: Option<String> = None;
    for _ in 0..20 {
        let mut url = reqwest::Url::parse(endpoint)
            .map_err(|_| format!("{} 不是有效的模型接口", endpoint))?;
        if let Some(token) = page_token.as_deref() {
            url.query_pairs_mut().append_pair("pageToken", token);
        }
        let response = provider_model_request(client, url, api_key, protocol)
            .send()
            .await
            .map_err(|error| format!("{}：{}", endpoint, provider_request_error(&error)))?;
        let status = response.status();
        if !status.is_success() {
            let detail = provider_response_error_detail(&response.text().await.unwrap_or_default());
            return Err(if detail.is_empty() {
                format!("{} 返回 {}", endpoint, status)
            } else {
                format!("{} 返回 {} ({})", endpoint, status, detail)
            });
        }
        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|_| format!("{} 返回的不是有效 JSON", endpoint))?;
        for model in parse_model_ids(&payload, protocol) {
            if !models.contains(&model) {
                models.push(model);
            }
        }
        page_token = if protocol == ProviderModelProtocol::Gemini {
            payload
                .get("nextPageToken")
                .or_else(|| payload.get("next_page_token"))
                .and_then(serde_json::Value::as_str)
                .filter(|token| !token.is_empty())
                .map(str::to_string)
        } else {
            None
        };
        if page_token.is_none() {
            break;
        }
    }
    if models.is_empty() {
        Err(format!("{} 未返回可用的生成模型", endpoint))
    } else {
        Ok(models)
    }
}

fn provider_response_error_detail(raw: &str) -> String {
    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(raw) {
        for pointer in ["/error/message", "/message", "/error"] {
            if let Some(message) = payload.pointer(pointer).and_then(serde_json::Value::as_str) {
                return message.chars().take(120).collect();
            }
        }
    }
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect()
}

fn anthropic_messages_endpoint(base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    if base_url.ends_with("/v1/messages") {
        base_url.to_string()
    } else if base_url.ends_with("/v1") {
        format!("{}/messages", base_url)
    } else {
        format!("{}/v1/messages", base_url)
    }
}

fn anthropic_test_payload(model: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "max_tokens": 1,
        "messages": [{ "role": "user", "content": "Reply with OK." }]
    })
}

fn gemini_generate_endpoint(base_url: &str, model: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let api_base = if base_url.ends_with("/v1beta") || base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{}/v1beta", base_url)
    };
    format!(
        "{}/models/{}:streamGenerateContent?alt=sse",
        api_base, model
    )
}

fn gemini_test_payload() -> serde_json::Value {
    serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{ "text": "Reply with OK." }]
        }],
        "generationConfig": { "maxOutputTokens": 1 }
    })
}

fn provider_uses_anthropic_messages(provider: &ProviderProfile) -> bool {
    provider.api_format == "anthropic"
        || provider.provider_type == "anthropic"
        || (provider.api_format == "auto"
            && provider
                .enabled_apps
                .iter()
                .any(|app| app == "claude-code" || app == "claude-desktop"))
}

fn provider_anthropic_model(provider: &ProviderProfile) -> String {
    serde_json::from_str::<serde_json::Value>(&provider.settings_config)
        .ok()
        .and_then(|settings| {
            json_string_at(&settings, &["env", "ANTHROPIC_MODEL"]).map(str::to_string)
        })
        .filter(|model| !model.trim().is_empty())
        .or_else(|| (!provider.codex_model.trim().is_empty()).then(|| provider.codex_model.clone()))
        .unwrap_or_else(|| provider.claude_sonnet_model.clone())
}

#[tauri::command]
fn run_ready_check() -> Result<ReadyCheck, String> {
    let providers = list_providers()?;
    let clients = cached_client_detection();
    let clients_ready = clients.iter().filter(|client| client.installed).count();
    let providers_ready = providers
        .iter()
        .filter(|provider| {
            provider.provider_type == "official"
                || (!provider.base_url.trim().is_empty()
                    && !provider.base_url.contains("agentdock.example")
                    && (provider.api_key_configured || is_local_url(&provider.base_url)))
        })
        .count();

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if !clients
        .iter()
        .any(|client| client.id == "codex" && client.installed)
    {
        blockers.push("Codex 尚未安装".to_string());
    }

    if providers_ready == 0 {
        blockers.push("还没有可用供应商".to_string());
    }

    if !clients
        .iter()
        .any(|client| client.id == "claude-code" && client.installed)
    {
        warnings.push("Claude Code 未安装，可以稍后安装".to_string());
    }

    let score = match (blockers.len(), warnings.len()) {
        (0, 0) => 100,
        (0, _) => 88,
        (1, _) => 72,
        _ => 48,
    };

    Ok(ReadyCheck {
        score,
        blockers,
        warnings,
        clients_ready,
        providers_ready,
    })
}

#[tauri::command]
fn list_providers() -> Result<Vec<ProviderProfile>, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let path = providers_path(&dirs);

    if !path.exists() {
        let defaults = default_providers();
        write_providers(&dirs, &defaults)?;
        return Ok(defaults);
    }

    let raw = fs::read_to_string(&path).map_err(|err| format!("读取供应商配置失败: {}", err))?;
    let mut providers: Vec<ProviderProfile> =
        serde_json::from_str(&raw).map_err(|err| format!("解析供应商配置失败: {}", err))?;
    let mut migrated = false;
    let original_len = providers.len();
    providers.retain(|provider| !provider.base_url.contains("agentdock.example"));
    migrated |= providers.len() != original_len;
    for provider in &mut providers {
        migrated |= migrate_app_ids(&mut provider.enabled_apps);
        migrated |= migrate_app_ids(&mut provider.active_apps);
        if provider.active && provider.active_apps.is_empty() {
            provider.active_apps = provider.enabled_apps.clone();
            migrated = true;
        }
        provider.active = !provider.active_apps.is_empty();
    }
    if providers
        .iter()
        .any(|provider| provider.id.starts_with("cc-switch-") && !provider.activation_reviewed)
    {
        if let Ok(Some((_path, _source_kind, candidates))) = load_cc_switch_candidates() {
            migrated |= reconcile_cc_switch_activations(&mut providers, &candidates);
        }
    }
    if migrated {
        write_providers(&dirs, &providers)?;
    }
    Ok(providers)
}

fn reconcile_cc_switch_activations(
    providers: &mut [ProviderProfile],
    candidates: &[CcSwitchProviderCandidate],
) -> bool {
    let source_states = candidates
        .iter()
        .map(|candidate| {
            (
                cc_switch_provider_id(candidate),
                (candidate.app_id.as_str(), candidate.is_current),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut migrated = false;
    for provider in providers
        .iter_mut()
        .filter(|provider| provider.id.starts_with("cc-switch-") && !provider.activation_reviewed)
    {
        let Some(&(app_id, is_current)) = source_states.get(&provider.id) else {
            continue;
        };
        if !is_current {
            provider.active_apps.retain(|app| app != app_id);
            provider.active = !provider.active_apps.is_empty();
        }
        provider.activation_reviewed = true;
        migrated = true;
    }
    migrated
}

fn provider_api_key_for_edit(
    provider_id: &str,
    providers: &[ProviderProfile],
    secrets: &BTreeMap<String, String>,
) -> Result<String, String> {
    if !providers.iter().any(|provider| provider.id == provider_id) {
        return Err("未找到供应商".to_string());
    }
    Ok(secrets.get(provider_id).cloned().unwrap_or_default())
}

#[tauri::command]
fn get_provider_api_key(provider_id: String) -> Result<String, String> {
    let provider_id = provider_id.trim();
    if provider_id.is_empty() {
        return Err("未找到供应商".to_string());
    }
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let providers = list_providers()?;
    let secrets = read_provider_secrets(&dirs)?;
    provider_api_key_for_edit(provider_id, &providers, &secrets)
}

#[tauri::command]
fn save_provider(input: ProviderInput) -> Result<ProviderProfile, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;

    if input.name.trim().is_empty() {
        return Err("供应商名称不能为空".to_string());
    }
    let normalized_url = normalize_base_url(&input.base_url);
    let is_official = input.provider_type.trim() == "official";
    if !is_official
        && !(normalized_url.starts_with("https://") || normalized_url.starts_with("http://"))
    {
        return Err("Base URL 必须以 http:// 或 https:// 开头".to_string());
    }

    let mut providers = list_providers()?;
    let mut secrets = read_provider_secrets(&dirs)?;
    let now = now_rfc3339();
    let requested_id = input.id.clone().filter(|id| !id.trim().is_empty());
    let provider_id = requested_id.clone().unwrap_or_else(|| {
        let app = input
            .enabled_apps
            .as_ref()
            .and_then(|apps| apps.first())
            .map(|app| slugify(app))
            .unwrap_or_else(|| "provider".to_string());
        unique_provider_id(&format!("{}-{}", app, slugify(&input.name)), &providers)
    });

    let existing = providers
        .iter()
        .find(|provider| provider.id == provider_id)
        .cloned();
    let existing_created_at = existing
        .as_ref()
        .map(|provider| provider.created_at.clone())
        .unwrap_or_else(|| now.clone());

    let enabled_apps = input.enabled_apps.unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|provider| provider.enabled_apps.clone())
            .unwrap_or_else(|| vec!["codex".to_string()])
    });
    let active_apps = input.active_apps.unwrap_or_else(|| {
        if input.active == Some(true) {
            enabled_apps.clone()
        } else {
            existing
                .as_ref()
                .map(|provider| provider.active_apps.clone())
                .unwrap_or_default()
        }
    });
    if let Some(api_key) = input.api_key.as_ref() {
        let api_key = api_key.trim();
        if !api_key.is_empty() {
            secrets.insert(provider_id.clone(), api_key.to_string());
            write_provider_secrets(&dirs, &secrets)?;
        }
    }
    let mut raw_settings_config = input.settings_config.unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|provider| provider.settings_config.clone())
            .unwrap_or_default()
    });
    if let Some(api_key) = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        raw_settings_config = raw_settings_config.replace(api_key, "${AGENTDOCK_API_KEY}");
    }
    let settings_config = if raw_settings_config.trim().is_empty() {
        String::new()
    } else {
        let app_id = enabled_apps.first().map(String::as_str).unwrap_or("codex");
        validate_provider_settings_config(app_id, &raw_settings_config)?
    };

    let provider = ProviderProfile {
        id: provider_id.clone(),
        name: input.name.trim().to_string(),
        notes: input.notes.unwrap_or_default().trim().to_string(),
        website_url: input.website_url.unwrap_or_default().trim().to_string(),
        preset_id: input.preset_id.unwrap_or_default().trim().to_string(),
        provider_type: input.provider_type.trim().to_string(),
        base_url: normalized_url,
        api_format: input
            .api_format
            .unwrap_or_else(|| "auto".to_string())
            .trim()
            .to_string(),
        settings_config,
        enabled_apps,
        codex_model: input.codex_model.unwrap_or_else(default_codex_model),
        gemini_model: input.gemini_model.unwrap_or_else(default_gemini_model),
        claude_sonnet_model: input
            .claude_sonnet_model
            .unwrap_or_else(|| "claude-sonnet-5".to_string()),
        claude_haiku_model: input
            .claude_haiku_model
            .unwrap_or_else(|| "claude-haiku-4-5".to_string()),
        claude_opus_model: input
            .claude_opus_model
            .unwrap_or_else(|| "claude-opus-4-8".to_string()),
        active: !active_apps.is_empty(),
        active_apps,
        api_key_configured: secrets.contains_key(&provider_id),
        activation_reviewed: true,
        created_at: existing_created_at,
        updated_at: now,
    };

    providers.retain(|item| item.id != provider_id);
    for app in &provider.active_apps {
        for item in &mut providers {
            item.active_apps.retain(|active_app| active_app != app);
            item.active = !item.active_apps.is_empty();
        }
    }
    providers.insert(0, provider.clone());

    write_providers(&dirs, &providers)?;
    Ok(provider)
}

#[tauri::command]
fn activate_provider(provider_id: String, app_id: String) -> Result<ProviderProfile, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut providers = list_providers()?;
    let mut selected = None;
    for provider in &mut providers {
        provider.active_apps.retain(|app| app != &app_id);
        if provider.id == provider_id {
            if !provider.enabled_apps.iter().any(|app| app == &app_id) {
                return Err("这个供应商没有启用当前客户端".to_string());
            }
            provider.active_apps.push(app_id.clone());
            provider.activation_reviewed = true;
            provider.updated_at = now_rfc3339();
        }
        provider.active = !provider.active_apps.is_empty();
        if provider.id == provider_id {
            selected = Some(provider.clone());
        }
    }

    let selected = selected.ok_or_else(|| "未找到供应商".to_string())?;
    write_providers(&dirs, &providers)?;
    Ok(selected)
}

#[tauri::command]
fn delete_provider(provider_id: String) -> Result<OperationResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut providers = list_providers()?;
    if !remove_provider_profile(&mut providers, &provider_id) {
        return Err("未找到供应商".to_string());
    }
    write_providers(&dirs, &providers)?;

    let mut secrets = read_provider_secrets(&dirs)?;
    secrets.remove(&provider_id);
    write_provider_secrets(&dirs, &secrets)?;

    Ok(OperationResult {
        ok: true,
        message: "供应商已删除".to_string(),
    })
}

fn remove_provider_profile(providers: &mut Vec<ProviderProfile>, provider_id: &str) -> bool {
    let before = providers.len();
    providers.retain(|provider| provider.id != provider_id);
    providers.len() != before
}

#[tauri::command]
fn preview_provider_config(provider: ProviderProfile) -> Result<ConfigPreview, String> {
    let base_url = normalize_base_url(&provider.base_url);
    let claude_model = provider_anthropic_model(&provider);
    let codex_base_url = ensure_v1_url(&base_url);
    let model = serde_json::to_string(&provider.codex_model).map_err(|err| err.to_string())?;
    let provider_name = serde_json::to_string(&provider.name).map_err(|err| err.to_string())?;
    let codex_base_url = serde_json::to_string(&codex_base_url).map_err(|err| err.to_string())?;
    let codex_toml = format!(
        r#"model_provider = "custom"
model = {}
model_reasoning_effort = "high"

[model_providers.custom]
name = {}
base_url = {}
wire_api = "responses"
env_key = "OPENAI_API_KEY"
"#,
        model, provider_name, codex_base_url
    );

    let mut claude_env = serde_json::Map::from_iter([
        (
            "ANTHROPIC_BASE_URL".to_string(),
            serde_json::Value::String(base_url.clone()),
        ),
        (
            "ANTHROPIC_AUTH_TOKEN".to_string(),
            serde_json::Value::String("${AGENTDOCK_API_KEY}".to_string()),
        ),
        (
            "ANTHROPIC_MODEL".to_string(),
            serde_json::Value::String(claude_model.clone()),
        ),
        (
            "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
            serde_json::Value::String(provider.claude_sonnet_model.clone()),
        ),
        (
            "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
            serde_json::Value::String(provider.claude_haiku_model.clone()),
        ),
        (
            "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
            serde_json::Value::String(provider.claude_opus_model.clone()),
        ),
    ]);
    if provider.preset_id == "volcengine-agentplan" {
        for key in [
            "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
            "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
            "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME",
        ] {
            claude_env.insert(
                key.to_string(),
                serde_json::Value::String(claude_model.clone()),
            );
        }
        claude_env.insert(
            "ANTHROPIC_DEFAULT_FABLE_MODEL".to_string(),
            serde_json::Value::String(provider.claude_sonnet_model.clone()),
        );
    }
    let claude_env_json = serde_json::json!({ "env": claude_env });

    let gemini_env_json = serde_json::json!({
        "env": {
            "GOOGLE_GEMINI_BASE_URL": base_url,
            "GEMINI_API_KEY": "${AGENTDOCK_API_KEY}",
            "GEMINI_MODEL": provider.gemini_model
        }
    });

    Ok(ConfigPreview {
        codex_toml,
        claude_env_json: serde_json::to_string_pretty(&claude_env_json)
            .map_err(|err| err.to_string())?,
        gemini_env_json: serde_json::to_string_pretty(&gemini_env_json)
            .map_err(|err| err.to_string())?,
    })
}

fn grok_provider_toml(provider: &ProviderProfile) -> Result<String, String> {
    let mut root = toml::Table::new();
    let mut models = toml::Table::new();
    models.insert(
        "default".to_string(),
        toml::Value::String("agentdock".to_string()),
    );
    root.insert("models".to_string(), toml::Value::Table(models));

    let mut definition = toml::Table::new();
    definition.insert(
        "model".to_string(),
        toml::Value::String(provider.codex_model.clone()),
    );
    definition.insert(
        "base_url".to_string(),
        toml::Value::String(provider.base_url.clone()),
    );
    definition.insert(
        "name".to_string(),
        toml::Value::String(provider.name.clone()),
    );
    definition.insert(
        "env_key".to_string(),
        toml::Value::String("XAI_API_KEY".to_string()),
    );
    let backend = match provider.api_format.as_str() {
        "responses" => "responses",
        "anthropic" => "messages",
        _ => "chat_completions",
    };
    definition.insert(
        "api_backend".to_string(),
        toml::Value::String(backend.to_string()),
    );
    let mut model = toml::Table::new();
    model.insert("agentdock".to_string(), toml::Value::Table(definition));
    root.insert("model".to_string(), toml::Value::Table(model));
    toml::to_string_pretty(&root).map_err(|err| format!("生成 Grok config.toml 失败: {}", err))
}

fn grok_default_model_from_config(config: &str) -> Option<String> {
    let root = config.parse::<toml::Table>().ok()?;
    let model = root
        .get("models")?
        .as_table()?
        .get("default")?
        .as_str()?
        .trim();
    if model.is_empty() || model.chars().any(char::is_control) {
        None
    } else {
        Some(model.to_string())
    }
}

fn grok_default_model_from_settings(settings: Option<&serde_json::Value>) -> Option<String> {
    match settings {
        Some(settings) => settings
            .get("config")
            .and_then(serde_json::Value::as_str)
            .and_then(grok_default_model_from_config),
        None => Some("agentdock".to_string()),
    }
}

fn preserve_toml_section(path: &Path, content: &str, section: &str) -> Result<String, String> {
    let mut updated: toml::Table = content
        .parse()
        .map_err(|err| format!("config.toml 格式错误: {}", err))?;
    if path.exists() {
        let existing: toml::Table = fs::read_to_string(path)
            .map_err(|err| format!("读取现有 config.toml 失败: {}", err))?
            .parse()
            .map_err(|err| format!("现有 config.toml 格式错误: {}", err))?;
        if let Some(value) = existing.get(section) {
            updated.insert(section.to_string(), value.clone());
        }
    }
    toml::to_string_pretty(&updated).map_err(|err| format!("生成 config.toml 失败: {}", err))
}

fn managed_cli_command_names(client_id: &str) -> &'static [&'static str] {
    match client_id {
        "codex" => &["codex"],
        "claude-code" => &["claude"],
        "antigravity" => &["antigravity", "agy"],
        "grok" => &["grok"],
        "opencode" => &["opencode"],
        "openclaw" => &["openclaw"],
        "hermes" => &["hermes"],
        _ => &[],
    }
}

fn managed_cli_bin_dir(home: &Path) -> PathBuf {
    home.join(".agentdock").join("bin")
}

fn managed_cli_shim_path(bin_dir: &Path, command_name: &str) -> PathBuf {
    if cfg!(windows) {
        bin_dir.join(format!("{}.cmd", command_name))
    } else {
        bin_dir.join(command_name)
    }
}

#[cfg(unix)]
fn managed_cli_shim_content(app_executable: &Path, client_id: &str) -> String {
    format!(
        "#!/bin/sh\n# {}\nexec {} --agentdock-cli {} \"$@\"\n",
        MANAGED_CLI_MARKER,
        shell_quote(&app_executable.to_string_lossy()),
        shell_quote(client_id)
    )
}

#[cfg(windows)]
fn managed_cli_shim_content(app_executable: &Path, client_id: &str) -> String {
    let executable = app_executable.to_string_lossy().replace('%', "%%");
    format!(
        "@echo off\r\nrem {}\r\n\"{}\" --agentdock-cli {} %*\r\n",
        MANAGED_CLI_MARKER, executable, client_id
    )
}

fn write_managed_cli_shims(
    bin_dir: &Path,
    app_executable: &Path,
    client: &ManagedClientRecord,
) -> Result<Vec<String>, String> {
    let command_names = managed_cli_command_names(&client.id);
    if command_names.is_empty() {
        return Ok(Vec::new());
    }
    fs::create_dir_all(bin_dir).map_err(|err| format!("创建 AgentDock 命令目录失败: {}", err))?;
    let content = managed_cli_shim_content(app_executable, &client.id);
    for command_name in command_names {
        let path = managed_cli_shim_path(bin_dir, command_name);
        if path.exists() {
            let existing = fs::read_to_string(&path)
                .map_err(|err| format!("读取现有 {} 命令失败: {}", command_name, err))?;
            if !existing.contains(MANAGED_CLI_MARKER) {
                return Err(format!(
                    "{} 已存在且不属于 AgentDock，请先移走该文件: {}",
                    command_name,
                    path.display()
                ));
            }
        }
        fs::write(&path, &content)
            .map_err(|err| format!("写入 {} 终端命令失败: {}", command_name, err))?;
        make_executable(&path)?;
    }
    Ok(command_names
        .iter()
        .map(|name| (*name).to_string())
        .collect())
}

fn remove_managed_cli_shims(home: &Path, client_id: &str) -> Result<(), String> {
    let bin_dir = managed_cli_bin_dir(home);
    for command_name in managed_cli_command_names(client_id) {
        let path = managed_cli_shim_path(&bin_dir, command_name);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|err| format!("读取 {} 终端命令失败: {}", command_name, err))?;
        if content.contains(MANAGED_CLI_MARKER) {
            fs::remove_file(&path)
                .map_err(|err| format!("删除 {} 终端命令失败: {}", command_name, err))?;
        }
    }
    Ok(())
}

fn upsert_managed_path_block(existing: &str, block: &str) -> Result<String, String> {
    match (
        existing.find(MANAGED_PATH_BLOCK_START),
        existing.find(MANAGED_PATH_BLOCK_END),
    ) {
        (Some(start), Some(end)) if end >= start => {
            let suffix_start = end + MANAGED_PATH_BLOCK_END.len();
            Ok(format!(
                "{}{}{}",
                &existing[..start],
                block,
                &existing[suffix_start..]
            ))
        }
        (None, None) => {
            let mut updated = existing.to_string();
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            if !updated.is_empty() {
                updated.push('\n');
            }
            updated.push_str(block);
            updated.push('\n');
            Ok(updated)
        }
        _ => Err("Shell 配置中的 AgentDock PATH 标记不完整，请手动清理后重试".to_string()),
    }
}

#[cfg(unix)]
fn shell_profile_targets(home: &Path, shell: Option<&str>) -> Vec<(PathBuf, &'static str)> {
    let shell_name = shell
        .and_then(|shell| Path::new(shell).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or(if cfg!(target_os = "macos") {
            "zsh"
        } else {
            "sh"
        });
    let posix_block = concat!(
        "# >>> AgentDock CLI >>>\n",
        "case \":$PATH:\" in\n",
        "  *\":$HOME/.agentdock/bin:\"*) ;;\n",
        "  *) export PATH=\"$HOME/.agentdock/bin:$PATH\" ;;\n",
        "esac\n",
        "# <<< AgentDock CLI <<<"
    );
    let fish_block = concat!(
        "# >>> AgentDock CLI >>>\n",
        "fish_add_path --global \"$HOME/.agentdock/bin\"\n",
        "# <<< AgentDock CLI <<<"
    );
    match shell_name {
        "zsh" => vec![(home.join(".zshrc"), posix_block)],
        "bash" => vec![
            (home.join(".bash_profile"), posix_block),
            (home.join(".bashrc"), posix_block),
        ],
        "fish" => vec![(home.join(".config/fish/config.fish"), fish_block)],
        _ => vec![(home.join(".profile"), posix_block)],
    }
}

#[cfg(unix)]
fn ensure_managed_cli_path(home: &Path, bin_dir: &Path) -> Result<(), String> {
    if bin_dir != managed_cli_bin_dir(home) {
        return Err("AgentDock 命令目录无效".to_string());
    }
    let shell = env::var("SHELL").ok();
    for (profile, block) in shell_profile_targets(home, shell.as_deref()) {
        if let Some(parent) = profile.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建 Shell 配置目录失败: {}", err))?;
        }
        let existing = if profile.exists() {
            fs::read_to_string(&profile)
                .map_err(|err| format!("读取 {} 失败: {}", profile.display(), err))?
        } else {
            String::new()
        };
        let updated = upsert_managed_path_block(&existing, block)?;
        if updated != existing {
            fs::write(&profile, updated)
                .map_err(|err| format!("更新 {} 失败: {}", profile.display(), err))?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn ensure_managed_cli_path(_home: &Path, bin_dir: &Path) -> Result<(), String> {
    let script = r#"$target=$args[0]; $current=[Environment]::GetEnvironmentVariable('Path','User'); $parts=@($current -split ';' | Where-Object { $_ }); if ($parts -notcontains $target) { [Environment]::SetEnvironmentVariable('Path', (($parts + $target) -join ';'), 'User') }"#;
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .arg(bin_dir)
        .output()
        .map_err(|err| format!("更新用户 PATH 失败: {}", err))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "更新用户 PATH 失败: {}",
            command_failure_detail(&output)
        ))
    }
}

fn install_managed_cli_access(client: &ManagedClientRecord) -> Result<Vec<String>, String> {
    let home = dirs_home().ok_or_else(|| "无法确定用户主目录".to_string())?;
    let bin_dir = managed_cli_bin_dir(&home);
    let app_executable =
        env::current_exe().map_err(|err| format!("读取 AgentDock 路径失败: {}", err))?;
    let commands = write_managed_cli_shims(&bin_dir, &app_executable, client)?;
    if !commands.is_empty() {
        ensure_managed_cli_path(&home, &bin_dir)?;
    }
    Ok(commands)
}

fn repair_managed_cli_access() -> Result<(), String> {
    for client in list_managed_clients()?
        .into_iter()
        .filter(|client| client.installed && Path::new(&client.launcher_path).is_file())
    {
        install_managed_cli_access(&client)?;
    }
    Ok(())
}

fn managed_cli_request(args: &[OsString]) -> Option<Result<(String, Vec<OsString>), String>> {
    if args.get(1).and_then(|value| value.to_str()) != Some("--agentdock-cli") {
        return None;
    }
    let client_id = match args.get(2).and_then(|value| value.to_str()) {
        Some(client_id) if !client_id.is_empty() => client_id.to_string(),
        _ => return Some(Err("缺少托管客户端名称".to_string())),
    };
    Some(Ok((client_id, args.iter().skip(3).cloned().collect())))
}

fn run_managed_cli_command(client_id: &str, args: &[OsString]) -> Result<i32, String> {
    let spec = client_spec(client_id)?;
    if managed_cli_command_names(spec.id).is_empty() {
        return Err("这个客户端不支持终端命令".to_string());
    }
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let client = list_managed_clients()?
        .into_iter()
        .find(|client| client.id == spec.id && client.installed)
        .ok_or_else(|| format!("{} 尚未由 AgentDock 安装", spec.name))?;
    if !Path::new(&client.launcher_path).is_file() {
        return Err(format!(
            "{} 启动文件不存在，请在 AgentDock 中重新安装",
            spec.name
        ));
    }

    let active_provider = list_providers()?.into_iter().find(|provider| {
        provider
            .active_apps
            .iter()
            .any(|active_app| active_app == spec.id)
    });
    let environment = match active_provider.as_ref() {
        Some(provider) => {
            ensure_provider_launch_config(&dirs, spec.id, provider)?;
            provider_launch_environment(&dirs, spec.id, provider)?
        }
        None => BTreeMap::new(),
    };
    let mut command = Command::new(&client.launcher_path);
    command.args(args).envs(environment);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        Err(format!("无法启动 {}: {}", client.name, error))
    }
    #[cfg(windows)]
    {
        let status = command
            .status()
            .map_err(|err| format!("无法启动 {}: {}", client.name, err))?;
        Ok(status.code().unwrap_or(1))
    }
}

#[tauri::command]
async fn install_client(client_id: String) -> Result<InstallClientResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let now = now_rfc3339();
    let spec = client_spec(&client_id)?;
    if spec.id == "hermes" {
        return install_hermes_client(&dirs).await;
    }
    let install_dir = dirs.clients_dir.join(&spec.id);
    let staging_dir = dirs.clients_dir.join(format!("{}.installing", spec.id));
    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir).map_err(|err| format!("清理临时安装目录失败: {}", err))?;
    }
    fs::create_dir_all(&staging_dir).map_err(|err| format!("创建客户端目录失败: {}", err))?;

    let (staged_executable, version, payload_message) =
        if let Some(payload_dir) = bundled_client_payload_dir(spec.id) {
            copy_dir_all(&payload_dir, &staging_dir)?;
            let executable = find_client_executable(&staging_dir, spec.id)?;
            (
                executable,
                "bundled".to_string(),
                format!("内置安装包 {}", payload_dir.display()),
            )
        } else {
            download_client_release(spec.id, &staging_dir).await?
        };

    make_executable(&staged_executable)?;
    let relative_executable = staged_executable
        .strip_prefix(&staging_dir)
        .map_err(|_| "安装程序返回了无效的启动路径".to_string())?
        .to_path_buf();
    fs::write(staging_dir.join("INSTALL_SOURCE.txt"), &payload_message)
        .map_err(|err| format!("写入安装来源失败: {}", err))?;

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|err| format!("替换旧客户端失败: {}", err))?;
    }
    fs::rename(&staging_dir, &install_dir).map_err(|err| format!("完成客户端安装失败: {}", err))?;
    let launcher_path = install_dir.join(relative_executable);
    let config_dir = dirs.managed_configs_dir.join(spec.id);
    fs::create_dir_all(&config_dir).map_err(|err| format!("创建客户端配置目录失败: {}", err))?;

    let detected_version = if spec.id == "antigravity" {
        version
    } else {
        command_version(&launcher_path.display().to_string())
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or(version)
    };

    let record = ManagedClientRecord {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        installed: true,
        version: detected_version,
        install_dir: install_dir.display().to_string(),
        launcher_path: launcher_path.display().to_string(),
        config_dir: config_dir.display().to_string(),
        installed_at: now.clone(),
        updated_at: now,
    };

    let mut clients = list_managed_clients()?;
    clients.retain(|client| client.id != record.id);
    clients.push(record.clone());
    write_json(&managed_clients_path(&dirs), &clients)?;
    let commands = install_managed_cli_access(&record)?;

    Ok(InstallClientResult {
        client: record,
        message: format!(
            "{} 已安装并通过启动文件校验。{}{}",
            spec.name,
            payload_message,
            if commands.is_empty() {
                String::new()
            } else {
                format!("。终端命令 {} 已就绪，请重新打开终端", commands.join("、"))
            }
        ),
    })
}

#[tauri::command]
fn list_managed_clients() -> Result<Vec<ManagedClientRecord>, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    read_json_or_seed(
        &managed_clients_path(&dirs),
        Vec::<ManagedClientRecord>::new(),
    )
}

#[tauri::command]
fn uninstall_client(client_id: String) -> Result<OperationResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut clients = list_managed_clients()?;
    let managed = clients
        .iter()
        .find(|client| client.id == client_id)
        .cloned()
        .ok_or_else(|| "这个客户端不是由 AgentDock 安装的，不能自动卸载".to_string())?;

    let install_dir = PathBuf::from(&managed.install_dir);
    if install_dir.starts_with(&dirs.clients_dir) && install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|err| format!("卸载客户端失败: {}", err))?;
    }
    clients.retain(|client| client.id != client_id);
    write_json(&managed_clients_path(&dirs), &clients)?;
    if let Some(home) = dirs_home() {
        remove_managed_cli_shims(&home, &managed.id)?;
    }

    Ok(OperationResult {
        ok: true,
        message: format!("{} 已从 AgentDock 托管目录移除", managed.name),
    })
}

#[tauri::command]
async fn test_provider(provider_id: String) -> Result<ProviderTestResult, String> {
    let start = Instant::now();
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let provider = list_providers()?
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| "未找到供应商".to_string())?;
    if provider.provider_type == "official" {
        return Ok(ProviderTestResult {
            ok: true,
            latency_ms: start.elapsed().as_millis(),
            message: "官方供应商使用客户端登录，无需测试 API Key".to_string(),
        });
    }
    let base_url = normalize_base_url(&provider.base_url);
    if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
        return Ok(ProviderTestResult {
            ok: false,
            latency_ms: start.elapsed().as_millis(),
            message: "Base URL 必须以 http:// 或 https:// 开头".to_string(),
        });
    }

    if base_url.contains("agentdock.example") {
        return Ok(ProviderTestResult {
            ok: false,
            latency_ms: start.elapsed().as_millis(),
            message: "当前是示例地址，请填入真实供应商地址".to_string(),
        });
    }

    let secrets = read_provider_secrets(&dirs)?;
    let api_key = secrets.get(&provider.id).cloned().unwrap_or_default();
    if api_key.is_empty() && !is_local_url(&base_url) {
        return Ok(ProviderTestResult {
            ok: false,
            latency_ms: start.elapsed().as_millis(),
            message: "请先填写 API Key".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(12))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建网络请求失败: {}", err))?;

    if provider_uses_anthropic_messages(&provider) {
        let endpoint = anthropic_messages_endpoint(&base_url);
        let model = provider_anthropic_model(&provider);
        let response = client
            .post(&endpoint)
            .header("accept", "application/json")
            .header("anthropic-version", "2023-06-01")
            .bearer_auth(&api_key)
            .header("x-api-key", &api_key)
            .json(&anthropic_test_payload(&model))
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                return Ok(ProviderTestResult {
                    ok: false,
                    latency_ms: start.elapsed().as_millis().max(1),
                    message: provider_request_error(&error),
                });
            }
        };
        let status = response.status();
        if status.is_success() {
            return Ok(ProviderTestResult {
                ok: true,
                latency_ms: start.elapsed().as_millis().max(1),
                message: format!("{} 连接成功，消息接口返回 {}", provider.name, status),
            });
        }
        let response_text = response.text().await.unwrap_or_default();
        let detail = response_text.chars().take(180).collect::<String>();
        return Ok(ProviderTestResult {
            ok: false,
            latency_ms: start.elapsed().as_millis().max(1),
            message: if detail.is_empty() {
                format!("连接返回 {}，请检查地址、协议、模型和密钥", status)
            } else {
                format!("连接返回 {}: {}", status, detail)
            },
        });
    }

    let is_gemini = provider.api_format == "gemini"
        || provider.provider_type == "gemini"
        || provider.enabled_apps.iter().any(|app| app == "antigravity");
    if is_gemini {
        let endpoint = gemini_generate_endpoint(&base_url, &provider.gemini_model);
        let mut request = client
            .post(&endpoint)
            .header("accept", "text/event-stream")
            .json(&gemini_test_payload());
        if !api_key.is_empty() {
            request = request.header("x-goog-api-key", &api_key);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                return Ok(ProviderTestResult {
                    ok: false,
                    latency_ms: start.elapsed().as_millis().max(1),
                    message: provider_request_error(&error),
                });
            }
        };
        let status = response.status();
        if status.is_success() {
            return Ok(ProviderTestResult {
                ok: true,
                latency_ms: start.elapsed().as_millis().max(1),
                message: format!(
                    "{} 连接成功，模型 {} 生成接口返回 {}",
                    provider.name, provider.gemini_model, status
                ),
            });
        }
        let detail = provider_response_error_detail(&response.text().await.unwrap_or_default());
        return Ok(ProviderTestResult {
            ok: false,
            latency_ms: start.elapsed().as_millis().max(1),
            message: if detail.is_empty() {
                format!(
                    "模型 {} 生成接口返回 {}，请检查模型、地址和密钥",
                    provider.gemini_model, status
                )
            } else {
                format!(
                    "模型 {} 生成接口返回 {}: {}",
                    provider.gemini_model, status, detail
                )
            },
        });
    }
    let protocol = if is_gemini {
        ProviderModelProtocol::Gemini
    } else {
        ProviderModelProtocol::OpenAi
    };
    let endpoints = provider_model_endpoints(&base_url, protocol);
    let mut last_failure = None;
    for endpoint in endpoints {
        let url = match reqwest::Url::parse(&endpoint) {
            Ok(url) => url,
            Err(_) => {
                last_failure = Some(format!("模型接口地址无效: {}", endpoint));
                continue;
            }
        };
        let response = match provider_model_request(&client, url, &api_key, protocol)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => {
                last_failure = Some(provider_request_error(&error));
                continue;
            }
        };
        let status = response.status();
        if status.is_success() {
            return Ok(ProviderTestResult {
                ok: true,
                latency_ms: start.elapsed().as_millis().max(1),
                message: format!("{} 连接成功，模型接口返回 {}", provider.name, status),
            });
        }

        let response_text = response.text().await.unwrap_or_default();
        let detail = response_text.chars().take(180).collect::<String>();
        last_failure = Some(if detail.is_empty() {
            format!("连接返回 {}，请检查地址、协议和密钥", status)
        } else {
            format!("连接返回 {}: {}", status, detail)
        });
    }

    Ok(ProviderTestResult {
        ok: false,
        latency_ms: start.elapsed().as_millis().max(1),
        message: last_failure.unwrap_or_else(|| "供应商没有可测试的模型接口".to_string()),
    })
}

fn provider_request_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "连接超时，请检查网络或供应商状态".to_string()
    } else if error.is_connect() {
        "无法连接供应商，请检查请求地址和网络".to_string()
    } else if error.is_request() {
        "供应商请求无法发送，请检查请求地址".to_string()
    } else {
        "供应商连接失败".to_string()
    }
}

#[tauri::command]
fn apply_active_provider_configs() -> Result<ApplyProviderResult, String> {
    let providers = list_providers()?;
    let mut written_files = Vec::new();
    let mut backup_dirs = Vec::new();
    let mut provider_ids = Vec::new();

    for app in supported_provider_apps() {
        if let Some(provider) = providers.iter().find(|provider| {
            provider
                .active_apps
                .iter()
                .any(|active_app| active_app == app)
        }) {
            let result = apply_provider_for_app(provider, app)?;
            written_files.extend(result.written_files);
            backup_dirs.push(result.backup_dir);
            provider_ids.push(provider.id.clone());
        }
    }

    if provider_ids.is_empty() {
        return Err("还没有为任何客户端选择供应商".to_string());
    }

    Ok(ApplyProviderResult {
        provider_id: provider_ids.join(","),
        backup_dir: backup_dirs.join(","),
        written_files,
    })
}

#[tauri::command]
fn apply_provider_config(
    provider_id: String,
    app_id: String,
) -> Result<ApplyProviderResult, String> {
    let provider = list_providers()?
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| "未找到供应商".to_string())?;
    if !provider.enabled_apps.iter().any(|app| app == &app_id) {
        return Err("这个供应商没有启用当前客户端".to_string());
    }
    apply_provider_for_app(&provider, &app_id)
}

fn apply_provider_for_app(
    provider: &ProviderProfile,
    app_id: &str,
) -> Result<ApplyProviderResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    if !supported_provider_apps().contains(&app_id) {
        return Err("不支持的客户端".to_string());
    }
    if provider.provider_type == "official" {
        return Ok(ApplyProviderResult {
            provider_id: provider.id.clone(),
            backup_dir: String::new(),
            written_files: Vec::new(),
        });
    }
    let secrets = read_provider_secrets(&dirs)?;
    let api_key = secrets.get(&provider.id).cloned().unwrap_or_default();
    if api_key.is_empty() && !is_local_url(&provider.base_url) {
        return Err("当前供应商没有 API Key，请先补充密钥".to_string());
    }
    let preview = preview_provider_config(provider.clone())?;
    let custom_settings = materialized_provider_settings(provider, &api_key)?;
    let backup_dir = dirs.backups_dir.join(format!(
        "provider-{}-{}-{}",
        provider.id,
        app_id,
        OffsetDateTime::now_utc().unix_timestamp()
    ));
    fs::create_dir_all(&backup_dir).map_err(|err| format!("创建备份目录失败: {}", err))?;

    let (relative_path, content, env_content) = match app_id {
        "codex" => (
            "codex/config.toml".to_string(),
            custom_settings
                .as_ref()
                .and_then(|settings| settings.get("config"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or(preview.codex_toml),
            Some(format!(
                "OPENAI_API_KEY={}\nOPENAI_BASE_URL={}\n",
                api_key,
                ensure_v1_url(&provider.base_url)
            )),
        ),
        "claude-code" | "claude-desktop" => (
            format!("{}/settings.json", app_id),
            match custom_settings.as_ref() {
                Some(settings) => serde_json::to_string_pretty(settings)
                    .map_err(|err| format!("生成 Claude 配置失败: {}", err))?,
                None => preview
                    .claude_env_json
                    .replace("${AGENTDOCK_API_KEY}", &api_key),
            },
            Some(format!(
                "ANTHROPIC_AUTH_TOKEN={}\nANTHROPIC_BASE_URL={}\nANTHROPIC_MODEL={}\n",
                api_key,
                provider.base_url,
                provider_anthropic_model(provider)
            )),
        ),
        "antigravity" => (
            "antigravity/settings.json".to_string(),
            match custom_settings.as_ref() {
                Some(settings) => serde_json::to_string_pretty(settings)
                    .map_err(|err| format!("生成 Antigravity 配置失败: {}", err))?,
                None => preview
                    .gemini_env_json
                    .replace("${AGENTDOCK_API_KEY}", &api_key),
            },
            Some(format!(
                "GEMINI_API_KEY={}\nGOOGLE_GEMINI_BASE_URL={}\nGEMINI_MODEL={}\n",
                api_key, provider.base_url, provider.gemini_model
            )),
        ),
        "grok" => {
            let default_model = grok_default_model_from_settings(custom_settings.as_ref());
            let mut environment = format!("XAI_API_KEY={}\n", api_key);
            if let Some(default_model) = default_model {
                environment.push_str(&format!("GROK_DEFAULT_MODEL={}\n", default_model));
            }
            (
                "grok/config.toml".to_string(),
                custom_settings
                    .as_ref()
                    .and_then(|settings| settings.get("config"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                    .unwrap_or(grok_provider_toml(provider)?),
                Some(environment),
            )
        }
        "opencode" | "openclaw" | "hermes" => (
            format!("{}/provider.json", app_id),
            match custom_settings.as_ref() {
                Some(settings) => serde_json::to_string_pretty(settings)
                    .map_err(|err| format!("生成客户端配置失败: {}", err))?,
                None => serde_json::to_string_pretty(&serde_json::json!({
                    "provider": provider.name,
                    "baseUrl": provider.base_url,
                    "apiKey": api_key,
                    "model": provider.codex_model,
                    "apiFormat": provider.api_format,
                }))
                .map_err(|err| err.to_string())?,
            },
            None,
        ),
        _ => return Err("不支持的客户端".to_string()),
    };

    if app_id == "codex" {
        let auth = codex_auth_for_provider(custom_settings.as_ref(), &api_key);
        let managed_dir = dirs.managed_configs_dir.join("codex");
        let mut written_files = write_codex_config_pair(
            &managed_dir.join("config.toml"),
            &managed_dir.join("auth.json"),
            &content,
            &auth,
            &backup_dir,
            "managed-codex",
        )?;

        let user_dir = external_codex_home()?;
        written_files.extend(write_codex_config_pair(
            &user_dir.join("config.toml"),
            &user_dir.join("auth.json"),
            &content,
            &auth,
            &backup_dir,
            "user-codex",
        )?);

        if let Some(env_content) = env_content {
            let env_target = managed_dir.join("provider.env");
            fs::write(&env_target, env_content)
                .map_err(|err| format!("写入客户端密钥环境失败: {}", err))?;
            protect_secret_file(&env_target)?;
            written_files.push(env_target.display().to_string());
        }

        return Ok(ApplyProviderResult {
            provider_id: provider.id.clone(),
            backup_dir: backup_dir.display().to_string(),
            written_files,
        });
    }

    let target = dirs.managed_configs_dir.join(&relative_path);
    let content = if app_id == "grok" {
        preserve_toml_section(&target, &content, "mcp_servers")?
    } else {
        content
    };
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建配置目录失败: {}", err))?;
    }
    if target.exists() {
        let backup = backup_dir.join(relative_path.replace('/', "__"));
        fs::copy(&target, backup).map_err(|err| format!("备份旧配置失败: {}", err))?;
    }
    fs::write(&target, content).map_err(|err| format!("写入配置失败: {}", err))?;
    protect_secret_file(&target)?;
    let mut written_files = vec![target.display().to_string()];

    if let Some(env_content) = env_content {
        let env_target = dirs.managed_configs_dir.join(app_id).join("provider.env");
        fs::write(&env_target, env_content)
            .map_err(|err| format!("写入客户端密钥环境失败: {}", err))?;
        protect_secret_file(&env_target)?;
        written_files.push(env_target.display().to_string());
    }

    if app_id == "antigravity" {
        let settings_path = antigravity_gemini_settings_path(&dirs);
        let backup_path = backup_dir.join("managed-antigravity-gemini-settings.json");
        merge_antigravity_gemini_settings(&settings_path, Some(&backup_path))?;
        written_files.push(settings_path.display().to_string());
    }

    if app_id == "grok" {
        let sync = sync_mcp_servers()?;
        for path in sync.written_files {
            if !written_files.contains(&path) {
                written_files.push(path);
            }
        }
    }

    Ok(ApplyProviderResult {
        provider_id: provider.id.clone(),
        backup_dir: backup_dir.display().to_string(),
        written_files,
    })
}

#[tauri::command]
async fn run_diagnostics() -> Result<DiagnosticsReport, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let desktop = desktop_status()?;
    let mut checks = vec![diagnostic_check(
        "system-platform",
        "系统",
        "pass",
        "桌面运行环境",
        &format!(
            "AgentDock {} · {} {}",
            desktop.app_version,
            desktop.platform,
            env::consts::ARCH
        ),
        "",
    )];

    for (id, title, path) in [
        ("system-data-dir", "数据目录可写", &dirs.data_dir),
        ("system-config-dir", "配置目录可写", &dirs.config_dir),
        ("system-runtime-dir", "运行目录可写", &dirs.runtime_dir),
    ] {
        match probe_directory_writable(path) {
            Ok(()) => checks.push(diagnostic_check(
                id,
                "系统",
                "pass",
                title,
                &path.display().to_string(),
                "",
            )),
            Err(error) => checks.push(diagnostic_check(
                id,
                "系统",
                "error",
                title,
                &error,
                "检查目录权限或磁盘剩余空间",
            )),
        }
    }

    let catalog = match list_software_catalog().await {
        Ok(catalog) => catalog,
        Err(error) => {
            checks.push(diagnostic_check(
                "clients-version-service",
                "客户端",
                "warning",
                "无法检查客户端最新版本",
                &error,
                "检查网络后重新诊断",
            ));
            Vec::new()
        }
    };
    let installed_client_ids = desktop
        .clients
        .iter()
        .filter(|client| client.installed)
        .map(|client| client.id.clone())
        .collect::<HashSet<_>>();
    for client in desktop.clients.iter().filter(|client| client.installed) {
        let catalog_item = catalog.iter().find(|item| item.client_id == client.id);
        let detail = format!(
            "{} · {} · {}",
            client.version.as_deref().unwrap_or("版本未知"),
            if client.managed_by_agentdock {
                "AgentDock 托管"
            } else {
                "系统安装"
            },
            client.executable.as_deref().unwrap_or("启动路径未知")
        );
        checks.push(diagnostic_check(
            &format!("client-{}", client.id),
            "客户端",
            "pass",
            &format!("{} 可正常启动", client.name),
            &detail,
            "",
        ));
        if let Some(item) = catalog_item.filter(|item| item.update_available) {
            checks.push(diagnostic_check(
                &format!("client-{}-update", client.id),
                "客户端",
                "warning",
                &format!("{} 有新版本", client.name),
                &format!(
                    "当前 {}，最新 {}",
                    item.current_version.as_deref().unwrap_or("未知"),
                    item.latest_version.as_deref().unwrap_or("最新版")
                ),
                "前往客户端页面点击更新",
            ));
        }
        match client.config_path.as_deref() {
            Some(path) if Path::new(path).exists() => checks.push(diagnostic_check(
                &format!("client-{}-config", client.id),
                "客户端",
                "pass",
                &format!("{} 配置位置可用", client.name),
                path,
                "",
            )),
            _ => checks.push(diagnostic_check(
                &format!("client-{}-config", client.id),
                "客户端",
                "warning",
                &format!("{} 尚未生成配置", client.name),
                "客户端已安装，但还没有可读取的配置位置",
                "添加并启用一个供应商后会自动生成",
            )),
        }
    }

    let providers = list_providers()?;
    let relevant_providers = providers
        .iter()
        .filter(|provider| provider_is_active_for_diagnostics(provider, &installed_client_ids))
        .cloned()
        .collect::<Vec<_>>();
    let tests = join_all(relevant_providers.into_iter().map(|provider| async move {
        let result = test_provider(provider.id.clone()).await;
        (provider, result)
    }))
    .await;
    for (provider, result) in tests {
        let apps = provider
            .active_apps
            .iter()
            .filter(|app| installed_client_ids.contains(*app))
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        match result {
            Ok(result) if result.ok => checks.push(diagnostic_check(
                &format!("provider-{}", provider.id),
                "供应商",
                "pass",
                &format!("{} 连接正常", provider.name),
                &format!("{} · {} ms · {}", apps, result.latency_ms, result.message),
                "",
            )),
            Ok(result) => checks.push(diagnostic_check(
                &format!("provider-{}", provider.id),
                "供应商",
                "error",
                &format!("{} 无法使用", provider.name),
                &result.message,
                "编辑供应商的请求地址或 API Key 后重试",
            )),
            Err(error) => checks.push(diagnostic_check(
                &format!("provider-{}", provider.id),
                "供应商",
                "error",
                &format!("{} 检查失败", provider.name),
                &error,
                "检查供应商配置后重试",
            )),
        }
    }

    let mcp_servers = list_mcp_servers()?;
    if mcp_servers.is_empty() {
        checks.push(diagnostic_check(
            "mcp-empty",
            "MCP",
            "pass",
            "MCP 配置可用",
            "尚未添加 MCP 服务器；这不会影响客户端基础功能",
            "",
        ));
    }
    for server in mcp_servers.iter().filter(|server| {
        server.enabled
            && server
                .apps
                .iter()
                .any(|app| installed_client_ids.contains(app))
    }) {
        let validation = if server.apps.is_empty() {
            Err("没有关联任何客户端".to_string())
        } else if server.transport == "stdio" && server.command.trim().is_empty() {
            Err("缺少启动命令".to_string())
        } else if matches!(server.transport.as_str(), "http" | "sse") {
            validate_mcp_url(&server.command)
        } else if server.transport != "stdio" {
            Err(format!("不支持的传输类型 {}", server.transport))
        } else {
            Ok(())
        };
        match validation {
            Ok(()) => checks.push(diagnostic_check(
                &format!("mcp-{}", server.id),
                "MCP",
                "pass",
                &format!("{} 配置有效", server.name),
                &format!(
                    "{} · 已启用到 {} 个客户端",
                    server.transport.to_uppercase(),
                    server.apps.len()
                ),
                "",
            )),
            Err(error) => checks.push(diagnostic_check(
                &format!("mcp-{}", server.id),
                "MCP",
                "error",
                &format!("{} 配置无效", server.name),
                &error,
                "打开 MCP 页面修正配置",
            )),
        }
    }

    match get_usage_stats(Some(7)) {
        Ok(stats) if stats.errors.is_empty() => checks.push(diagnostic_check(
            "usage-data",
            "统计",
            "pass",
            "本地统计数据可读取",
            &if stats.sources.is_empty() {
                "尚未发现客户端会话记录".to_string()
            } else {
                format!("已识别：{}", stats.sources.join("、"))
            },
            "",
        )),
        Ok(stats) => checks.push(diagnostic_check(
            "usage-data",
            "统计",
            "warning",
            "部分统计数据无法读取",
            &stats.errors.join("；"),
            "检查对应会话目录的读取权限",
        )),
        Err(error) => checks.push(diagnostic_check(
            "usage-data",
            "统计",
            "warning",
            "统计检查失败",
            &error,
            "重新运行诊断",
        )),
    }

    Ok(finalize_diagnostics(checks))
}

fn provider_is_active_for_diagnostics(
    provider: &ProviderProfile,
    installed_client_ids: &HashSet<String>,
) -> bool {
    provider
        .active_apps
        .iter()
        .any(|app| installed_client_ids.contains(app))
}

fn diagnostic_check(
    id: &str,
    category: &str,
    status: &str,
    title: &str,
    detail: &str,
    action: &str,
) -> DiagnosticCheck {
    DiagnosticCheck {
        id: id.to_string(),
        category: category.to_string(),
        status: status.to_string(),
        title: title.to_string(),
        detail: detail.to_string(),
        action: action.to_string(),
    }
}

fn finalize_diagnostics(checks: Vec<DiagnosticCheck>) -> DiagnosticsReport {
    let passed = checks.iter().filter(|check| check.status == "pass").count();
    let warnings = checks
        .iter()
        .filter(|check| check.status == "warning")
        .count();
    let failed = checks
        .iter()
        .filter(|check| check.status == "error")
        .count();
    let mut category_penalties = BTreeMap::<&str, usize>::new();
    for check in &checks {
        let penalty = match check.status.as_str() {
            "error" => 20,
            "warning" => 5,
            _ => 0,
        };
        if penalty > 0 {
            category_penalties
                .entry(check.category.as_str())
                .and_modify(|current| *current = (*current).max(penalty))
                .or_insert(penalty);
        }
    }
    let penalty = category_penalties.values().sum::<usize>();
    DiagnosticsReport {
        generated_at: now_rfc3339(),
        score: 100usize.saturating_sub(penalty).min(100) as u8,
        passed,
        warnings,
        failed,
        checks,
    }
}

fn probe_directory_writable(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| format!("无法创建 {}: {}", path.display(), err))?;
    let probe = path.join(format!(
        ".agentdock-diagnostic-{}-{}",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::write(&probe, b"ok").map_err(|err| format!("无法写入 {}: {}", path.display(), err))?;
    fs::remove_file(&probe).map_err(|err| format!("无法清理诊断文件: {}", err))
}

#[tauri::command]
fn launch_client(
    client_id: String,
    working_directory: Option<String>,
    request_id: String,
) -> Result<LaunchClientResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    validate_launch_request(&client_id, &request_id)?;
    let client = cached_client_for_launch(&client_id)
        .or_else(|| {
            refresh_client_detection()
                .into_iter()
                .find(|client| client.id == client_id)
        })
        .ok_or_else(|| "未找到客户端".to_string())?;
    let executable = client
        .executable
        .as_ref()
        .ok_or_else(|| "客户端还未安装".to_string())?;
    let uses_working_directory = client_uses_working_directory(&client_id, Path::new(executable));
    let working_directory = if uses_working_directory {
        Some(validate_working_directory(
            working_directory.as_deref().unwrap_or_default(),
        )?)
    } else {
        None
    };

    let active_provider = list_providers()?.into_iter().find(|provider| {
        provider
            .active_apps
            .iter()
            .any(|active_app| active_app == &client_id)
    });
    let environment = match active_provider.as_ref() {
        Some(provider) => {
            ensure_provider_launch_config(&dirs, &client_id, provider)?;
            provider_launch_environment(&dirs, &client_id, provider)?
        }
        None => BTreeMap::new(),
    };
    if client_id == "antigravity"
        && active_provider
            .as_ref()
            .is_some_and(|provider| provider.provider_type != "official")
        && !antigravity_proxy_runtime_ready(executable)
    {
        return Err(
            "自定义 Antigravity 代理需要新版客户端运行时，请先在软件页面更新 Antigravity CLI"
                .to_string(),
        );
    }

    #[cfg(target_os = "macos")]
    if is_macos_app_bundle(Path::new(executable)) {
        launch_macos_app_bundle(Path::new(executable))?;
    } else {
        launch_in_terminal(
            &dirs,
            &client_id,
            executable,
            &environment,
            working_directory
                .as_deref()
                .ok_or_else(|| "启动命令行客户端前，请先选择项目目录".to_string())?,
            &request_id,
        )?;
    }
    #[cfg(not(target_os = "macos"))]
    if uses_working_directory {
        launch_in_terminal(
            &dirs,
            &client_id,
            executable,
            &environment,
            working_directory
                .as_deref()
                .ok_or_else(|| "启动命令行客户端前，请先选择项目目录".to_string())?,
            &request_id,
        )?;
    } else {
        launch_desktop_client(Path::new(executable))?;
    }

    let working_directory_display = working_directory
        .as_ref()
        .map(|path| path.display().to_string());

    Ok(LaunchClientResult {
        launched: true,
        message: match (active_provider, working_directory_display.as_deref()) {
            (Some(provider), Some(directory)) => {
                format!(
                    "已使用 {} 在 {} 启动 {}",
                    provider.name, directory, client.name
                )
            }
            (None, Some(directory)) => format!("已在 {} 启动 {}", directory, client.name),
            (Some(provider), None) => format!("已使用 {} 启动 {}", provider.name, client.name),
            (None, None) => format!("已启动 {}", client.name),
        },
        client_id,
        working_directory: working_directory_display,
        request_id,
    })
}

fn validate_launch_request(client_id: &str, request_id: &str) -> Result<(), String> {
    if !supported_provider_apps().contains(&client_id) {
        return Err("不支持的客户端".to_string());
    }
    if !(8..=80).contains(&request_id.len())
        || !request_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err("客户端启动请求无效，请重试".to_string());
    }
    Ok(())
}

fn client_uses_working_directory(client_id: &str, executable: &Path) -> bool {
    if client_id == "claude-desktop" {
        return false;
    }
    #[cfg(target_os = "macos")]
    if is_macos_app_bundle(executable) {
        return false;
    }
    true
}

#[cfg(not(target_os = "macos"))]
fn launch_desktop_client(path: &Path) -> Result<(), String> {
    Command::new(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("无法启动 {}: {}", path.display(), error))
}

#[cfg(target_os = "macos")]
fn is_macos_app_bundle(path: &Path) -> bool {
    path.is_dir()
        && path.extension().and_then(|value| value.to_str()) == Some("app")
        && path.join("Contents/Info.plist").is_file()
}

#[cfg(target_os = "macos")]
fn launch_macos_app_bundle(path: &Path) -> Result<(), String> {
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|error| format!("无法启动 {}: {}", path.display(), error))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("启动 {} 失败: {}", path.display(), status))
    }
}

fn antigravity_proxy_runtime_ready(executable: &str) -> bool {
    fs::read_to_string(executable)
        .map(|content| content.contains("GOOGLE_GEMINI_BASE_URL") && content.contains("gemini-cli"))
        .unwrap_or(false)
}

fn ensure_provider_launch_config(
    dirs: &AgentDockDirs,
    app_id: &str,
    provider: &ProviderProfile,
) -> Result<(), String> {
    if app_id == "codex" && provider.provider_type != "official" {
        let codex_home = dirs.managed_configs_dir.join("codex");
        if !codex_home_is_ready(&codex_home) {
            apply_provider_for_app(provider, app_id)?;
        }
    }
    if app_id == "antigravity" && provider.provider_type != "official" {
        merge_antigravity_gemini_settings(&antigravity_gemini_settings_path(dirs), None)?;
    }
    Ok(())
}

fn codex_home_is_ready(path: &Path) -> bool {
    path.join("config.toml").is_file() && path.join("auth.json").is_file()
}

fn antigravity_gemini_settings_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.managed_configs_dir
        .join("antigravity")
        .join("gemini-cli-home")
        .join(".gemini")
        .join("settings.json")
}

fn merge_antigravity_gemini_settings(
    path: &Path,
    backup_path: Option<&Path>,
) -> Result<bool, String> {
    let existed = path.exists();
    let mut settings = if existed {
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("读取 Antigravity 运行配置失败: {}", err))?;
        serde_json::from_str::<serde_json::Value>(&raw)
            .map_err(|err| format!("解析 Antigravity 运行配置失败: {}", err))?
    } else {
        serde_json::json!({})
    };
    let root = settings
        .as_object_mut()
        .ok_or_else(|| "Antigravity 运行配置必须是 JSON 对象".to_string())?;
    let mut changed = false;
    let security = root
        .entry("security".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !security.is_object() {
        *security = serde_json::json!({});
        changed = true;
    }
    let auth = security
        .as_object_mut()
        .expect("security was normalized to an object")
        .entry("auth".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !auth.is_object() {
        *auth = serde_json::json!({});
        changed = true;
    }
    let auth = auth
        .as_object_mut()
        .expect("auth was normalized to an object");
    if auth.get("selectedType").and_then(serde_json::Value::as_str) != Some("gemini-api-key") {
        auth.insert(
            "selectedType".to_string(),
            serde_json::Value::String("gemini-api-key".to_string()),
        );
        changed = true;
    }

    let general = root
        .entry("general".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !general.is_object() {
        *general = serde_json::json!({});
        changed = true;
    }
    let general = general
        .as_object_mut()
        .expect("general was normalized to an object");
    if general
        .get("maxAttempts")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        general.insert("maxAttempts".to_string(), serde_json::json!(1));
        changed = true;
    }
    if general
        .get("retryFetchErrors")
        .and_then(serde_json::Value::as_bool)
        != Some(false)
    {
        general.insert("retryFetchErrors".to_string(), serde_json::json!(false));
        changed = true;
    }

    if !changed {
        return Ok(false);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Antigravity 认证配置目录失败: {}", err))?;
    }
    if existed {
        if let Some(backup_path) = backup_path {
            fs::copy(path, backup_path)
                .map_err(|err| format!("备份 Antigravity 运行配置失败: {}", err))?;
        }
    }
    write_json(path, &settings).map_err(|err| format!("写入 Antigravity 运行配置失败: {}", err))?;
    Ok(true)
}

fn provider_launch_environment(
    dirs: &AgentDockDirs,
    app_id: &str,
    provider: &ProviderProfile,
) -> Result<BTreeMap<String, String>, String> {
    let mut environment = BTreeMap::new();
    if provider.provider_type == "official" {
        return Ok(environment);
    }

    let api_key = read_provider_secrets(dirs)?
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();
    if api_key.is_empty() && !is_local_url(&provider.base_url) {
        return Err("当前供应商没有 API Key，请先补充密钥".to_string());
    }

    let materialized_settings = materialized_provider_settings(provider, &api_key)?;
    if let Some(settings) = materialized_settings.as_ref() {
        let section = if app_id == "codex" { "auth" } else { "env" };
        if let Some(values) = settings.get(section).and_then(serde_json::Value::as_object) {
            for (key, value) in values {
                if let Some(value) = value.as_str() {
                    environment.insert(key.clone(), value.to_string());
                }
            }
        }
    }

    match app_id {
        "codex" => {
            environment
                .entry("OPENAI_API_KEY".to_string())
                .or_insert(api_key);
            environment.insert(
                "CODEX_HOME".to_string(),
                dirs.managed_configs_dir.join("codex").display().to_string(),
            );
        }
        "claude-code" | "claude-desktop" => {
            environment
                .entry("ANTHROPIC_AUTH_TOKEN".to_string())
                .or_insert(api_key);
            environment
                .entry("ANTHROPIC_BASE_URL".to_string())
                .or_insert_with(|| provider.base_url.clone());
            environment
                .entry("ANTHROPIC_MODEL".to_string())
                .or_insert_with(|| provider_anthropic_model(provider));
            environment
                .entry("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string())
                .or_insert_with(|| provider.claude_sonnet_model.clone());
            environment
                .entry("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string())
                .or_insert_with(|| provider.claude_haiku_model.clone());
            environment
                .entry("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string())
                .or_insert_with(|| provider.claude_opus_model.clone());
            if provider.preset_id == "volcengine-agentplan" {
                for key in [
                    "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
                    "ANTHROPIC_DEFAULT_FABLE_MODEL_NAME",
                ] {
                    environment
                        .entry(key.to_string())
                        .or_insert_with(|| provider_anthropic_model(provider));
                }
                environment
                    .entry("ANTHROPIC_DEFAULT_FABLE_MODEL".to_string())
                    .or_insert_with(|| provider.claude_sonnet_model.clone());
            }
            if app_id == "claude-code" {
                environment.insert(
                    "CLAUDE_CONFIG_DIR".to_string(),
                    dirs.managed_configs_dir
                        .join("claude-code")
                        .display()
                        .to_string(),
                );
            }
        }
        "antigravity" => {
            environment
                .entry("GEMINI_API_KEY".to_string())
                .or_insert(api_key);
            environment
                .entry("GOOGLE_GEMINI_BASE_URL".to_string())
                .or_insert_with(|| provider.base_url.clone());
            environment
                .entry("GEMINI_MODEL".to_string())
                .or_insert_with(|| provider.gemini_model.clone());
            environment.insert(
                "GEMINI_CLI_HOME".to_string(),
                dirs.managed_configs_dir
                    .join("antigravity")
                    .join("gemini-cli-home")
                    .display()
                    .to_string(),
            );
        }
        "grok" => {
            environment
                .entry("XAI_API_KEY".to_string())
                .or_insert(api_key);
            environment.remove("GROK_DEFAULT_MODEL");
            if let Some(default_model) =
                grok_default_model_from_settings(materialized_settings.as_ref())
            {
                environment.insert("GROK_DEFAULT_MODEL".to_string(), default_model);
            }
            environment.insert(
                "GROK_HOME".to_string(),
                dirs.managed_configs_dir.join("grok").display().to_string(),
            );
        }
        "opencode" => {
            environment.insert("OPENAI_API_KEY".to_string(), api_key);
            environment.insert("OPENAI_BASE_URL".to_string(), provider.base_url.clone());
            environment.insert(
                "OPENCODE_CONFIG".to_string(),
                dirs.managed_configs_dir
                    .join("opencode/provider.json")
                    .display()
                    .to_string(),
            );
        }
        "openclaw" => {
            environment.insert("OPENAI_API_KEY".to_string(), api_key);
            environment.insert("OPENAI_BASE_URL".to_string(), provider.base_url.clone());
            environment.insert(
                "OPENCLAW_CONFIG_PATH".to_string(),
                dirs.managed_configs_dir
                    .join("openclaw/provider.json")
                    .display()
                    .to_string(),
            );
            environment.insert(
                "OPENCLAW_STATE_DIR".to_string(),
                dirs.managed_configs_dir
                    .join("openclaw/state")
                    .display()
                    .to_string(),
            );
        }
        "hermes" => {
            environment.insert("OPENAI_API_KEY".to_string(), api_key);
            environment.insert("OPENAI_BASE_URL".to_string(), provider.base_url.clone());
            environment.insert(
                "HERMES_CONFIG_PATH".to_string(),
                dirs.managed_configs_dir
                    .join("hermes/provider.json")
                    .display()
                    .to_string(),
            );
            environment.insert(
                "HERMES_HOME".to_string(),
                dirs.clients_dir.join("hermes/home").display().to_string(),
            );
        }
        _ => return Err("不支持的客户端".to_string()),
    }

    Ok(environment)
}

fn launch_in_terminal(
    dirs: &AgentDockDirs,
    client_id: &str,
    executable: &str,
    environment: &BTreeMap<String, String>,
    working_directory: &Path,
    request_id: &str,
) -> Result<(), String> {
    let settings = read_app_settings(dirs)?;
    #[cfg(target_os = "macos")]
    {
        let launcher = write_unix_launcher(
            dirs,
            client_id,
            executable,
            environment,
            working_directory,
            request_id,
        )?;
        return launch_macos_terminal_with_fallback(
            &settings.preferred_terminal,
            &launcher,
            std::time::Duration::from_secs(20),
        );
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let launcher = write_powershell_launcher(
            dirs,
            client_id,
            executable,
            environment,
            working_directory,
            request_id,
        )?;
        let powershell_args = [
            "-NoExit",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ];
        let spawn_result = match settings.preferred_terminal.as_str() {
            "cmd" => Command::new("cmd.exe")
                .creation_flags(0x00000010)
                .args(["/K", "powershell.exe"])
                .args(powershell_args)
                .arg(&launcher)
                .spawn(),
            "wt" if find_executable("wt").is_some() => Command::new("wt.exe")
                .args(["powershell.exe"])
                .args(powershell_args)
                .arg(&launcher)
                .spawn(),
            _ => Command::new("powershell.exe")
                .creation_flags(0x00000010)
                .args(powershell_args)
                .arg(&launcher)
                .spawn(),
        };
        spawn_result.map_err(|err| format!("打开终端失败: {}", err))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let launcher = write_unix_launcher(
            dirs,
            client_id,
            executable,
            environment,
            working_directory,
            request_id,
        )?;
        let terminal_args = |terminal: &str| -> &'static [&'static str] {
            match terminal {
                "gnome-terminal" => &["--"],
                _ => &["-e"],
            }
        };
        let mut terminals = vec![settings.preferred_terminal.as_str()];
        for fallback in terminal_options() {
            if !terminals.contains(fallback) {
                terminals.push(fallback);
            }
        }
        for terminal in terminals {
            if find_executable(terminal).is_some() {
                Command::new(terminal)
                    .args(terminal_args(terminal))
                    .arg(&launcher)
                    .spawn()
                    .map_err(|err| format!("打开终端失败: {}", err))?;
                return Ok(());
            }
        }
        Err("没有找到可用的终端程序".to_string())
    }
}

#[cfg(target_os = "macos")]
fn launch_macos_terminal_with_fallback(
    preferred: &str,
    launcher: &Path,
    confirmation_timeout: std::time::Duration,
) -> Result<(), String> {
    let preferred_result = launch_macos_terminal(preferred, launcher);
    if preferred_result.is_ok() {
        if wait_for_launcher_consumption(launcher, confirmation_timeout) {
            return Ok(());
        }
        let _ = fs::remove_file(launcher);
        return Err(format!(
            "首选终端已打开，但客户端命令未执行；请检查终端是否允许打开 {}",
            launcher.display()
        ));
    }

    let preferred_error = preferred_result.unwrap_err();
    if should_fallback_to_macos_terminal(preferred, true) {
        let fallback_result = launch_macos_terminal("terminal", launcher);
        if fallback_result.is_ok() && wait_for_launcher_consumption(launcher, confirmation_timeout)
        {
            return Ok(());
        }
        let _ = fs::remove_file(launcher);
        return match fallback_result {
            Err(error) => Err(format!(
                "{}；系统 Terminal 回退失败：{}",
                preferred_error, error
            )),
            Ok(()) => Err(format!(
                "终端已打开，但客户端命令未执行；请检查 Terminal 是否允许打开 {}",
                launcher.display()
            )),
        };
    }

    let _ = fs::remove_file(launcher);
    Err(preferred_error)
}

#[cfg(target_os = "macos")]
fn should_fallback_to_macos_terminal(preferred: &str, preferred_launch_failed: bool) -> bool {
    preferred != "terminal" && preferred_launch_failed
}

#[cfg(target_os = "macos")]
fn wait_for_launcher_consumption(path: &Path, timeout: std::time::Duration) -> bool {
    let started = Instant::now();
    while path.exists() && started.elapsed() < timeout {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    !path.exists()
}

#[cfg(target_os = "macos")]
fn launch_macos_terminal(preferred: &str, launcher: &Path) -> Result<(), String> {
    match preferred {
        "iterm2" => launch_macos_iterm(launcher),
        "alacritty" => launch_macos_terminal_app("Alacritty", launcher, true),
        "kitty" => launch_macos_terminal_app("kitty", launcher, false),
        "ghostty" => launch_macos_terminal_app("Ghostty", launcher, true),
        "wezterm" => launch_macos_terminal_app("WezTerm", launcher, true),
        _ => launch_macos_terminal_file("Terminal", launcher),
    }
}

#[cfg(target_os = "macos")]
fn launch_macos_terminal_file(app: &str, launcher: &Path) -> Result<(), String> {
    let status = Command::new("open")
        .arg("-a")
        .arg(app)
        .arg(launcher)
        .status()
        .map_err(|err| format!("启动 {} 失败: {}", app, err))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("启动 {} 失败: {}", app, status))
    }
}

#[cfg(target_os = "macos")]
fn macos_launcher_command(launcher: &Path) -> String {
    format!("exec sh {}", shell_quote(&launcher.to_string_lossy()))
}

#[cfg(target_os = "macos")]
fn macos_iterm_script() -> &'static str {
    r#"on run argv
    set launcher_command to item 1 of argv
    set had_windows to false
    if application "iTerm" is running then
        tell application "iTerm" to set had_windows to (count of windows) > 0
    end if

    tell application "iTerm"
        if had_windows then
            set launched_window to create window with default profile
            tell current session of launched_window to write text launcher_command
            activate
            return
        end if

        activate
        set waited to 0
        repeat while (count of windows) = 0 and waited < 100
            delay 0.1
            set waited to waited + 1
        end repeat
        if (count of windows) = 0 then error "iTerm2 did not create a default window"
        tell current session of current window to write text launcher_command
    end tell
end run"#
}

#[cfg(target_os = "macos")]
fn launch_macos_iterm(launcher: &Path) -> Result<(), String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(macos_iterm_script())
        .arg("--")
        .arg(macos_launcher_command(launcher))
        .output()
        .map_err(|err| format!("启动 iTerm2 失败: {}", err))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "启动 iTerm2 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(target_os = "macos")]
fn launch_macos_terminal_app(app: &str, launcher: &Path, use_e: bool) -> Result<(), String> {
    let mut command = Command::new("open");
    command.args(["-na", app, "--args"]);
    if use_e {
        command.arg("-e");
    }
    let output = command
        .arg("sh")
        .arg(launcher)
        .output()
        .map_err(|err| format!("启动 {} 失败: {}", app, err))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "启动 {} 失败: {}",
            app,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(unix)]
fn write_unix_launcher(
    dirs: &AgentDockDirs,
    client_id: &str,
    executable: &str,
    environment: &BTreeMap<String, String>,
    working_directory: &Path,
    request_id: &str,
) -> Result<PathBuf, String> {
    let path = launch_script_path(&dirs.runtime_dir, client_id, request_id, "command");
    let content = unix_launcher_content(executable, environment, working_directory)?;
    fs::write(&path, content).map_err(|err| format!("写入客户端启动器失败: {}", err))?;
    make_private_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn unix_launcher_content(
    executable: &str,
    environment: &BTreeMap<String, String>,
    working_directory: &Path,
) -> Result<String, String> {
    let mut content = String::from("#!/bin/sh\n");
    content.push_str(&format!(
        "cd -- {}\n",
        shell_quote(&working_directory.to_string_lossy())
    ));
    for (key, value) in environment {
        validate_launch_value(key, value)?;
        content.push_str(&format!("export {}={}\n", key, shell_quote(value)));
    }
    content.push_str("/bin/rm -f -- \"$0\"\n");
    content.push_str(&format!("exec {} \"$@\"\n", shell_quote(executable)));
    Ok(content)
}

#[cfg(windows)]
fn write_powershell_launcher(
    dirs: &AgentDockDirs,
    client_id: &str,
    executable: &str,
    environment: &BTreeMap<String, String>,
    working_directory: &Path,
    request_id: &str,
) -> Result<PathBuf, String> {
    let path = launch_script_path(&dirs.runtime_dir, client_id, request_id, "ps1");
    let mut content = format!(
        "Set-Location -LiteralPath {}\r\n",
        powershell_quote(&working_directory.to_string_lossy())
    );
    for (key, value) in environment {
        validate_launch_value(key, value)?;
        content.push_str(&format!("$env:{} = {}\r\n", key, powershell_quote(value)));
    }
    content.push_str(
        "Remove-Item -LiteralPath $PSCommandPath -Force -ErrorAction SilentlyContinue\r\n",
    );
    content.push_str(&format!("& {}\r\n", powershell_quote(executable)));
    fs::write(&path, content).map_err(|err| format!("写入客户端启动器失败: {}", err))?;
    Ok(path)
}

fn launch_script_path(
    runtime_dir: &Path,
    client_id: &str,
    request_id: &str,
    extension: &str,
) -> PathBuf {
    runtime_dir.join(format!(
        "launch-{}-{}.{}",
        slugify(client_id),
        slugify(request_id),
        extension
    ))
}

fn validate_launch_value(key: &str, value: &str) -> Result<(), String> {
    if key.is_empty()
        || !key.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
        || value.contains('\0')
    {
        return Err("供应商启动参数包含无效字符".to_string());
    }
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(windows)]
fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[tauri::command]
fn open_path(path: String) -> Result<OperationResult, String> {
    let requested = PathBuf::from(path);
    let target = existing_directory_for_path(&requested)
        .ok_or_else(|| "无法找到可打开的配置目录".to_string())?;

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(&target);
        command
    };
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(&target);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(&target);
        command
    };

    command
        .spawn()
        .map_err(|err| format!("打开路径失败: {}", err))?;
    Ok(OperationResult {
        ok: true,
        message: if requested == target {
            format!("已打开 {}", target.display())
        } else {
            format!("目标尚未生成，已打开最近的目录 {}", target.display())
        },
    })
}

#[tauri::command]
fn open_external(url: String) -> Result<OperationResult, String> {
    let parsed = reqwest::Url::parse(url.trim()).map_err(|_| "链接格式无效".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("只允许打开 HTTP 或 HTTPS 链接".to_string());
    }
    let target = parsed.as_str();

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(target);
        command
    };
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("explorer");
        command.arg(target);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(target);
        command
    };

    command
        .spawn()
        .map_err(|error| format!("打开链接失败: {}", error))?;
    Ok(OperationResult {
        ok: true,
        message: "已在浏览器中打开链接".to_string(),
    })
}

#[tauri::command]
fn list_skills() -> Result<Vec<SkillRecord>, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let path = skills_path(&dirs);
    let mut skills: Vec<SkillRecord> = read_json_or_seed(&path, default_skills())?;
    let mut migrated = false;
    for skill in &mut skills {
        migrated |= migrate_app_ids(&mut skill.apps);
    }
    if migrated {
        write_json(&path, &skills)?;
    }
    Ok(skills)
}

#[tauri::command]
fn install_skill(input: SkillInstallInput) -> Result<SkillRecord, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut skills = list_skills()?;
    let now = now_rfc3339();
    let id = slugify(&input.id);
    let skill_dir = active_skills_dir(&dirs)?.join(&id);
    fs::create_dir_all(&skill_dir).map_err(|err| format!("创建 Skill 目录失败: {}", err))?;
    fs::write(
        skill_dir.join("SKILL.md"),
        format!(
            "---\nname: {}\ndescription: {}\n---\n\nManaged by AgentDock.\n",
            input.name.clone().unwrap_or_else(|| id.clone()),
            input
                .description
                .clone()
                .unwrap_or_else(|| "AgentDock managed skill".to_string())
        ),
    )
    .map_err(|err| format!("写入 Skill 文件失败: {}", err))?;

    let record = SkillRecord {
        id: id.clone(),
        name: input.name.unwrap_or_else(|| id.clone()),
        description: input
            .description
            .unwrap_or_else(|| "AgentDock managed skill".to_string()),
        source: input.source.unwrap_or_else(|| "local".to_string()),
        installed: true,
        apps: input
            .apps
            .unwrap_or_else(|| vec!["codex".to_string(), "claude-code".to_string()]),
        updated_at: now,
    };

    skills.retain(|skill| skill.id != id);
    skills.push(record.clone());
    write_json(&skills_path(&dirs), &skills)?;
    Ok(record)
}

#[tauri::command]
fn toggle_skill_app(skill_id: String, app: String, enabled: bool) -> Result<SkillRecord, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut skills = list_skills()?;
    let skill = skills
        .iter_mut()
        .find(|skill| skill.id == skill_id)
        .ok_or_else(|| "未找到 Skill".to_string())?;

    if enabled && !skill.apps.iter().any(|item| item == &app) {
        skill.apps.push(app);
    } else if !enabled {
        skill.apps.retain(|item| item != &app);
    }
    skill.updated_at = now_rfc3339();
    let updated = skill.clone();
    write_json(&skills_path(&dirs), &skills)?;
    Ok(updated)
}

#[tauri::command]
fn uninstall_skill(skill_id: String) -> Result<SkillRecord, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut skills = list_skills()?;
    let skill = skills
        .iter_mut()
        .find(|skill| skill.id == skill_id)
        .ok_or_else(|| "未找到 Skill".to_string())?;

    let skill_dir = active_skills_dir(&dirs)?.join(&skill.id);
    if skill_dir.exists() {
        let backup_dir = dirs.backups_dir.join(format!(
            "skill-{}-{}",
            skill.id,
            OffsetDateTime::now_utc().unix_timestamp()
        ));
        copy_dir_all(&skill_dir, &backup_dir)?;
        fs::remove_dir_all(&skill_dir).map_err(|err| format!("卸载 Skill 失败: {}", err))?;
    }
    skill.installed = false;
    skill.updated_at = now_rfc3339();
    let updated = skill.clone();
    write_json(&skills_path(&dirs), &skills)?;
    Ok(updated)
}

#[tauri::command]
fn sync_skills() -> Result<SyncResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let settings = read_app_settings(&dirs)?;
    let skills_dir = active_skills_dir(&dirs)?;
    let skills = list_skills()?;
    let mut written_files = Vec::new();

    for skill in skills.iter().filter(|skill| skill.installed) {
        let source = skills_dir.join(&skill.id).join("SKILL.md");
        if !source.exists() {
            continue;
        }
        for app in &skill.apps {
            let target = dirs
                .managed_configs_dir
                .join(app)
                .join("skills")
                .join(&skill.id)
                .join("SKILL.md");
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("创建 Skill 同步目录失败: {}", err))?;
            }
            sync_skill_file(&source, &target, &settings.skill_sync_method)?;
            written_files.push(target.display().to_string());
            if app == "grok" {
                if let Some(home) = dirs_home() {
                    let user_target = home.join(".grok/skills").join(&skill.id).join("SKILL.md");
                    if let Some(parent) = user_target.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|err| format!("创建 Grok Skill 目录失败: {}", err))?;
                    }
                    sync_skill_file(&source, &user_target, &settings.skill_sync_method)?;
                    written_files.push(user_target.display().to_string());
                }
            }
        }
    }

    Ok(SyncResult {
        message: format!("已同步 {} 个 Skill 配置文件", written_files.len()),
        written_files,
    })
}

fn sync_skill_file(source: &Path, target: &Path, method: &str) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建 Skill 同步目录失败: {}", err))?;
    }
    if fs::symlink_metadata(target).is_ok() {
        fs::remove_file(target).map_err(|err| format!("替换旧 Skill 同步文件失败: {}", err))?;
    }
    if method == "symlink" {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(source, target)
                .map_err(|err| format!("创建 Skill 符号链接失败: {}", err))?;
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(source, target).map_err(|err| {
                format!(
                    "创建 Skill 符号链接失败，请启用 Windows 开发者模式或改用复制文件: {}",
                    err
                )
            })?;
        }
        #[cfg(not(any(unix, windows)))]
        {
            return Err("当前系统不支持 Skill 符号链接".to_string());
        }
    } else {
        fs::copy(source, target).map_err(|err| format!("同步 Skill 失败: {}", err))?;
    }
    Ok(())
}

#[tauri::command]
fn list_mcp_servers() -> Result<Vec<McpServerRecord>, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let path = mcp_servers_path(&dirs);
    let mut servers: Vec<McpServerRecord> = read_json_or_seed(&path, default_mcp_servers())?;
    let mut migrated = false;
    for server in &mut servers {
        migrated |= migrate_app_ids(&mut server.apps);
    }
    if migrated {
        write_json(&path, &servers)?;
    }
    Ok(servers)
}

#[tauri::command]
fn upsert_mcp_server(input: McpServerInput) -> Result<McpServerRecord, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut servers = list_mcp_servers()?;
    let id = input.id.trim().to_string();
    if id.is_empty() {
        return Err("MCP 服务器 ID 不能为空".to_string());
    }
    let record = McpServerRecord {
        id: id.clone(),
        name: input.name.unwrap_or_else(|| id.clone()),
        description: input.description.unwrap_or_default(),
        homepage: input.homepage.unwrap_or_default(),
        docs: input.docs.unwrap_or_default(),
        tags: input.tags.unwrap_or_default(),
        transport: input.transport.unwrap_or_else(|| "stdio".to_string()),
        command: input.command.unwrap_or_else(|| "npx".to_string()),
        args: input.args.unwrap_or_default(),
        env: input.env.unwrap_or_default(),
        headers: input.headers.unwrap_or_default(),
        cwd: input.cwd.unwrap_or_default(),
        extra: input.extra.unwrap_or_default(),
        apps: input
            .apps
            .unwrap_or_else(|| vec!["codex".to_string(), "claude-code".to_string()]),
        enabled: input.enabled.unwrap_or(true),
        updated_at: now_rfc3339(),
    };

    servers.retain(|server| server.id != id);
    servers.push(record.clone());
    write_json(&mcp_servers_path(&dirs), &servers)?;
    sync_mcp_servers()?;
    Ok(record)
}

#[tauri::command]
fn toggle_mcp_app(
    server_id: String,
    app: String,
    enabled: bool,
) -> Result<McpServerRecord, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut servers = list_mcp_servers()?;
    let server = servers
        .iter_mut()
        .find(|server| server.id == server_id)
        .ok_or_else(|| "未找到 MCP 服务器".to_string())?;

    if enabled && !server.apps.iter().any(|item| item == &app) {
        server.apps.push(app);
    } else if !enabled {
        server.apps.retain(|item| item != &app);
    }
    if enabled {
        server.enabled = true;
    }
    server.updated_at = now_rfc3339();
    let updated = server.clone();
    write_json(&mcp_servers_path(&dirs), &servers)?;
    sync_mcp_servers()?;
    Ok(updated)
}

#[tauri::command]
fn toggle_mcp_server(server_id: String, enabled: bool) -> Result<McpServerRecord, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut servers = list_mcp_servers()?;
    let server = servers
        .iter_mut()
        .find(|server| server.id == server_id)
        .ok_or_else(|| "未找到 MCP 服务器".to_string())?;
    server.enabled = enabled;
    server.updated_at = now_rfc3339();
    let updated = server.clone();
    write_json(&mcp_servers_path(&dirs), &servers)?;
    sync_mcp_servers()?;
    Ok(updated)
}

#[tauri::command]
async fn list_mcp_tools(server_id: String) -> Result<McpToolsResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let server = list_mcp_servers()?
        .into_iter()
        .find(|server| server.id == server_id)
        .ok_or_else(|| "未找到 MCP 服务器".to_string())?;
    let started = Instant::now();
    let tools = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        discover_mcp_tools(&dirs, &server),
    )
    .await
    .map_err(|_| "连接 MCP 服务器超时，请检查网络或服务器配置".to_string())??;

    Ok(McpToolsResult {
        server_id: server.id,
        server_name: server.name,
        transport: server.transport,
        tools: tools.into_iter().map(mcp_tool_info).collect(),
        latency_ms: started.elapsed().as_millis(),
    })
}

async fn discover_mcp_tools(
    dirs: &AgentDockDirs,
    server: &McpServerRecord,
) -> Result<Vec<Tool>, String> {
    match server.transport.as_str() {
        "stdio" => {
            let command = mcp_stdio_command(dirs, server)?;
            let (transport, _) = TokioChildProcess::builder(command)
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("启动 MCP 服务器失败: {}", error))?;
            let client =
                ().serve(transport)
                    .await
                    .map_err(|error| format!("MCP 初始化失败: {}", error))?;
            let tools = client
                .list_all_tools()
                .await
                .map_err(|error| format!("读取 MCP 工具失败: {}", error))?;
            let _ = client.cancel().await;
            Ok(tools)
        }
        "http" => {
            validate_mcp_url(&server.command)?;
            let transport = StreamableHttpClientTransport::with_client(
                mcp_http_client(&server.headers)?,
                StreamableHttpClientTransportConfig::with_uri(server.command.clone()),
            );
            let client = ()
                .serve(transport)
                .await
                .map_err(|error| format!("MCP HTTP 初始化失败: {}", error))?;
            let tools = client
                .list_all_tools()
                .await
                .map_err(|error| format!("读取 MCP 工具失败: {}", error))?;
            let _ = client.cancel().await;
            Ok(tools)
        }
        "sse" => {
            validate_mcp_url(&server.command)?;
            let transport = SseClientTransport::start_with_client(
                mcp_http_client(&server.headers)?,
                SseClientConfig {
                    sse_endpoint: server.command.clone().into(),
                    ..Default::default()
                },
            )
            .await
            .map_err(|error| format!("MCP SSE 连接失败: {}", error))?;
            let client = ()
                .serve(transport)
                .await
                .map_err(|error| format!("MCP SSE 初始化失败: {}", error))?;
            let tools = client
                .list_all_tools()
                .await
                .map_err(|error| format!("读取 MCP 工具失败: {}", error))?;
            let _ = client.cancel().await;
            Ok(tools)
        }
        transport => Err(format!("不支持的 MCP 传输类型: {}", transport)),
    }
}

fn mcp_stdio_command(
    dirs: &AgentDockDirs,
    server: &McpServerRecord,
) -> Result<tokio::process::Command, String> {
    let mut paths = mcp_command_paths();
    let executable = resolve_mcp_executable(dirs, &paths, &server.command);
    if let Some(parent) = executable.parent() {
        paths.insert(0, parent.to_path_buf());
    }
    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path.clone()));
    let mut command = tokio::process::Command::new(executable);
    command.args(&server.args).envs(&server.env);
    if !server.cwd.trim().is_empty() {
        command.current_dir(&server.cwd);
    }
    let joined =
        env::join_paths(paths).map_err(|error| format!("生成 MCP PATH 失败: {}", error))?;
    if !server.env.contains_key("PATH") {
        command.env("PATH", joined);
    }

    let invokes_npx = server.command.to_ascii_lowercase().contains("npx")
        || server
            .args
            .iter()
            .any(|arg| arg.eq_ignore_ascii_case("npx"));
    if invokes_npx {
        if !server.env.contains_key("npm_config_registry") {
            command.env("npm_config_registry", "https://registry.npmmirror.com");
        }
        command
            .env("npm_config_update_notifier", "false")
            .env("npm_config_yes", "true");
    }
    if server.command.to_ascii_lowercase().contains("uvx")
        && !server.env.contains_key("UV_DEFAULT_INDEX")
    {
        command.env(
            "UV_DEFAULT_INDEX",
            "https://pypi.tuna.tsinghua.edu.cn/simple",
        );
    }
    Ok(command)
}

fn command_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs_home() {
        paths.extend([
            home.join(".local/bin"),
            home.join(".cargo/bin"),
            home.join(".npm-global/bin"),
            home.join(".volta/bin"),
            home.join(".bun/bin"),
            home.join(".asdf/shims"),
            home.join(".local/share/mise/shims"),
            home.join(".local/share/pnpm"),
            home.join("Library/pnpm"),
            home.join("Library/Application Support/pnpm"),
        ]);
        append_version_manager_bins(&mut paths, &home.join(".nvm/versions/node"), "bin");
        append_version_manager_bins(
            &mut paths,
            &home.join(".local/share/fnm/node-versions"),
            "installation/bin",
        );
        #[cfg(windows)]
        paths.push(home.join("AppData/Roaming/npm"));
    }
    #[cfg(target_os = "macos")]
    paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    #[cfg(all(unix, not(target_os = "macos")))]
    paths.extend([
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);
    if let Some(current) = env::var_os("PATH") {
        paths.extend(env::split_paths(&current));
    }
    if let Some(grok_bin) = dirs_home().map(|home| home.join(".grok/bin")) {
        paths.retain(|path| path != &grok_bin);
    }
    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path.clone()));
    paths
}

fn append_version_manager_bins(paths: &mut Vec<PathBuf>, root: &Path, suffix: &str) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    entries.reverse();
    paths.extend(entries.into_iter().map(|path| path.join(suffix)));
}

fn mcp_command_paths() -> Vec<PathBuf> {
    command_search_paths()
}

fn resolve_mcp_executable(dirs: &AgentDockDirs, paths: &[PathBuf], command: &str) -> PathBuf {
    let configured = PathBuf::from(command);
    if configured.components().count() > 1 {
        return configured;
    }
    for path in paths {
        for candidate in executable_candidates(command) {
            let executable = path.join(candidate);
            if executable.is_file() {
                return executable;
            }
        }
    }
    for root in [
        dirs.runtime_dir.join("hermes/bin"),
        dirs.clients_dir.join("openclaw/runtime"),
    ] {
        for candidate in executable_candidates(command) {
            if let Some(executable) = find_file_named(&root, &candidate.to_string_lossy()) {
                return executable;
            }
        }
    }
    configured
}

fn mcp_http_client(headers: &BTreeMap<String, String>) -> Result<reqwest::Client, String> {
    let mut defaults = reqwest::header::HeaderMap::new();
    for (name, value) in headers {
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| format!("MCP 请求头名称无效: {}", name))?;
        let value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|_| format!("MCP 请求头内容无效: {}", name))?;
        defaults.insert(name, value);
    }
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(8))
        .default_headers(defaults)
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("创建 MCP HTTP 客户端失败: {}", error))
}

fn validate_mcp_url(value: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(value).map_err(|_| "MCP URL 格式无效".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("MCP URL 必须使用 HTTP 或 HTTPS".to_string());
    }
    Ok(())
}

fn mcp_tool_info(tool: Tool) -> McpToolInfo {
    McpToolInfo {
        name: tool.name.into_owned(),
        title: tool.title,
        description: tool
            .description
            .map(|description| description.into_owned())
            .unwrap_or_default(),
        input_schema: serde_json::Value::Object((*tool.input_schema).clone()),
        output_schema: tool
            .output_schema
            .map(|schema| serde_json::Value::Object((*schema).clone())),
        annotations: tool
            .annotations
            .and_then(|annotations| serde_json::to_value(annotations).ok()),
    }
}

#[tauri::command]
fn delete_mcp_server(server_id: String) -> Result<OperationResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let mut servers = list_mcp_servers()?;
    let before = servers.len();
    servers.retain(|server| server.id != server_id);
    if before == servers.len() {
        return Err("未找到 MCP 服务器".to_string());
    }
    write_json(&mcp_servers_path(&dirs), &servers)?;
    sync_mcp_servers()?;
    Ok(OperationResult {
        ok: true,
        message: "MCP 服务器已删除".to_string(),
    })
}

#[tauri::command]
fn import_mcp_from_apps() -> Result<McpImportResult, String> {
    let dirs = agentdock_dirs()?;
    ensure_dirs(&dirs)?;
    let home = dirs_home().ok_or_else(|| "无法确定用户主目录".to_string())?;
    let mut servers = list_mcp_servers()?;
    let mut discovered = Vec::new();
    let mut scanned_apps = Vec::new();
    let mut errors = Vec::new();

    import_json_mcp_file(
        &home.join(".claude.json"),
        "mcpServers",
        "claude-code",
        "standard",
        &mut discovered,
        &mut scanned_apps,
        &mut errors,
    );
    if let Some(path) = claude_desktop_config_path() {
        import_json_mcp_file(
            &path,
            "mcpServers",
            "claude-desktop",
            "standard",
            &mut discovered,
            &mut scanned_apps,
            &mut errors,
        );
    }
    for path in [
        home.join(".agy/settings.json"),
        home.join(".gemini/settings.json"),
    ] {
        import_json_mcp_file(
            &path,
            "mcpServers",
            "antigravity",
            "standard",
            &mut discovered,
            &mut scanned_apps,
            &mut errors,
        );
    }
    import_json_mcp_file(
        &home.join(".config/opencode/opencode.json"),
        "mcp",
        "opencode",
        "opencode",
        &mut discovered,
        &mut scanned_apps,
        &mut errors,
    );
    import_json_mcp_file(
        &home.join(".openclaw/openclaw.json"),
        "mcpServers",
        "openclaw",
        "standard",
        &mut discovered,
        &mut scanned_apps,
        &mut errors,
    );
    import_toml_mcp_file(
        &home.join(".codex/config.toml"),
        "codex",
        &mut discovered,
        &mut scanned_apps,
        &mut errors,
    );
    import_toml_mcp_file(
        &home.join(".grok/config.toml"),
        "grok",
        &mut discovered,
        &mut scanned_apps,
        &mut errors,
    );
    import_hermes_mcp_file(
        &home.join(".hermes/config.yaml"),
        &mut discovered,
        &mut scanned_apps,
        &mut errors,
    );

    let mut imported = 0;
    let mut linked = 0;
    for (app, raw_id, value, style) in discovered {
        let Some(record) = mcp_record_from_value(&raw_id, &value, &app, &style) else {
            errors.push(format!("{}: MCP 服务器 {} 的格式无法识别", app, raw_id));
            continue;
        };
        if let Some(existing) = servers.iter_mut().find(|item| item.id == record.id) {
            if !existing.apps.contains(&app) {
                existing.apps.push(app);
                existing.updated_at = now_rfc3339();
                linked += 1;
            }
        } else {
            servers.push(record);
            imported += 1;
        }
    }
    scanned_apps.sort();
    scanned_apps.dedup();
    if imported > 0 || linked > 0 {
        write_json(&mcp_servers_path(&dirs), &servers)?;
    }

    Ok(McpImportResult {
        imported,
        linked,
        scanned_apps,
        errors,
    })
}

#[tauri::command]
fn sync_mcp_servers() -> Result<SyncResult, String> {
    let servers = list_mcp_servers()?;
    let dirs = agentdock_dirs()?;
    let home = dirs_home().ok_or_else(|| "无法确定用户主目录".to_string())?;
    let installed_apps = refresh_client_detection()
        .into_iter()
        .filter(|client| client.installed)
        .map(|client| client.id)
        .collect::<HashSet<_>>();
    let mut written_files = Vec::new();
    let mut errors = Vec::new();

    sync_json_mcp_projection(
        &home.join(".claude.json"),
        home.join(".claude").exists() || home.join(".claude.json").exists(),
        "mcpServers",
        "claude-code",
        "standard",
        &servers,
        &mut written_files,
        &mut errors,
    );
    if let Some(path) = claude_desktop_config_path() {
        let initialized = path.exists() || path.parent().map(Path::exists).unwrap_or(false);
        sync_json_mcp_projection(
            &path,
            initialized,
            "mcpServers",
            "claude-desktop",
            "standard",
            &servers,
            &mut written_files,
            &mut errors,
        );
    }
    sync_json_mcp_projection(
        &home.join(".agy/settings.json"),
        home.join(".agy").exists(),
        "mcpServers",
        "antigravity",
        "standard",
        &servers,
        &mut written_files,
        &mut errors,
    );
    sync_json_mcp_projection(
        &home.join(".config/opencode/opencode.json"),
        home.join(".config/opencode").exists(),
        "mcp",
        "opencode",
        "opencode",
        &servers,
        &mut written_files,
        &mut errors,
    );
    sync_json_mcp_projection(
        &home.join(".openclaw/openclaw.json"),
        home.join(".openclaw").exists(),
        "mcpServers",
        "openclaw",
        "standard",
        &servers,
        &mut written_files,
        &mut errors,
    );
    sync_toml_mcp_projection(
        &home.join(".codex/config.toml"),
        home.join(".codex").exists() || installed_apps.contains("codex"),
        "codex",
        &servers,
        &mut written_files,
        &mut errors,
    );
    sync_toml_mcp_projection(
        &home.join(".grok/config.toml"),
        home.join(".grok").exists() || installed_apps.contains("grok"),
        "grok",
        &servers,
        &mut written_files,
        &mut errors,
    );
    sync_toml_mcp_projection(
        &dirs.managed_configs_dir.join("grok/config.toml"),
        installed_apps.contains("grok"),
        "grok",
        &servers,
        &mut written_files,
        &mut errors,
    );
    sync_hermes_mcp_projection(
        &home.join(".hermes/config.yaml"),
        &servers,
        &mut written_files,
        &mut errors,
    );

    let message = if errors.is_empty() {
        format!("已同步 {} 个客户端配置", written_files.len())
    } else {
        format!(
            "已同步 {} 个客户端配置，{} 个配置写入失败：{}",
            written_files.len(),
            errors.len(),
            errors.join("；")
        )
    };
    Ok(SyncResult {
        written_files,
        message,
    })
}

fn import_json_mcp_file(
    path: &Path,
    key: &str,
    app: &str,
    style: &str,
    discovered: &mut Vec<(String, String, serde_json::Value, String)>,
    scanned_apps: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !path.exists() {
        return;
    }
    scanned_apps.push(app.to_string());
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            errors.push(format!("{}: 读取失败 ({})", app, error));
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("{}: 配置不是有效 JSON ({})", app, error));
            return;
        }
    };
    let Some(map) = value.get(key).and_then(serde_json::Value::as_object) else {
        return;
    };
    for (id, spec) in map {
        discovered.push((app.to_string(), id.clone(), spec.clone(), style.to_string()));
    }
}

fn import_toml_mcp_file(
    path: &Path,
    app: &str,
    discovered: &mut Vec<(String, String, serde_json::Value, String)>,
    scanned_apps: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !path.exists() {
        return;
    }
    scanned_apps.push(app.to_string());
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            errors.push(format!("{}: 读取失败 ({})", app, error));
            return;
        }
    };
    let root: toml::Table = match raw.parse() {
        Ok(root) => root,
        Err(error) => {
            errors.push(format!("{}: config.toml 格式错误 ({})", app, error));
            return;
        }
    };
    let table = root
        .get("mcp_servers")
        .and_then(toml::Value::as_table)
        .or_else(|| {
            root.get("mcp")
                .and_then(toml::Value::as_table)
                .and_then(|mcp| mcp.get("servers"))
                .and_then(toml::Value::as_table)
        });
    if let Some(table) = table {
        for (id, spec) in table {
            if let Ok(value) = serde_json::to_value(spec) {
                discovered.push((app.to_string(), id.clone(), value, "standard".to_string()));
            }
        }
    }
}

fn import_hermes_mcp_file(
    path: &Path,
    discovered: &mut Vec<(String, String, serde_json::Value, String)>,
    scanned_apps: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !path.exists() {
        return;
    }
    scanned_apps.push("hermes".to_string());
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            errors.push(format!("hermes: 读取失败 ({})", error));
            return;
        }
    };
    let yaml: serde_yaml::Value = match serde_yaml::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("hermes: config.yaml 格式错误 ({})", error));
            return;
        }
    };
    let value = serde_json::to_value(yaml).unwrap_or(serde_json::Value::Null);
    if let Some(map) = value
        .get("mcp_servers")
        .and_then(serde_json::Value::as_object)
    {
        for (id, spec) in map {
            discovered.push((
                "hermes".to_string(),
                id.clone(),
                spec.clone(),
                "standard".to_string(),
            ));
        }
    }
}

fn mcp_record_from_value(
    raw_id: &str,
    value: &serde_json::Value,
    app: &str,
    style: &str,
) -> Option<McpServerRecord> {
    let object = value.as_object()?;
    let raw_type = object
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("stdio");
    let transport = match raw_type {
        "local" | "stdio" => "stdio",
        "remote" | "sse" => "sse",
        "streamable-http" | "http" => "http",
        _ if object.get("url").is_some() => "http",
        _ => "stdio",
    }
    .to_string();
    let command_array = object.get("command").and_then(serde_json::Value::as_array);
    let command = if transport == "stdio" {
        object
            .get("command")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                command_array
                    .and_then(|items| items.first())
                    .and_then(serde_json::Value::as_str)
            })?
            .to_string()
    } else {
        object
            .get("url")
            .and_then(serde_json::Value::as_str)
            .or_else(|| object.get("command").and_then(serde_json::Value::as_str))?
            .to_string()
    };
    let args = if style == "opencode" {
        command_array
            .map(|items| {
                items
                    .iter()
                    .skip(1)
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    } else {
        object
            .get("args")
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    let env = json_string_map(
        object
            .get(if style == "opencode" {
                "environment"
            } else {
                "env"
            })
            .unwrap_or(&serde_json::Value::Null),
    );
    let headers = json_string_map(
        object
            .get("headers")
            .or_else(|| object.get("http_headers"))
            .unwrap_or(&serde_json::Value::Null),
    );
    let known_fields = [
        "type",
        "command",
        "args",
        "env",
        "environment",
        "url",
        "headers",
        "http_headers",
        "cwd",
        "enabled",
    ];
    let extra = object
        .iter()
        .filter(|(key, _)| !known_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let id = raw_id.trim().to_string();
    Some(McpServerRecord {
        id: id.clone(),
        name: raw_id.to_string(),
        description: String::new(),
        homepage: String::new(),
        docs: String::new(),
        tags: vec!["已导入".to_string()],
        transport,
        command,
        args,
        env,
        headers,
        cwd: object
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        extra,
        apps: vec![app.to_string()],
        enabled: true,
        updated_at: now_rfc3339(),
    })
}

fn json_string_map(value: &serde_json::Value) -> BTreeMap<String, String> {
    value
        .as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn sync_json_mcp_projection(
    path: &Path,
    initialized: bool,
    key: &str,
    app: &str,
    style: &str,
    servers: &[McpServerRecord],
    written_files: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !initialized {
        return;
    }
    let result = (|| -> Result<(), String> {
        let mut root: serde_json::Value = if path.exists() {
            let raw = fs::read_to_string(path)
                .map_err(|error| format!("读取 {} 失败: {}", path.display(), error))?;
            serde_json::from_str(&raw)
                .map_err(|error| format!("{} 不是有效 JSON: {}", path.display(), error))?
        } else {
            serde_json::json!({})
        };
        let object = root
            .as_object_mut()
            .ok_or_else(|| format!("{} 的根配置必须是 JSON 对象", path.display()))?;
        let mut projection = serde_json::Map::new();
        for server in servers
            .iter()
            .filter(|server| server.enabled && server.apps.iter().any(|item| item == app))
        {
            projection.insert(server.id.clone(), mcp_json_projection(server, style));
        }
        object.insert(key.to_string(), serde_json::Value::Object(projection));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建 {} 失败: {}", parent.display(), error))?;
        }
        write_json(path, &root)
    })();
    match result {
        Ok(()) => written_files.push(path.display().to_string()),
        Err(error) => errors.push(format!("{}: {}", app, error)),
    }
}

fn mcp_json_projection(server: &McpServerRecord, style: &str) -> serde_json::Value {
    let mut value: serde_json::Map<String, serde_json::Value> = server
        .extra
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if style == "opencode" {
        if server.transport == "stdio" {
            let command = std::iter::once(server.command.clone())
                .chain(server.args.clone())
                .collect::<Vec<_>>();
            value.insert("type".to_string(), serde_json::json!("local"));
            value.insert("command".to_string(), serde_json::json!(command));
            value.insert("environment".to_string(), serde_json::json!(server.env));
            value.insert("enabled".to_string(), serde_json::json!(true));
        } else {
            value.insert("type".to_string(), serde_json::json!("remote"));
            value.insert("url".to_string(), serde_json::json!(server.command));
            value.insert("headers".to_string(), serde_json::json!(server.headers));
            value.insert("enabled".to_string(), serde_json::json!(true));
        }
    } else if server.transport == "stdio" {
        value.insert("command".to_string(), serde_json::json!(server.command));
        value.insert("args".to_string(), serde_json::json!(server.args));
        value.insert("env".to_string(), serde_json::json!(server.env));
        if !server.cwd.is_empty() {
            value.insert("cwd".to_string(), serde_json::json!(server.cwd));
        }
    } else {
        value.insert("type".to_string(), serde_json::json!(server.transport));
        value.insert("url".to_string(), serde_json::json!(server.command));
        value.insert("headers".to_string(), serde_json::json!(server.headers));
    }
    serde_json::Value::Object(value)
}

fn sync_toml_mcp_projection(
    path: &Path,
    initialized: bool,
    app: &str,
    servers: &[McpServerRecord],
    written_files: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !initialized {
        return;
    }
    let result = (|| -> Result<(), String> {
        let mut root: toml::Table = if path.exists() {
            fs::read_to_string(path)
                .map_err(|error| format!("读取 config.toml 失败: {}", error))?
                .parse()
                .map_err(|error| format!("config.toml 格式错误: {}", error))?
        } else {
            toml::Table::new()
        };
        let mut projection = toml::Table::new();
        for server in servers
            .iter()
            .filter(|server| server.enabled && server.apps.iter().any(|item| item == app))
        {
            projection.insert(server.id.clone(), mcp_toml_projection(server));
        }
        root.remove("mcp_servers");
        if !projection.is_empty() {
            root.insert("mcp_servers".to_string(), toml::Value::Table(projection));
        }
        let raw = toml::to_string_pretty(&root)
            .map_err(|error| format!("生成 config.toml 失败: {}", error))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建 {} 失败: {}", parent.display(), error))?;
        }
        fs::write(path, raw).map_err(|error| format!("写入 config.toml 失败: {}", error))
    })();
    match result {
        Ok(()) => written_files.push(path.display().to_string()),
        Err(error) => errors.push(format!("{}: {}", app, error)),
    }
}

fn mcp_toml_projection(server: &McpServerRecord) -> toml::Value {
    let mut table: toml::Table = server
        .extra
        .iter()
        .filter_map(|(key, value)| json_value_to_toml(value).map(|value| (key.clone(), value)))
        .collect();
    if server.transport == "stdio" {
        table.insert(
            "command".to_string(),
            toml::Value::String(server.command.clone()),
        );
        if !server.args.is_empty() {
            table.insert(
                "args".to_string(),
                toml::Value::Array(
                    server
                        .args
                        .iter()
                        .cloned()
                        .map(toml::Value::String)
                        .collect(),
                ),
            );
        }
        if !server.env.is_empty() {
            table.insert("env".to_string(), string_map_to_toml(&server.env));
        }
        if !server.cwd.is_empty() {
            table.insert("cwd".to_string(), toml::Value::String(server.cwd.clone()));
        }
    } else {
        table.insert(
            "url".to_string(),
            toml::Value::String(server.command.clone()),
        );
        if !server.headers.is_empty() {
            table.insert(
                "http_headers".to_string(),
                string_map_to_toml(&server.headers),
            );
        }
    }
    toml::Value::Table(table)
}

fn json_value_to_toml(value: &serde_json::Value) -> Option<toml::Value> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Bool(value) => Some(toml::Value::Boolean(*value)),
        serde_json::Value::Number(value) => value
            .as_i64()
            .map(toml::Value::Integer)
            .or_else(|| value.as_f64().map(toml::Value::Float)),
        serde_json::Value::String(value) => Some(toml::Value::String(value.clone())),
        serde_json::Value::Array(values) => Some(toml::Value::Array(
            values.iter().filter_map(json_value_to_toml).collect(),
        )),
        serde_json::Value::Object(values) => Some(toml::Value::Table(
            values
                .iter()
                .filter_map(|(key, value)| {
                    json_value_to_toml(value).map(|value| (key.clone(), value))
                })
                .collect(),
        )),
    }
}

fn string_map_to_toml(map: &BTreeMap<String, String>) -> toml::Value {
    toml::Value::Table(
        map.iter()
            .map(|(key, value)| (key.clone(), toml::Value::String(value.clone())))
            .collect(),
    )
}

fn sync_hermes_mcp_projection(
    path: &Path,
    servers: &[McpServerRecord],
    written_files: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !path.exists() && !path.parent().map(Path::exists).unwrap_or(false) {
        return;
    }
    let result = (|| -> Result<(), String> {
        let mut root: serde_json::Value = if path.exists() {
            let raw = fs::read_to_string(path)
                .map_err(|error| format!("读取 config.yaml 失败: {}", error))?;
            let yaml: serde_yaml::Value = serde_yaml::from_str(&raw)
                .map_err(|error| format!("config.yaml 格式错误: {}", error))?;
            serde_json::to_value(yaml)
                .map_err(|error| format!("转换 config.yaml 失败: {}", error))?
        } else {
            serde_json::json!({})
        };
        let object = root
            .as_object_mut()
            .ok_or_else(|| "config.yaml 根配置必须是对象".to_string())?;
        let mut projection = serde_json::Map::new();
        for server in servers
            .iter()
            .filter(|server| server.enabled && server.apps.iter().any(|item| item == "hermes"))
        {
            let mut value = mcp_json_projection(server, "standard");
            value["enabled"] = serde_json::Value::Bool(true);
            projection.insert(server.id.clone(), value);
        }
        object.insert(
            "mcp_servers".to_string(),
            serde_json::Value::Object(projection),
        );
        let raw = serde_yaml::to_string(&root)
            .map_err(|error| format!("生成 config.yaml 失败: {}", error))?;
        fs::write(path, raw).map_err(|error| format!("写入 config.yaml 失败: {}", error))
    })();
    match result {
        Ok(()) => written_files.push(path.display().to_string()),
        Err(error) => errors.push(format!("hermes: {}", error)),
    }
}

#[tauri::command]
fn get_usage_stats(days: Option<u32>) -> Result<UsageStats, String> {
    let days = days.unwrap_or(7).clamp(1, 90);
    let now = OffsetDateTime::now_utc();
    let today = now.date();
    let first_date = today - Duration::days((days - 1) as i64);
    let from_timestamp = first_date.midnight().assume_utc();
    let providers = list_providers().unwrap_or_default();
    let mut records = Vec::new();
    let mut sources = Vec::new();
    let mut errors = Vec::new();

    if let Some(home) = dirs_home() {
        scan_claude_usage(
            &home.join(".claude/projects"),
            from_timestamp,
            &providers,
            &mut records,
            &mut sources,
            &mut errors,
        );
        scan_codex_usage(
            &home.join(".codex/sessions"),
            from_timestamp,
            &providers,
            &mut records,
            &mut sources,
            &mut errors,
        );
        scan_opencode_usage(
            &home.join(".local/share/opencode/opencode.db"),
            from_timestamp,
            &providers,
            &mut records,
            &mut sources,
            &mut errors,
        );
        scan_grok_usage(
            &home.join(".grok/sessions"),
            from_timestamp,
            &providers,
            &mut records,
            &mut sources,
            &mut errors,
        );
    }

    records.retain(|record| record.timestamp >= from_timestamp && record.timestamp <= now);
    let mut summary = UsageSummary {
        total_tokens: 0,
        input_tokens: 0,
        output_tokens: 0,
        cached_tokens: 0,
        requests: 0,
        cost_usd: 0.0,
        unpriced_requests: 0,
    };
    let mut trend_map: BTreeMap<String, UsageAggregate> = BTreeMap::new();
    let mut client_map: HashMap<String, UsageAggregate> = HashMap::new();
    let mut provider_map: HashMap<String, UsageAggregate> = HashMap::new();
    let mut model_map: HashMap<String, UsageAggregate> = HashMap::new();

    for offset in (0..days).rev() {
        let date = today - Duration::days(offset as i64);
        trend_map.insert(date.to_string(), UsageAggregate::default());
    }
    for record in &records {
        let total = record.input_tokens + record.output_tokens + record.cached_tokens;
        summary.total_tokens += total;
        summary.input_tokens += record.input_tokens;
        summary.output_tokens += record.output_tokens;
        summary.cached_tokens += record.cached_tokens;
        summary.requests += 1;
        if let Some(cost) = record.cost_usd {
            summary.cost_usd += cost;
        } else {
            summary.unpriced_requests += 1;
        }
        let date = record.timestamp.date().to_string();
        add_usage_aggregate(trend_map.entry(date).or_default(), total, record.cost_usd);
        add_usage_aggregate(
            client_map.entry(record.client.clone()).or_default(),
            total,
            record.cost_usd,
        );
        add_usage_aggregate(
            provider_map.entry(record.provider.clone()).or_default(),
            total,
            record.cost_usd,
        );
        add_usage_aggregate(
            model_map.entry(record.model.clone()).or_default(),
            total,
            record.cost_usd,
        );
    }
    summary.cost_usd = round_cost(summary.cost_usd);

    let trend = trend_map
        .into_iter()
        .map(|(date, value)| UsageTrendPoint {
            date,
            total_tokens: value.tokens,
            requests: value.requests,
            cost_usd: round_cost(value.cost),
        })
        .collect();
    let total_tokens = summary.total_tokens;

    Ok(UsageStats {
        days,
        from: first_date.to_string(),
        to: today.to_string(),
        summary,
        trend,
        by_client: usage_breakdown(client_map, total_tokens, true),
        by_provider: usage_breakdown(provider_map, total_tokens, false),
        by_model: usage_breakdown(model_map, total_tokens, false),
        sources,
        errors,
    })
}

#[derive(Debug, Default)]
struct UsageAggregate {
    tokens: u64,
    requests: u64,
    cost: f64,
}

fn add_usage_aggregate(aggregate: &mut UsageAggregate, tokens: u64, cost: Option<f64>) {
    aggregate.tokens += tokens;
    aggregate.requests += 1;
    aggregate.cost += cost.unwrap_or(0.0);
}

fn usage_breakdown(
    values: HashMap<String, UsageAggregate>,
    total_tokens: u64,
    use_client_names: bool,
) -> Vec<UsageBreakdownItem> {
    let mut result = values
        .into_iter()
        .map(|(id, value)| UsageBreakdownItem {
            name: if use_client_names {
                usage_client_name(&id).to_string()
            } else {
                id.clone()
            },
            id,
            total_tokens: value.tokens,
            requests: value.requests,
            cost_usd: round_cost(value.cost),
            share: if total_tokens == 0 {
                0.0
            } else {
                value.tokens as f64 / total_tokens as f64
            },
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| right.total_tokens.cmp(&left.total_tokens));
    result
}

fn usage_client_name(id: &str) -> &str {
    match id {
        "claude-code" => "Claude Code",
        "codex" => "Codex",
        "opencode" => "OpenCode",
        "antigravity" => "Antigravity",
        "grok" => "Grok",
        other => other,
    }
}

fn scan_grok_usage(
    root: &Path,
    from: OffsetDateTime,
    providers: &[ProviderProfile],
    records: &mut Vec<UsageRecord>,
    sources: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !root.exists() {
        return;
    }
    sources.push("Grok 会话".to_string());
    let mut files = Vec::new();
    collect_files_named(root, "signals.json", &mut files);
    let provider = active_provider_name(providers, "grok");
    for path in files {
        let signals: serde_json::Value = match fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|raw| serde_json::from_str(&raw).map_err(|error| error.to_string()))
        {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("Grok: 无法读取 {} ({})", path.display(), error));
                continue;
            }
        };
        let Some(usage) = find_grok_usage(&signals) else {
            continue;
        };
        let input_tokens = json_u64_any(usage, &["inputTokens", "input_tokens"]);
        let output_tokens = json_u64_any(usage, &["outputTokens", "output_tokens"])
            + json_u64_any(usage, &["thoughtTokens", "thought_tokens"]);
        let cached_tokens = json_u64_any(
            usage,
            &[
                "cachedTokens",
                "cached_tokens",
                "cachedWriteTokens",
                "cached_write_tokens",
            ],
        );
        if input_tokens + output_tokens + cached_tokens == 0 {
            continue;
        }

        let summary_path = path.parent().map(|parent| parent.join("summary.json"));
        let summary = summary_path
            .as_ref()
            .and_then(|summary_path| fs::read_to_string(summary_path).ok())
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .unwrap_or(serde_json::Value::Null);
        let timestamp = [
            "updatedAt",
            "updated_at",
            "lastModified",
            "createdAt",
            "created_at",
        ]
        .iter()
        .find_map(|key| summary.get(*key).and_then(parse_json_timestamp))
        .unwrap_or_else(|| file_modified_time(&path).unwrap_or(OffsetDateTime::UNIX_EPOCH));
        if timestamp < from {
            continue;
        }
        let model = ["modelId", "model_id", "model"]
            .iter()
            .find_map(|key| summary.get(*key).and_then(serde_json::Value::as_str))
            .unwrap_or("未知模型")
            .to_string();
        records.push(UsageRecord {
            timestamp,
            client: "grok".to_string(),
            provider: provider.clone(),
            cost_usd: estimate_model_cost(&model, input_tokens, output_tokens, cached_tokens),
            model,
            input_tokens,
            output_tokens,
            cached_tokens,
        });
    }
}

fn find_grok_usage(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value {
        serde_json::Value::Object(values) => {
            if values.contains_key("inputTokens")
                || values.contains_key("input_tokens")
                || values.contains_key("totalTokens")
                || values.contains_key("total_tokens")
            {
                return Some(value);
            }
            values.values().find_map(find_grok_usage)
        }
        serde_json::Value::Array(values) => values.iter().rev().find_map(find_grok_usage),
        _ => None,
    }
}

fn json_u64_any(value: &serde_json::Value, keys: &[&str]) -> u64 {
    keys.iter().map(|key| json_u64(value, key)).sum()
}

fn scan_claude_usage(
    root: &Path,
    from: OffsetDateTime,
    providers: &[ProviderProfile],
    records: &mut Vec<UsageRecord>,
    sources: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !root.exists() {
        return;
    }
    sources.push("Claude Code 会话".to_string());
    let mut files = Vec::new();
    collect_files_with_extension(root, "jsonl", &mut files);
    let mut seen_messages = HashSet::new();
    let provider = active_provider_name(providers, "claude-code");

    for path in files {
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(error) => {
                errors.push(format!("Claude: 无法读取 {} ({})", path.display(), error));
                continue;
            }
        };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if value.get("type").and_then(serde_json::Value::as_str) != Some("assistant") {
                continue;
            }
            let timestamp = value
                .get("timestamp")
                .and_then(parse_json_timestamp)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
            if timestamp < from {
                continue;
            }
            let Some(message) = value.get("message") else {
                continue;
            };
            let message_id = message
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    format!("{}:{}", path.display(), timestamp.unix_timestamp_nanos())
                });
            if !seen_messages.insert(message_id) {
                continue;
            }
            let Some(usage) = message.get("usage") else {
                continue;
            };
            let model = message
                .get("model")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("未知模型")
                .to_string();
            let input_tokens =
                json_u64(usage, "input_tokens") + json_u64(usage, "cache_creation_input_tokens");
            let output_tokens = json_u64(usage, "output_tokens");
            let cached_tokens = json_u64(usage, "cache_read_input_tokens");
            if input_tokens + output_tokens + cached_tokens == 0 {
                continue;
            }
            records.push(UsageRecord {
                timestamp,
                client: "claude-code".to_string(),
                provider: provider.clone(),
                cost_usd: estimate_model_cost(&model, input_tokens, output_tokens, cached_tokens),
                model,
                input_tokens,
                output_tokens,
                cached_tokens,
            });
        }
    }
}

fn scan_codex_usage(
    root: &Path,
    from: OffsetDateTime,
    providers: &[ProviderProfile],
    records: &mut Vec<UsageRecord>,
    sources: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !root.exists() {
        return;
    }
    sources.push("Codex 会话".to_string());
    let mut files = Vec::new();
    collect_files_with_extension(root, "jsonl", &mut files);
    let provider = active_provider_name(providers, "codex");

    for path in files {
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(error) => {
                errors.push(format!("Codex: 无法读取 {} ({})", path.display(), error));
                continue;
            }
        };
        let mut current_model = "未知模型".to_string();
        let mut previous: Option<(u64, u64, u64)> = None;
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let event_type = value.get("type").and_then(serde_json::Value::as_str);
            if event_type == Some("turn_context") {
                if let Some(model) = value
                    .get("payload")
                    .and_then(|payload| payload.get("model"))
                    .and_then(serde_json::Value::as_str)
                {
                    current_model = model.to_string();
                }
                continue;
            }
            let payload = value.get("payload").unwrap_or(&serde_json::Value::Null);
            if event_type != Some("event_msg")
                || payload.get("type").and_then(serde_json::Value::as_str) != Some("token_count")
            {
                continue;
            }
            let Some(total) = payload
                .get("info")
                .and_then(|info| info.get("total_token_usage"))
            else {
                continue;
            };
            let current = (
                json_u64(total, "input_tokens"),
                json_u64(total, "output_tokens"),
                json_u64(total, "cached_input_tokens"),
            );
            let delta = previous
                .map(|old| {
                    (
                        current.0.saturating_sub(old.0),
                        current.1.saturating_sub(old.1),
                        current.2.saturating_sub(old.2),
                    )
                })
                .unwrap_or(current);
            previous = Some(current);
            if delta.0 + delta.1 + delta.2 == 0 {
                continue;
            }
            let timestamp = value
                .get("timestamp")
                .and_then(parse_json_timestamp)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);
            if timestamp < from {
                continue;
            }
            records.push(UsageRecord {
                timestamp,
                client: "codex".to_string(),
                provider: provider.clone(),
                cost_usd: estimate_model_cost(&current_model, delta.0, delta.1, delta.2),
                model: current_model.clone(),
                input_tokens: delta.0,
                output_tokens: delta.1,
                cached_tokens: delta.2,
            });
        }
    }
}

fn scan_opencode_usage(
    path: &Path,
    from: OffsetDateTime,
    providers: &[ProviderProfile],
    records: &mut Vec<UsageRecord>,
    sources: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    if !path.exists() {
        return;
    }
    sources.push("OpenCode 会话".to_string());
    let connection = match rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) {
        Ok(connection) => connection,
        Err(error) => {
            errors.push(format!("OpenCode: 无法读取数据库 ({})", error));
            return;
        }
    };
    let mut statement = match connection.prepare("SELECT id, data FROM message") {
        Ok(statement) => statement,
        Err(error) => {
            errors.push(format!("OpenCode: 无法查询消息 ({})", error));
            return;
        }
    };
    let rows = match statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(rows) => rows,
        Err(error) => {
            errors.push(format!("OpenCode: 查询消息失败 ({})", error));
            return;
        }
    };
    let fallback_provider = active_provider_name(providers, "opencode");
    for row in rows.flatten() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&row.1) else {
            continue;
        };
        if value.get("role").and_then(serde_json::Value::as_str) != Some("assistant") {
            continue;
        }
        let timestamp = value
            .get("time")
            .and_then(|time| time.get("created"))
            .and_then(parse_json_timestamp)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        if timestamp < from {
            continue;
        }
        let Some(tokens) = value.get("tokens") else {
            continue;
        };
        let input_tokens = json_u64(tokens, "input");
        let output_tokens = json_u64(tokens, "output") + json_u64(tokens, "reasoning");
        let cached_tokens = tokens
            .get("cache")
            .map(|cache| json_u64(cache, "read"))
            .unwrap_or(0);
        if input_tokens + output_tokens + cached_tokens == 0 {
            continue;
        }
        let model = value
            .get("modelID")
            .or_else(|| value.get("modelId"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("未知模型")
            .to_string();
        let provider = value
            .get("providerID")
            .or_else(|| value.get("providerId"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| fallback_provider.clone());
        let reported_cost = value.get("cost").and_then(serde_json::Value::as_f64);
        records.push(UsageRecord {
            timestamp,
            client: "opencode".to_string(),
            provider,
            cost_usd: reported_cost.or_else(|| {
                estimate_model_cost(&model, input_tokens, output_tokens, cached_tokens)
            }),
            model,
            input_tokens,
            output_tokens,
            cached_tokens,
        });
    }
}

fn collect_files_with_extension(root: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extension(&path, extension, files);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            files.push(path);
        }
    }
}

fn collect_files_named(root: &Path, file_name: &str, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_named(&path, file_name, files);
        } else if path.file_name().and_then(|name| name.to_str()) == Some(file_name) {
            files.push(path);
        }
    }
}

fn file_modified_time(path: &Path) -> Option<OffsetDateTime> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let seconds = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    OffsetDateTime::from_unix_timestamp(seconds as i64).ok()
}

fn parse_json_timestamp(value: &serde_json::Value) -> Option<OffsetDateTime> {
    if let Some(raw) = value.as_str() {
        return OffsetDateTime::parse(raw, &Rfc3339).ok();
    }
    let numeric = value
        .as_i64()
        .or_else(|| value.as_u64().map(|value| value as i64))?;
    let seconds = if numeric.abs() > 10_000_000_000 {
        numeric / 1000
    } else {
        numeric
    };
    OffsetDateTime::from_unix_timestamp(seconds).ok()
}

fn json_u64(value: &serde_json::Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().map(|value| value.max(0) as u64))
        })
        .unwrap_or(0)
}

fn active_provider_name(providers: &[ProviderProfile], app: &str) -> String {
    providers
        .iter()
        .find(|provider| provider.active_apps.iter().any(|item| item == app))
        .map(|provider| provider.name.clone())
        .unwrap_or_else(|| "未识别供应商".to_string())
}

fn estimate_model_cost(
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cached_tokens: u64,
) -> Option<f64> {
    let model = model.to_ascii_lowercase();
    let (input_rate, output_rate, cache_rate) = if model.contains("gpt-5.6-sol") {
        (5.0, 30.0, 0.5)
    } else if model.contains("glm-5.2") {
        (1.4, 4.4, 0.26)
    } else if model.contains("claude-opus") {
        (15.0, 75.0, 1.5)
    } else if model.contains("claude-sonnet") {
        (3.0, 15.0, 0.3)
    } else if model.contains("claude-haiku") {
        (1.0, 5.0, 0.1)
    } else {
        return None;
    };
    Some(
        (input_tokens as f64 * input_rate
            + output_tokens as f64 * output_rate
            + cached_tokens as f64 * cache_rate)
            / 1_000_000.0,
    )
}

fn round_cost(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub fn run() {
    let process_args = env::args_os().collect::<Vec<_>>();
    if let Some(request) = managed_cli_request(&process_args) {
        match request.and_then(|(client_id, args)| run_managed_cli_command(&client_id, &args)) {
            Ok(code) => std::process::exit(code),
            Err(error) => {
                eprintln!("AgentDock: {}", error);
                std::process::exit(1);
            }
        }
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Err(error) = repair_managed_cli_access() {
                eprintln!("AgentDock: 修复终端命令失败: {}", error);
            }
            TRAY_AVAILABLE.store(setup_tray(app).is_ok(), Ordering::Relaxed);
            let settings = agentdock_dirs()
                .and_then(|dirs| read_app_settings(&dirs))
                .unwrap_or_default();
            let system_startup = env::args().any(|arg| arg == "--agentdock-autostart");
            if !TRAY_AVAILABLE.load(Ordering::Relaxed)
                || should_show_main_window(&settings, system_startup)
            {
                show_main_window(app.handle());
            } else {
                set_dock_visibility(app.handle(), false);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let minimize = agentdock_dirs()
                    .and_then(|dirs| read_app_settings(&dirs))
                    .map(|settings| settings.minimize_to_tray_on_close)
                    .unwrap_or(false);
                if minimize && TRAY_AVAILABLE.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                    set_dock_visibility(window.app_handle(), false);
                } else {
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_desktop_status,
            check_app_update,
            install_app_update,
            get_app_settings,
            save_app_settings,
            list_software_catalog,
            parse_provider_config,
            detect_cc_switch_config,
            import_cc_switch_config,
            fetch_provider_models,
            run_ready_check,
            list_providers,
            get_provider_api_key,
            save_provider,
            activate_provider,
            delete_provider,
            preview_provider_config,
            install_client,
            list_managed_clients,
            uninstall_client,
            test_provider,
            apply_active_provider_configs,
            apply_provider_config,
            run_diagnostics,
            launch_client,
            open_path,
            open_external,
            list_skills,
            install_skill,
            toggle_skill_app,
            uninstall_skill,
            sync_skills,
            list_mcp_servers,
            upsert_mcp_server,
            toggle_mcp_app,
            toggle_mcp_server,
            list_mcp_tools,
            delete_mcp_server,
            import_mcp_from_apps,
            sync_mcp_servers,
            get_usage_stats
        ])
        .build(tauri::generate_context!())
        .expect("failed to build AgentDock");

    app.run(|app, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            show_main_window(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}

fn show_main_window(app: &tauri::AppHandle) {
    set_dock_visibility(app, true);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg(target_os = "macos")]
fn set_dock_visibility(app: &tauri::AppHandle, visible: bool) {
    let _ = app.set_dock_visibility(visible);
}

#[cfg(not(target_os = "macos"))]
fn set_dock_visibility(_app: &tauri::AppHandle, _visible: bool) {}

fn should_show_main_window(settings: &AppSettings, system_startup: bool) -> bool {
    !settings.silent_startup || !system_startup
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开 AgentDock", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 AgentDock", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut builder = TrayIconBuilder::with_id("agentdock")
        .tooltip("AgentDock")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

struct AgentDockDirs {
    data_dir: PathBuf,
    config_dir: PathBuf,
    runtime_dir: PathBuf,
    clients_dir: PathBuf,
    skills_dir: PathBuf,
    mcp_dir: PathBuf,
    backups_dir: PathBuf,
    managed_configs_dir: PathBuf,
}

fn agentdock_dirs() -> Result<AgentDockDirs, String> {
    let project_dirs = ProjectDirs::from("com", "AgentDock", "AgentDock")
        .ok_or_else(|| "无法解析当前系统的应用数据目录".to_string())?;
    let data_dir = project_dirs.data_dir().to_path_buf();
    let config_dir = project_dirs.config_dir().to_path_buf();
    let runtime_dir = data_dir.join("runtime");
    let clients_dir = data_dir.join("clients");
    let skills_dir = data_dir.join("skills");
    let mcp_dir = data_dir.join("mcp");
    let backups_dir = data_dir.join("backups");
    let managed_configs_dir = config_dir.join("managed-configs");
    Ok(AgentDockDirs {
        data_dir,
        config_dir,
        runtime_dir,
        clients_dir,
        skills_dir,
        mcp_dir,
        backups_dir,
        managed_configs_dir,
    })
}

fn ensure_dirs(dirs: &AgentDockDirs) -> Result<(), String> {
    fs::create_dir_all(&dirs.data_dir).map_err(|err| format!("创建数据目录失败: {}", err))?;
    fs::create_dir_all(&dirs.config_dir).map_err(|err| format!("创建配置目录失败: {}", err))?;
    fs::create_dir_all(&dirs.runtime_dir)
        .map_err(|err| format!("创建托管运行时目录失败: {}", err))?;
    fs::create_dir_all(&dirs.clients_dir).map_err(|err| format!("创建客户端目录失败: {}", err))?;
    fs::create_dir_all(&dirs.skills_dir).map_err(|err| format!("创建 Skills 目录失败: {}", err))?;
    fs::create_dir_all(&dirs.mcp_dir).map_err(|err| format!("创建 MCP 目录失败: {}", err))?;
    fs::create_dir_all(&dirs.backups_dir).map_err(|err| format!("创建备份目录失败: {}", err))?;
    fs::create_dir_all(&dirs.managed_configs_dir)
        .map_err(|err| format!("创建托管配置目录失败: {}", err))?;
    Ok(())
}

fn providers_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.config_dir.join("providers.json")
}

fn managed_clients_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.config_dir.join("managed-clients.json")
}

fn skills_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.config_dir.join("skills.json")
}

fn app_settings_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.config_dir.join("settings.json")
}

fn read_app_settings(dirs: &AgentDockDirs) -> Result<AppSettings, String> {
    let settings: AppSettings =
        read_json_or_seed(&app_settings_path(dirs), AppSettings::default())?;
    Ok(normalize_app_settings(settings))
}

fn normalize_app_settings(mut settings: AppSettings) -> AppSettings {
    const LANGUAGES: &[&str] = &["zh-CN", "zh-TW", "en-US", "ja-JP", "de-DE"];
    const THEMES: &[&str] = &["light", "dark", "system"];
    const STORAGE_LOCATIONS: &[&str] = &["agentdock", "unified"];
    const SYNC_METHODS: &[&str] = &["copy", "symlink"];

    if !LANGUAGES.contains(&settings.language.as_str()) {
        settings.language = "zh-CN".to_string();
    }
    if !THEMES.contains(&settings.theme.as_str()) {
        settings.theme = "system".to_string();
    }
    if !STORAGE_LOCATIONS.contains(&settings.skill_storage_location.as_str()) {
        settings.skill_storage_location = "agentdock".to_string();
    }
    if !SYNC_METHODS.contains(&settings.skill_sync_method.as_str()) {
        settings.skill_sync_method = "copy".to_string();
    }
    if !terminal_options().contains(&settings.preferred_terminal.as_str()) {
        settings.preferred_terminal = default_terminal().to_string();
    }
    if !settings.launch_on_startup {
        settings.silent_startup = false;
    }

    settings.current_working_directory =
        normalize_working_directory(&settings.current_working_directory).unwrap_or_default();
    let mut recent_directories = Vec::new();
    if !settings.current_working_directory.is_empty() {
        recent_directories.push(settings.current_working_directory.clone());
    }
    for directory in settings.recent_working_directories {
        if let Some(directory) = normalize_working_directory(&directory) {
            if !recent_directories.contains(&directory) {
                recent_directories.push(directory);
            }
        }
        if recent_directories.len() >= 8 {
            break;
        }
    }
    settings.recent_working_directories = recent_directories;

    let supported = supported_provider_apps();
    let supported_set = supported.into_iter().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    settings
        .client_order
        .retain(|client| supported_set.contains(client.as_str()) && seen.insert(client.clone()));
    for client in supported {
        if seen.insert(client.to_string()) {
            settings.client_order.push(client.to_string());
        }
    }

    let visible = settings
        .visible_clients
        .into_iter()
        .filter(|client| supported_set.contains(client.as_str()))
        .collect::<HashSet<_>>();
    settings.visible_clients = settings
        .client_order
        .iter()
        .filter(|client| visible.contains(*client))
        .cloned()
        .collect();
    if settings.visible_clients.is_empty() {
        if let Some(client) = settings.client_order.first() {
            settings.visible_clients.push(client.clone());
        }
    }
    settings
}

fn validate_working_directory(value: &str) -> Result<PathBuf, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("启动命令行客户端前，请先选择项目目录".to_string());
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err("项目目录必须使用绝对路径，请重新选择".to_string());
    }
    if !path.is_dir() {
        return Err("项目目录不存在，请重新选择".to_string());
    }
    fs::read_dir(&path).map_err(|_| "无法读取项目目录，请重新选择".to_string())?;
    fs::canonicalize(&path).map_err(|_| "无法读取项目目录，请重新选择".to_string())
}

fn normalize_working_directory(value: &str) -> Option<String> {
    validate_working_directory(value)
        .ok()
        .map(|path| path.display().to_string())
}

fn terminal_options() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        return &[
            "terminal",
            "iterm2",
            "alacritty",
            "kitty",
            "ghostty",
            "wezterm",
        ];
    }
    #[cfg(windows)]
    {
        return &["powershell", "cmd", "wt"];
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return &[
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "alacritty",
            "kitty",
            "ghostty",
        ];
    }
    #[allow(unreachable_code)]
    &["system"]
}

fn default_terminal() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        return "terminal";
    }
    #[cfg(windows)]
    {
        return "powershell";
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return "x-terminal-emulator";
    }
    #[allow(unreachable_code)]
    "system"
}

fn skills_storage_dir(dirs: &AgentDockDirs, settings: &AppSettings) -> Result<PathBuf, String> {
    if settings.skill_storage_location == "unified" {
        return dirs_home()
            .map(|home| home.join(".agents").join("skills"))
            .ok_or_else(|| "无法确定统一 Skills 目录".to_string());
    }
    Ok(dirs.skills_dir.clone())
}

fn active_skills_dir(dirs: &AgentDockDirs) -> Result<PathBuf, String> {
    let settings = read_app_settings(dirs)?;
    let path = skills_storage_dir(dirs, &settings)?;
    fs::create_dir_all(&path).map_err(|err| format!("创建 Skills 存储目录失败: {}", err))?;
    Ok(path)
}

fn migrate_skill_storage(
    dirs: &AgentDockDirs,
    previous: &AppSettings,
    next: &AppSettings,
) -> Result<(), String> {
    let source_root = skills_storage_dir(dirs, previous)?;
    let target_root = skills_storage_dir(dirs, next)?;
    if source_root == target_root || !source_root.exists() {
        return Ok(());
    }
    fs::create_dir_all(&target_root)
        .map_err(|err| format!("创建新的 Skills 存储目录失败: {}", err))?;
    let skills: Vec<SkillRecord> = read_json_or_seed(&skills_path(dirs), default_skills())?;
    migrate_installed_skill_dirs(&source_root, &target_root, &skills)
}

fn migrate_installed_skill_dirs(
    source_root: &Path,
    target_root: &Path,
    skills: &[SkillRecord],
) -> Result<(), String> {
    for skill in skills.iter().filter(|skill| skill.installed) {
        let source = source_root.join(&skill.id);
        if source.is_dir() {
            copy_dir_all(&source, &target_root.join(&skill.id))?;
        }
    }
    Ok(())
}

fn auto_launch_app_path() -> Result<PathBuf, String> {
    env::current_exe().map_err(|err| format!("无法获取应用路径: {}", err))
}

fn auto_launch_manager() -> Result<AutoLaunch, String> {
    let app_path = auto_launch_app_path()?;
    AutoLaunchBuilder::new()
        .set_app_name("AgentDock")
        .set_app_path(&app_path.to_string_lossy())
        .set_use_launch_agent(cfg!(target_os = "macos"))
        .set_args(&["--agentdock-autostart"])
        .build()
        .map_err(|err| format!("创建开机启动配置失败: {}", err))
}

fn set_auto_launch_enabled(enabled: bool) -> Result<(), String> {
    let manager = auto_launch_manager()?;
    if enabled {
        manager
            .enable()
            .map_err(|err| format!("启用开机启动失败: {}", err))
    } else {
        manager
            .disable()
            .map_err(|err| format!("关闭开机启动失败: {}", err))
    }
}

fn mcp_servers_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.config_dir.join("mcp-servers.json")
}

fn provider_secrets_path(dirs: &AgentDockDirs) -> PathBuf {
    dirs.config_dir.join("provider-secrets.json")
}

fn read_provider_secrets(dirs: &AgentDockDirs) -> Result<BTreeMap<String, String>, String> {
    read_json_or_seed(
        &provider_secrets_path(dirs),
        BTreeMap::<String, String>::new(),
    )
}

fn write_provider_secrets(
    dirs: &AgentDockDirs,
    secrets: &BTreeMap<String, String>,
) -> Result<(), String> {
    let path = provider_secrets_path(dirs);
    write_json(&path, secrets)?;
    protect_secret_file(&path)
}

fn write_providers(dirs: &AgentDockDirs, providers: &[ProviderProfile]) -> Result<(), String> {
    let path = providers_path(dirs);
    write_json(&path, &providers)?;
    protect_secret_file(&path)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| format!("写入配置失败: {}", err))
}

fn read_json_or_seed<T>(path: &Path, seed: T) -> Result<T, String>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    if !path.exists() {
        write_json(path, &seed)?;
        return Ok(seed);
    }
    let raw = fs::read_to_string(path).map_err(|err| format!("读取配置失败: {}", err))?;
    serde_json::from_str(&raw).map_err(|err| format!("解析配置失败: {}", err))
}

fn validate_provider_settings_config(app_id: &str, raw: &str) -> Result<String, String> {
    let settings: serde_json::Value =
        serde_json::from_str(raw).map_err(|err| format!("配置文件内容不是有效 JSON: {}", err))?;
    if !settings.is_object() {
        return Err("配置文件内容必须是 JSON 对象".to_string());
    }
    if app_id == "codex" {
        let config = settings
            .get("config")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Codex 配置必须包含 config.toml 内容".to_string())?;
        config
            .parse::<toml::Table>()
            .map_err(|err| format!("config.toml 格式错误: {}", err))?;
        if !settings
            .get("auth")
            .map(serde_json::Value::is_object)
            .unwrap_or(false)
        {
            return Err("Codex 配置必须包含 auth.json 对象".to_string());
        }
    } else if app_id == "grok" {
        let config = settings
            .get("config")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Grok 配置必须包含 config.toml 内容".to_string())?;
        config
            .parse::<toml::Table>()
            .map_err(|err| format!("config.toml 格式错误: {}", err))?;
    }
    serde_json::to_string_pretty(&settings).map_err(|err| err.to_string())
}

fn materialized_provider_settings(
    provider: &ProviderProfile,
    api_key: &str,
) -> Result<Option<serde_json::Value>, String> {
    if provider.settings_config.trim().is_empty() {
        return Ok(None);
    }
    let mut settings: serde_json::Value = serde_json::from_str(&provider.settings_config)
        .map_err(|err| format!("读取供应商配置内容失败: {}", err))?;
    replace_json_placeholder(&mut settings, api_key);
    Ok(Some(settings))
}

fn codex_auth_for_provider(
    custom_settings: Option<&serde_json::Value>,
    api_key: &str,
) -> serde_json::Value {
    let mut auth = custom_settings
        .and_then(|settings| settings.get("auth"))
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    auth.insert(
        "OPENAI_API_KEY".to_string(),
        serde_json::Value::String(api_key.to_string()),
    );
    serde_json::Value::Object(auth)
}

fn external_codex_home() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("CODEX_HOME").filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    dirs_home()
        .map(|home| home.join(".codex"))
        .ok_or_else(|| "无法确定 Codex 配置目录".to_string())
}

fn restore_file_snapshot(path: &Path, snapshot: Option<&[u8]>) {
    match snapshot {
        Some(content) => {
            let _ = fs::write(path, content);
        }
        None => {
            let _ = fs::remove_file(path);
        }
    }
}

fn write_codex_config_pair(
    config_path: &Path,
    auth_path: &Path,
    config_content: &str,
    auth: &serde_json::Value,
    backup_dir: &Path,
    backup_prefix: &str,
) -> Result<Vec<String>, String> {
    let config_content = preserve_toml_section(config_path, config_content, "mcp_servers")?;
    let auth_content = serde_json::to_vec_pretty(auth)
        .map_err(|err| format!("生成 Codex auth.json 失败: {}", err))?;
    let old_config = if config_path.exists() {
        Some(fs::read(config_path).map_err(|err| format!("读取现有 Codex 配置失败: {}", err))?)
    } else {
        None
    };
    let old_auth = if auth_path.exists() {
        Some(fs::read(auth_path).map_err(|err| format!("读取现有 Codex 密钥失败: {}", err))?)
    } else {
        None
    };

    fs::create_dir_all(
        config_path
            .parent()
            .ok_or_else(|| "Codex 配置路径无效".to_string())?,
    )
    .map_err(|err| format!("创建 Codex 配置目录失败: {}", err))?;
    if let Some(content) = old_config.as_deref() {
        fs::write(
            backup_dir.join(format!("{}__config.toml", backup_prefix)),
            content,
        )
        .map_err(|err| format!("备份 Codex config.toml 失败: {}", err))?;
    }
    if let Some(content) = old_auth.as_deref() {
        fs::write(
            backup_dir.join(format!("{}__auth.json", backup_prefix)),
            content,
        )
        .map_err(|err| format!("备份 Codex auth.json 失败: {}", err))?;
    }

    let write_result = (|| {
        fs::write(auth_path, &auth_content)
            .map_err(|err| format!("写入 Codex auth.json 失败: {}", err))?;
        protect_secret_file(auth_path)?;
        fs::write(config_path, config_content)
            .map_err(|err| format!("写入 Codex config.toml 失败: {}", err))?;
        protect_secret_file(config_path)?;
        Ok::<(), String>(())
    })();
    if let Err(error) = write_result {
        restore_file_snapshot(auth_path, old_auth.as_deref());
        restore_file_snapshot(config_path, old_config.as_deref());
        return Err(error);
    }

    Ok(vec![
        config_path.display().to_string(),
        auth_path.display().to_string(),
    ])
}

fn replace_json_placeholder(value: &mut serde_json::Value, api_key: &str) {
    match value {
        serde_json::Value::String(text) => {
            *text = text.replace("${AGENTDOCK_API_KEY}", api_key);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                replace_json_placeholder(item, api_key);
            }
        }
        serde_json::Value::Object(entries) => {
            for value in entries.values_mut() {
                replace_json_placeholder(value, api_key);
            }
        }
        _ => {}
    }
}

fn detect_clients() -> Vec<ClientStatus> {
    let managed = list_managed_clients().unwrap_or_default();
    vec![
        detect_client(
            "codex",
            "Codex",
            &["codex"],
            client_user_config_dir("codex"),
            &managed,
        ),
        detect_client(
            "claude-code",
            "Claude Code",
            &["claude"],
            client_user_config_dir("claude-code"),
            &managed,
        ),
        detect_client(
            "claude-desktop",
            "Claude Desktop",
            &["claude-desktop", "Claude Desktop"],
            client_user_config_dir("claude-desktop"),
            &managed,
        ),
        detect_client(
            "antigravity",
            "Antigravity CLI",
            &["agy"],
            client_user_config_dir("antigravity"),
            &managed,
        ),
        detect_client(
            "grok",
            "Grok",
            &["grok"],
            client_user_config_dir("grok"),
            &managed,
        ),
        detect_client(
            "opencode",
            "OpenCode",
            &["opencode"],
            client_user_config_dir("opencode"),
            &managed,
        ),
        detect_client(
            "openclaw",
            "OpenClaw",
            &["openclaw"],
            client_user_config_dir("openclaw"),
            &managed,
        ),
        detect_client(
            "hermes",
            "Hermes",
            &["hermes"],
            client_user_config_dir("hermes"),
            &managed,
        ),
    ]
}

fn client_detection_cache() -> &'static Mutex<Option<(Instant, Vec<ClientStatus>)>> {
    CLIENT_DETECTION_CACHE.get_or_init(|| Mutex::new(None))
}

fn refresh_client_detection() -> Vec<ClientStatus> {
    let clients = detect_clients();
    if let Ok(mut cache) = client_detection_cache().lock() {
        *cache = Some((Instant::now(), clients.clone()));
    }
    clients
}

fn cached_client_for_launch(client_id: &str) -> Option<ClientStatus> {
    client_detection_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.as_ref().map(|(_, clients)| clients.clone()))
        .and_then(|clients| clients.into_iter().find(|client| client.id == client_id))
        .filter(|client| {
            client
                .executable
                .as_deref()
                .is_some_and(|executable| Path::new(executable).exists())
        })
}

fn cached_client_detection() -> Vec<ClientStatus> {
    if let Ok(cache) = client_detection_cache().lock() {
        if let Some((detected_at, clients)) = cache.as_ref() {
            if detected_at.elapsed() < std::time::Duration::from_secs(30) {
                return clients.clone();
            }
        }
    }
    refresh_client_detection()
}

fn detect_client(
    id: &str,
    name: &str,
    executable_names: &[&str],
    user_config_dir: Option<PathBuf>,
    managed_clients: &[ManagedClientRecord],
) -> ClientStatus {
    let managed = managed_clients
        .iter()
        .find(|client| client.id == id && client.installed && managed_client_is_runnable(client));
    let (external_executable, external_version) = detect_external_client(id, executable_names);
    let executable = managed
        .map(|client| client.launcher_path.clone())
        .or(external_executable);
    let version = managed
        .map(|client| client.version.clone())
        .or(external_version);

    let config_path = if managed.is_some() {
        user_config_dir.and_then(|path| {
            fs::create_dir_all(&path).ok()?;
            Some(path.display().to_string())
        })
    } else {
        user_config_dir
            .filter(|path| path.is_dir())
            .map(|path| path.display().to_string())
    };

    ClientStatus {
        id: id.to_string(),
        name: name.to_string(),
        installed: executable.is_some() || managed.is_some(),
        version,
        executable,
        config_path,
        managed_by_agentdock: managed.is_some(),
    }
}

fn detect_external_client(id: &str, executable_names: &[&str]) -> (Option<String>, Option<String>) {
    let search_paths = client_command_search_paths(id);
    if let Some(path) = executable_names
        .iter()
        .find_map(|name| find_executable_in_paths(name, &search_paths))
    {
        let executable = path.display().to_string();
        let version = command_version(&executable)
            .ok()
            .filter(|value| !value.trim().is_empty());
        return (Some(executable), version);
    }

    #[cfg(target_os = "macos")]
    if let Some(bundle) = find_macos_client_app(id) {
        let version = macos_app_bundle_version(&bundle);
        return (Some(bundle.display().to_string()), version);
    }

    (None, None)
}

fn managed_client_is_runnable(client: &ManagedClientRecord) -> bool {
    let launcher = PathBuf::from(&client.launcher_path);
    if !launcher.is_file() {
        return false;
    }
    fs::read_to_string(&launcher)
        .map(|content| !content.contains("Native client payload will be launched"))
        .unwrap_or(true)
}

#[cfg(any(test, windows, all(unix, not(target_os = "macos"))))]
fn find_executable(name: &str) -> Option<PathBuf> {
    find_executable_in_paths(name, &command_search_paths())
}

fn client_command_search_paths(client_id: &str) -> Vec<PathBuf> {
    let mut paths = command_search_paths();
    if client_id == "grok" {
        if let Some(grok_bin) = dirs_home().map(|home| home.join(".grok/bin")) {
            paths.retain(|path| path != &grok_bin);
            paths.insert(0, grok_bin);
        }
    }
    paths
}

fn find_executable_in_paths(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    let candidates = executable_candidates(name);

    for dir in paths {
        for candidate in &candidates {
            let full_path = dir.join(candidate);
            if full_path.is_file() {
                return Some(full_path);
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn find_macos_client_app(id: &str) -> Option<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs_home() {
        roots.push(home.join("Applications"));
    }
    find_macos_client_app_in_roots(id, &roots)
}

#[cfg(target_os = "macos")]
fn find_macos_client_app_in_roots(id: &str, roots: &[PathBuf]) -> Option<PathBuf> {
    let app_names: &[&str] = match id {
        "codex" => &["Codex.app"],
        "claude-desktop" => &["Claude.app", "Claude Desktop.app"],
        "opencode" => &["OpenCode.app", "OpenCode Desktop.app"],
        "openclaw" => &["OpenClaw.app"],
        _ => &[],
    };
    roots
        .iter()
        .flat_map(|root| app_names.iter().map(move |name| root.join(name)))
        .find(|path| is_macos_app_bundle(path))
}

#[cfg(target_os = "macos")]
fn macos_app_bundle_version(bundle: &Path) -> Option<String> {
    let output = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn executable_candidates(name: &str) -> Vec<OsString> {
    if cfg!(windows) {
        let path_ext = env::var_os("PATHEXT")
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| ".EXE;.CMD;.BAT;.COM".to_string());
        let mut candidates = vec![OsString::from(name)];
        for ext in path_ext.split(';') {
            candidates.push(OsString::from(format!("{}{}", name, ext.to_lowercase())));
            candidates.push(OsString::from(format!("{}{}", name, ext.to_uppercase())));
        }
        candidates
    } else {
        vec![OsString::from(name)]
    }
}

fn command_version(executable: &str) -> Result<String, String> {
    #[cfg(windows)]
    let mut command = if executable.to_ascii_lowercase().ends_with(".cmd")
        || executable.to_ascii_lowercase().ends_with(".bat")
    {
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/C", executable, "--version"]);
        command
    } else {
        let mut command = Command::new(executable);
        command.arg("--version");
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new(executable);
        command.arg("--version");
        command
    };
    let mut search_paths = command_search_paths();
    if let Some(parent) = Path::new(executable).parent() {
        search_paths.insert(0, parent.to_path_buf());
    }
    if let Ok(path) = env::join_paths(search_paths) {
        command.env("PATH", path);
    }
    let output = command_output_with_timeout(&mut command, std::time::Duration::from_secs(2))?;

    if !output.status.success() {
        let detail = first_line(String::from_utf8_lossy(&output.stderr).trim());
        return Err(if detail.is_empty() {
            format!("版本命令执行失败: {}", output.status)
        } else {
            detail
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return Ok(first_line(&stdout));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Ok(first_line(&stderr))
}

fn command_output_with_timeout(
    command: &mut Command,
    timeout: std::time::Duration,
) -> Result<std::process::Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(_) => return child.wait_with_output().map_err(|error| error.to_string()),
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("读取客户端版本号超时".to_string());
            }
            None => std::thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
}

fn first_line(value: &str) -> String {
    value.lines().next().unwrap_or_default().trim().to_string()
}

fn client_user_config_dir(client_id: &str) -> Option<PathBuf> {
    if client_id == "claude-desktop" {
        return claude_desktop_config_path().and_then(|path| path.parent().map(Path::to_path_buf));
    }
    dirs_home().and_then(|home| client_user_config_dir_in(&home, client_id))
}

fn client_user_config_dir_in(home: &Path, client_id: &str) -> Option<PathBuf> {
    let relative = match client_id {
        "codex" => ".codex",
        "claude-code" => ".claude",
        "antigravity" => ".agy",
        "grok" => ".grok",
        "opencode" => ".config/opencode",
        "openclaw" => ".openclaw",
        "hermes" => ".hermes",
        _ => return None,
    };
    Some(home.join(relative))
}

fn dirs_home() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn claude_desktop_config_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs_home().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("Claude")
                .join("claude_desktop_config.json")
        })
    } else if cfg!(windows) {
        env::var_os("APPDATA").map(|appdata| {
            PathBuf::from(appdata)
                .join("Claude")
                .join("claude_desktop_config.json")
        })
    } else {
        dirs_home().map(|home| {
            home.join(".config")
                .join("Claude")
                .join("claude_desktop_config.json")
        })
    }
}

struct ClientSpec {
    id: &'static str,
    name: &'static str,
}

fn client_spec(client_id: &str) -> Result<ClientSpec, String> {
    match client_id {
        "codex" => Ok(ClientSpec {
            id: "codex",
            name: "Codex",
        }),
        "claude-code" | "claude" => Ok(ClientSpec {
            id: "claude-code",
            name: "Claude Code",
        }),
        "claude-desktop" => Ok(ClientSpec {
            id: "claude-desktop",
            name: "Claude Desktop",
        }),
        "antigravity" | "agy" => Ok(ClientSpec {
            id: "antigravity",
            name: "Antigravity CLI",
        }),
        "grok" => Ok(ClientSpec {
            id: "grok",
            name: "Grok",
        }),
        "opencode" => Ok(ClientSpec {
            id: "opencode",
            name: "OpenCode",
        }),
        "openclaw" => Ok(ClientSpec {
            id: "openclaw",
            name: "OpenClaw",
        }),
        "hermes" => Ok(ClientSpec {
            id: "hermes",
            name: "Hermes",
        }),
        other => Err(format!("暂不支持安装客户端: {}", other)),
    }
}

async fn download_client_release(
    client_id: &str,
    target_dir: &Path,
) -> Result<(PathBuf, String, String), String> {
    match client_id {
        "codex" => match download_codex_from_npm(target_dir).await {
            Ok(result) => Ok(result),
            Err(mirror_error) => {
                download_github_client("openai/codex", codex_asset_name()?, client_id, target_dir)
                    .await
                    .map_err(|official_error| {
                        format!(
                            "国内镜像与官方源均不可用。镜像: {}; GitHub: {}",
                            mirror_error, official_error
                        )
                    })
            }
        },
        "claude-code" => download_claude_code_from_npm(target_dir).await,
        "antigravity" => download_antigravity_cli(target_dir).await,
        "grok" => download_grok_from_npm(target_dir).await,
        "opencode" => download_opencode_from_npm(target_dir).await,
        "openclaw" => download_openclaw_client(target_dir).await,
        _ => Err(format!(
            "{} 当前只支持本机检测",
            client_spec(client_id)?.name
        )),
    }
}

fn codex_asset_name() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("codex-aarch64-apple-darwin.tar.gz"),
        ("macos", "x86_64") => Ok("codex-x86_64-apple-darwin.tar.gz"),
        ("windows", "aarch64") => Ok("codex-aarch64-pc-windows-msvc.exe.zip"),
        ("windows", "x86_64") => Ok("codex-x86_64-pc-windows-msvc.exe.zip"),
        ("linux", "aarch64") => Ok("codex-aarch64-unknown-linux-musl.tar.gz"),
        ("linux", "x86_64") => Ok("codex-x86_64-unknown-linux-musl.tar.gz"),
        (os, arch) => Err(format!("Codex 暂不支持当前平台: {} {}", os, arch)),
    }
}

async fn download_github_client(
    repository: &str,
    asset_name: &str,
    client_id: &str,
    target_dir: &Path,
) -> Result<(PathBuf, String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建下载请求失败: {}", err))?;
    let release_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        repository
    );
    let release: serde_json::Value = client
        .get(&release_url)
        .send()
        .await
        .map_err(|err| format!("查询最新版本失败: {}", err))?
        .error_for_status()
        .map_err(|err| format!("查询最新版本失败: {}", err))?
        .json()
        .await
        .map_err(|err| format!("解析最新版本失败: {}", err))?;
    let version = release
        .get("tag_name")
        .and_then(|value| value.as_str())
        .unwrap_or("latest")
        .to_string();
    let asset = release
        .get("assets")
        .and_then(|value| value.as_array())
        .and_then(|assets| {
            assets.iter().find(|asset| {
                asset.get("name").and_then(|value| value.as_str()) == Some(asset_name)
            })
        })
        .ok_or_else(|| format!("最新版本没有适用于本机的安装包: {}", asset_name))?;
    let url = asset
        .get("browser_download_url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "安装包缺少下载地址".to_string())?;
    let expected_digest = asset
        .get("digest")
        .and_then(|value| value.as_str())
        .and_then(|value| value.strip_prefix("sha256:"))
        .map(str::to_string);
    let bytes = download_bytes(&client, url).await?;
    if let Some(expected) = expected_digest {
        verify_sha256(&bytes, &expected)?;
    }
    extract_archive(&bytes, asset_name, target_dir)?;
    let executable = find_client_executable(target_dir, client_id)?;
    Ok((
        executable,
        version.clone(),
        format!("GitHub 官方版本 {}", version),
    ))
}

async fn download_codex_from_npm(target_dir: &Path) -> Result<(PathBuf, String, String), String> {
    download_npm_native_client(
        "@openai/codex",
        "@openai/codex",
        Some(codex_npm_platform_suffix()?),
        None,
        "codex",
        target_dir,
    )
    .await
}

async fn download_claude_code_from_npm(
    target_dir: &Path,
) -> Result<(PathBuf, String, String), String> {
    download_npm_native_client(
        "@anthropic-ai/claude-code",
        claude_platform_package()?,
        None,
        None,
        "claude-code",
        target_dir,
    )
    .await
}

#[derive(Debug, Deserialize)]
struct AntigravityManifest {
    version: String,
    url: String,
    sha512: String,
}

async fn download_antigravity_cli(target_dir: &Path) -> Result<(PathBuf, String, String), String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(12))
        .timeout(std::time::Duration::from_secs(600))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建 Antigravity 下载请求失败: {}", err))?;
    let manifest_url = antigravity_manifest_url()?;
    let manifest: AntigravityManifest = client
        .get(&manifest_url)
        .send()
        .await
        .map_err(|err| format!("查询 Agy 最新版本失败: {}", err))?
        .error_for_status()
        .map_err(|err| format!("查询 Agy 最新版本失败: {}", err))?
        .json()
        .await
        .map_err(|err| format!("解析 Agy 版本信息失败: {}", err))?;
    let bytes = download_bytes(&client, &manifest.url).await?;
    verify_sha512_hex(&bytes, &manifest.sha512)?;

    let agy_executable = if manifest.url.contains(".tar.gz") {
        extract_archive(&bytes, "agy.tar.gz", target_dir)?;
        let source = find_file_named(target_dir, "antigravity")
            .ok_or_else(|| "Agy 安装包中缺少 antigravity 启动文件".to_string())?;
        let target = target_dir.join("agy");
        fs::copy(source, &target).map_err(|err| format!("写入 Agy 启动文件失败: {}", err))?;
        target
    } else {
        let target = target_dir.join(if cfg!(windows) { "agy.exe" } else { "agy" });
        fs::write(&target, bytes).map_err(|err| format!("写入 Agy 启动文件失败: {}", err))?;
        target
    };
    make_executable(&agy_executable)?;

    let (node_path, _) = download_managed_node_runtime(&client, target_dir).await?;
    let (gemini_entry, gemini_source) = download_gemini_proxy_cli(&client, target_dir).await?;
    let launcher =
        write_antigravity_launcher(target_dir, &agy_executable, &node_path, &gemini_entry)?;
    let version = antigravity_bundle_version(&manifest.version);

    Ok((
        launcher,
        version,
        format!(
            "Google 官方 Agy {} 与 {} Gemini CLI {}，完整性校验通过",
            manifest.version, gemini_source, GEMINI_PROXY_CLI_VERSION
        ),
    ))
}

const GEMINI_PROXY_CLI_VERSION: &str = "0.40.0";

fn antigravity_bundle_version(agy_version: &str) -> String {
    format!("{}+gemini.{}", agy_version.trim(), GEMINI_PROXY_CLI_VERSION)
}

async fn download_gemini_proxy_cli(
    client: &reqwest::Client,
    target_dir: &Path,
) -> Result<(PathBuf, &'static str), String> {
    let package_dir = target_dir.join("gemini-cli");
    let registries = [
        ("https://registry.npmmirror.com", "npmmirror 国内镜像"),
        ("https://registry.npmjs.org", "npm 官方源"),
    ];
    let mut errors = Vec::new();

    for (registry, source_name) in registries {
        let result = async {
            let metadata_url =
                npm_metadata_url(registry, "@google/gemini-cli", GEMINI_PROXY_CLI_VERSION);
            let metadata: serde_json::Value = client
                .get(metadata_url)
                .send()
                .await
                .map_err(|err| format!("查询 Gemini CLI 版本失败: {}", err))?
                .error_for_status()
                .map_err(|err| format!("查询 Gemini CLI 版本失败: {}", err))?
                .json()
                .await
                .map_err(|err| format!("解析 Gemini CLI 版本失败: {}", err))?;
            let dist = metadata
                .get("dist")
                .ok_or_else(|| "Gemini CLI 安装包缺少 dist 信息".to_string())?;
            let url = dist
                .get("tarball")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "Gemini CLI 安装包缺少下载地址".to_string())?;
            let integrity = dist
                .get("integrity")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "Gemini CLI 安装包缺少完整性校验".to_string())?;
            let bytes = download_bytes(client, url).await?;
            verify_npm_integrity(&bytes, integrity)?;
            if package_dir.exists() {
                fs::remove_dir_all(&package_dir)
                    .map_err(|err| format!("清理 Gemini CLI 安装目录失败: {}", err))?;
            }
            fs::create_dir_all(&package_dir)
                .map_err(|err| format!("创建 Gemini CLI 安装目录失败: {}", err))?;
            extract_archive(&bytes, "gemini-cli.tgz", &package_dir)?;
            find_file_named(&package_dir, "gemini.js")
                .ok_or_else(|| "Gemini CLI 安装包中缺少 gemini.js".to_string())
        }
        .await;

        match result {
            Ok(entry) => return Ok((entry, source_name)),
            Err(error) => errors.push(format!("{}: {}", source_name, error)),
        }
    }

    Err(format!("Gemini CLI 安装失败: {}", errors.join("；")))
}

async fn download_opencode_from_npm(
    target_dir: &Path,
) -> Result<(PathBuf, String, String), String> {
    download_npm_native_client(
        "opencode-ai",
        opencode_platform_package()?,
        None,
        None,
        "opencode",
        target_dir,
    )
    .await
}

async fn download_grok_from_npm(target_dir: &Path) -> Result<(PathBuf, String, String), String> {
    const GROK_MINIMUM_VERSION: &str = "0.2.101";
    download_npm_native_client(
        "@xai-official/grok",
        grok_platform_package()?,
        None,
        Some(GROK_MINIMUM_VERSION),
        "grok",
        target_dir,
    )
    .await
}

const MANAGED_NODE_VERSION: &str = "24.15.0";

async fn download_managed_node_runtime(
    client: &reqwest::Client,
    target_dir: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    let (node_asset, node_sha256) = managed_node_asset()?;
    let node_urls = [
        format!(
            "https://npmmirror.com/mirrors/node/v{}/{}",
            MANAGED_NODE_VERSION, node_asset
        ),
        format!(
            "https://nodejs.org/dist/v{}/{}",
            MANAGED_NODE_VERSION, node_asset
        ),
    ];
    let mut node_errors = Vec::new();
    let mut node_bytes = None;
    for url in node_urls {
        match download_bytes(client, &url).await {
            Ok(bytes) => {
                verify_sha256(&bytes, node_sha256)?;
                node_bytes = Some(bytes);
                break;
            }
            Err(error) => node_errors.push(error),
        }
    }
    let node_bytes = node_bytes.ok_or_else(|| {
        format!(
            "国内镜像和 Node.js 官方源均不可用: {}",
            node_errors.join("；")
        )
    })?;
    let runtime_dir = target_dir.join("runtime");
    fs::create_dir_all(&runtime_dir).map_err(|err| format!("创建 Node 运行时目录失败: {}", err))?;
    extract_archive(&node_bytes, node_asset, &runtime_dir)?;
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    let node_path = find_file_named(&runtime_dir, node_name)
        .ok_or_else(|| "Node 运行时中缺少启动文件".to_string())?;
    make_executable(&node_path)?;
    let npm_cli = find_file_named(&runtime_dir, "npm-cli.js")
        .ok_or_else(|| "Node 运行时中缺少 npm".to_string())?;
    Ok((node_path, npm_cli))
}

async fn download_openclaw_client(target_dir: &Path) -> Result<(PathBuf, String, String), String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(12))
        .timeout(std::time::Duration::from_secs(600))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建 OpenClaw 下载请求失败: {}", err))?;
    let (node_path, npm_cli) = download_managed_node_runtime(&client, target_dir).await?;

    let version = fetch_npm_latest_version(&client, "openclaw")
        .await?
        .ok_or_else(|| "无法获取 OpenClaw 最新版本".to_string())?;
    let app_dir = target_dir.join("app");
    let cache_dir = target_dir.join("npm-cache");
    let registries = [
        ("https://registry.npmmirror.com", "npmmirror 国内镜像"),
        ("https://registry.npmjs.org", "npm 官方源"),
    ];
    let mut install_errors = Vec::new();
    let mut source_name = "";
    for (registry, source) in registries {
        if app_dir.exists() {
            fs::remove_dir_all(&app_dir)
                .map_err(|err| format!("清理 OpenClaw 安装目录失败: {}", err))?;
        }
        fs::create_dir_all(&app_dir)
            .map_err(|err| format!("创建 OpenClaw 安装目录失败: {}", err))?;
        let mut command = Command::new(&node_path);
        command
            .arg(&npm_cli)
            .args(["install", "--omit=dev", "--no-audit", "--no-fund"])
            .arg(format!("--prefix={}", app_dir.display()))
            .arg(format!("--registry={}", registry))
            .arg(format!("openclaw@{}", version))
            .env("npm_config_cache", &cache_dir)
            .env("npm_config_update_notifier", "false");
        prepend_command_path(&mut command, node_path.parent())?;
        let output = command
            .output()
            .map_err(|err| format!("启动 OpenClaw 安装器失败: {}", err))?;
        if output.status.success() {
            source_name = source;
            break;
        }
        install_errors.push(format!("{}: {}", source, command_failure_detail(&output)));
    }
    if source_name.is_empty() {
        return Err(format!(
            "OpenClaw 依赖安装失败: {}",
            install_errors.join("；")
        ));
    }

    let entry = find_file_named(&app_dir, "openclaw.mjs")
        .ok_or_else(|| "OpenClaw 安装后缺少 openclaw.mjs".to_string())?;
    let launcher = write_node_client_launcher(target_dir, "openclaw", &node_path, &entry)?;
    Ok((
        launcher,
        version.clone(),
        format!(
            "{}，OpenClaw {}，托管 Node {}，npm 完整性校验通过",
            source_name, version, MANAGED_NODE_VERSION
        ),
    ))
}

async fn install_hermes_client(dirs: &AgentDockDirs) -> Result<InstallClientResult, String> {
    let install_dir = dirs.clients_dir.join("hermes");
    let home_dir = install_dir.join("home");
    let runtime_dir = install_dir.join("runtime");
    let venv_dir = install_dir.join("venv");
    fs::create_dir_all(&home_dir).map_err(|err| format!("创建 Hermes HOME 失败: {}", err))?;
    fs::create_dir_all(&runtime_dir)
        .map_err(|err| format!("创建 Hermes 运行时目录失败: {}", err))?;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(12))
        .timeout(std::time::Duration::from_secs(600))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建 Hermes 下载请求失败: {}", err))?;
    let version = fetch_pypi_latest_version(&client, "hermes-agent")
        .await?
        .ok_or_else(|| "无法获取 Hermes 最新版本".to_string())?;

    let uv_installer_url = if cfg!(windows) {
        "https://astral.sh/uv/install.ps1"
    } else {
        "https://astral.sh/uv/install.sh"
    };
    let installer = download_bytes(&client, uv_installer_url).await?;
    let installer_path = runtime_dir.join(if cfg!(windows) {
        "install-uv.ps1"
    } else {
        "install-uv.sh"
    });
    fs::write(&installer_path, installer).map_err(|err| format!("写入 uv 安装器失败: {}", err))?;

    let uv_dir = runtime_dir.join("bin");
    fs::create_dir_all(&uv_dir).map_err(|err| format!("创建 uv 目录失败: {}", err))?;
    let uv_path = uv_dir.join(if cfg!(windows) { "uv.exe" } else { "uv" });
    if !uv_path.is_file() {
        #[cfg(windows)]
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&installer_path)
            .env("UV_UNMANAGED_INSTALL", &uv_dir)
            .env("UV_INSTALL_DIR", &uv_dir)
            .output()
            .map_err(|err| format!("启动 uv 安装器失败: {}", err))?;
        #[cfg(not(windows))]
        let output = Command::new("/bin/sh")
            .arg(&installer_path)
            .env("UV_UNMANAGED_INSTALL", &uv_dir)
            .output()
            .map_err(|err| format!("启动 uv 安装器失败: {}", err))?;
        if !output.status.success() {
            return Err(format!(
                "安装 Hermes 托管 uv 失败: {}",
                command_failure_detail(&output)
            ));
        }
    }
    if !uv_path.is_file() {
        return Err("uv 安装器完成后未找到 uv 启动文件".to_string());
    }
    make_executable(&uv_path)?;

    let python_install = Command::new(&uv_path)
        .args(["python", "install", "3.11"])
        .env("UV_CACHE_DIR", runtime_dir.join("uv-cache"))
        .env("UV_PYTHON_INSTALL_DIR", runtime_dir.join("python"))
        .output()
        .map_err(|err| format!("启动 Python 安装器失败: {}", err))?;
    if !python_install.status.success() {
        return Err(format!(
            "安装 Hermes 托管 Python 失败: {}",
            command_failure_detail(&python_install)
        ));
    }

    if venv_dir.exists() {
        fs::remove_dir_all(&venv_dir)
            .map_err(|err| format!("更新 Hermes 虚拟环境失败: {}", err))?;
    }
    let venv = Command::new(&uv_path)
        .arg("venv")
        .arg(&venv_dir)
        .args(["--python", "3.11"])
        .env("UV_CACHE_DIR", runtime_dir.join("uv-cache"))
        .env("UV_PYTHON_INSTALL_DIR", runtime_dir.join("python"))
        .output()
        .map_err(|err| format!("创建 Hermes 虚拟环境失败: {}", err))?;
    if !venv.status.success() {
        return Err(format!(
            "创建 Hermes 虚拟环境失败: {}",
            command_failure_detail(&venv)
        ));
    }

    let python_path = if cfg!(windows) {
        venv_dir.join("Scripts/python.exe")
    } else {
        venv_dir.join("bin/python")
    };
    let package_specs = [
        format!("hermes-agent[all]=={}", version),
        format!("hermes-agent=={}", version),
    ];
    let indexes = [
        (
            "https://mirrors.aliyun.com/pypi/simple/",
            "阿里云 PyPI 镜像",
        ),
        ("https://pypi.org/simple/", "PyPI 官方源"),
    ];
    let mut installed_source = None;
    let mut errors = Vec::new();
    for package_spec in package_specs {
        for (index, source) in indexes {
            let output = Command::new(&uv_path)
                .args(["pip", "install", "--python"])
                .arg(&python_path)
                .arg("--index-url")
                .arg(index)
                .arg(&package_spec)
                .env("UV_CACHE_DIR", runtime_dir.join("uv-cache"))
                .env("UV_PYTHON_INSTALL_DIR", runtime_dir.join("python"))
                .env("HERMES_HOME", &home_dir)
                .output()
                .map_err(|err| format!("启动 Hermes 包安装器失败: {}", err))?;
            if output.status.success() {
                installed_source = Some((source, package_spec.contains("[all]")));
                break;
            }
            errors.push(format!("{}: {}", source, command_failure_detail(&output)));
        }
        if installed_source.is_some() {
            break;
        }
    }
    let (source, full_features) =
        installed_source.ok_or_else(|| format!("Hermes 安装失败: {}", errors.join("；")))?;
    let launcher = if cfg!(windows) {
        venv_dir.join("Scripts/hermes.exe")
    } else {
        venv_dir.join("bin/hermes")
    };
    if !launcher.is_file() {
        return Err("Hermes 安装完成后缺少启动文件".to_string());
    }
    make_executable(&launcher)?;
    let config_dir = dirs.managed_configs_dir.join("hermes");
    fs::create_dir_all(&config_dir).map_err(|err| format!("创建 Hermes 配置目录失败: {}", err))?;

    let now = now_rfc3339();
    let record = ManagedClientRecord {
        id: "hermes".to_string(),
        name: "Hermes".to_string(),
        installed: true,
        version: command_version(&launcher.display().to_string())
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or(version.clone()),
        install_dir: install_dir.display().to_string(),
        launcher_path: launcher.display().to_string(),
        config_dir: config_dir.display().to_string(),
        installed_at: now.clone(),
        updated_at: now,
    };
    save_managed_client_record(record.clone())?;
    let commands = install_managed_cli_access(&record)?;
    Ok(InstallClientResult {
        client: record,
        message: format!(
            "Hermes {} 已通过 {} 安装，托管 Python 运行时已就绪{}。终端命令 {} 已就绪，请重新打开终端",
            version,
            source,
            if full_features {
                ""
            } else {
                "（核心功能）"
            },
            commands.join("、")
        ),
    })
}

async fn download_npm_native_client(
    root_package: &str,
    platform_package: &str,
    platform_version_suffix: Option<&str>,
    minimum_version: Option<&str>,
    client_id: &str,
    target_dir: &Path,
) -> Result<(PathBuf, String, String), String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(12))
        .timeout(std::time::Duration::from_secs(300))
        .user_agent(format!("AgentDock/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| format!("创建下载请求失败: {}", err))?;
    let registries = [
        ("https://registry.npmmirror.com", "npmmirror 国内镜像"),
        ("https://registry.npmjs.org", "npm 官方源"),
    ];
    let latest_version = fetch_npm_latest_version(&client, root_package)
        .await?
        .ok_or_else(|| "无法获取最新版本".to_string())?;
    let version = minimum_version
        .filter(|minimum| version_is_newer(minimum, &latest_version))
        .unwrap_or(&latest_version)
        .to_string();
    let platform_version = platform_version_suffix
        .map(|suffix| format!("{}-{}", version, suffix))
        .unwrap_or_else(|| version.clone());
    let mut errors = Vec::new();

    for (registry, source_name) in registries {
        let result = async {
            let metadata_url = npm_metadata_url(registry, platform_package, &platform_version);
            let metadata: serde_json::Value = client
                .get(&metadata_url)
                .send()
                .await
                .map_err(|err| format!("查询本机安装包失败: {}", err))?
                .error_for_status()
                .map_err(|err| format!("查询本机安装包失败: {}", err))?
                .json()
                .await
                .map_err(|err| format!("解析安装包失败: {}", err))?;
            let dist = metadata
                .get("dist")
                .ok_or_else(|| "安装包缺少校验信息".to_string())?;
            let url = dist
                .get("tarball")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "安装包缺少下载地址".to_string())?;
            let integrity = dist
                .get("integrity")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "安装包缺少 SHA-512 校验".to_string())?;
            let bytes = download_bytes(&client, url).await?;
            verify_npm_integrity(&bytes, integrity)?;
            extract_archive(&bytes, "client.tgz", target_dir)?;
            if client_id == "grok" {
                decompress_grok_binary(target_dir)?;
            }
            let executable = find_client_executable(target_dir, client_id)?;
            let detected_version = command_version(&executable.display().to_string())?;
            if version_is_newer(&version, &detected_version) {
                return Err(format!(
                    "安装包版本校验失败：期望 {}，实际 {}",
                    version, detected_version
                ));
            }
            Ok::<_, String>((executable, version.clone()))
        }
        .await;

        match result {
            Ok((executable, version)) => {
                return Ok((
                    executable,
                    version.clone(),
                    format!("{}，官方包版本 {}，SHA-512 校验通过", source_name, version),
                ));
            }
            Err(error) => errors.push(format!("{}: {}", source_name, error)),
        }
    }

    Err(errors.join("；"))
}

fn decompress_grok_binary(root: &Path) -> Result<PathBuf, String> {
    let compressed_name = if cfg!(windows) {
        "grok.exe.br"
    } else {
        "grok.br"
    };
    let compressed_path = find_file_named(root, compressed_name)
        .ok_or_else(|| "Grok 安装包中缺少平台二进制".to_string())?;
    let output_name = compressed_name
        .strip_suffix(".br")
        .ok_or_else(|| "Grok 安装包文件名无效".to_string())?;
    let output_path = compressed_path.with_file_name(output_name);
    let compressed =
        fs::read(&compressed_path).map_err(|err| format!("读取 Grok 安装包失败: {}", err))?;
    let mut decoder = brotli::Decompressor::new(Cursor::new(compressed), 4096);
    let mut executable = Vec::new();
    decoder
        .read_to_end(&mut executable)
        .map_err(|err| format!("解压 Grok 启动文件失败: {}", err))?;
    fs::write(&output_path, executable)
        .map_err(|err| format!("写入 Grok 启动文件失败: {}", err))?;
    make_executable(&output_path)?;
    Ok(output_path)
}

fn npm_metadata_url(registry: &str, package: &str, version: &str) -> String {
    format!("{}/{}/{}", registry, package.replace('/', "%2F"), version)
}

async fn load_software_catalog_seeds(client: &reqwest::Client) -> Vec<SoftwareCatalogSeed> {
    if let Some(url) = option_env!("AGENTDOCK_SOFTWARE_CATALOG_URL") {
        if let Ok(response) = client.get(url).send().await {
            if let Ok(response) = response.error_for_status() {
                if let Ok(payload) = response.json::<RemoteSoftwareCatalog>().await {
                    if !payload.items.is_empty() {
                        return payload.items;
                    }
                }
            }
        }
    }
    built_in_software_catalog()
}

fn built_in_software_catalog() -> Vec<SoftwareCatalogSeed> {
    vec![
        SoftwareCatalogSeed {
            id: "codex".to_string(),
            client_id: "codex".to_string(),
            name: "Codex".to_string(),
            description: "OpenAI 官方终端编程代理，适合代码生成、修改和自动化任务。".to_string(),
            publisher: "OpenAI".to_string(),
            website_url: "https://github.com/openai/codex".to_string(),
            category: "编程客户端".to_string(),
            recommended: true,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "claude-code".to_string(),
            client_id: "claude-code".to_string(),
            name: "Claude Code".to_string(),
            description: "Anthropic 官方终端编程代理，支持代码库分析和多文件任务。".to_string(),
            publisher: "Anthropic".to_string(),
            website_url: "https://www.anthropic.com/claude-code".to_string(),
            category: "编程客户端".to_string(),
            recommended: true,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "antigravity".to_string(),
            client_id: "antigravity".to_string(),
            name: "Antigravity CLI".to_string(),
            description: "Google 新一代终端代理，Agy 已接替原 Gemini CLI 工作流。".to_string(),
            publisher: "Google".to_string(),
            website_url: "https://antigravity.google/product/antigravity-cli".to_string(),
            category: "编程客户端".to_string(),
            recommended: true,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "opencode".to_string(),
            client_id: "opencode".to_string(),
            name: "OpenCode".to_string(),
            description: "开源终端编程代理，可连接多种模型供应商。".to_string(),
            publisher: "Anomaly".to_string(),
            website_url: "https://opencode.ai".to_string(),
            category: "编程客户端".to_string(),
            recommended: true,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "grok".to_string(),
            client_id: "grok".to_string(),
            name: "Grok".to_string(),
            description: "xAI 官方终端编程代理，支持规划、子代理、MCP 和并行任务。".to_string(),
            publisher: "xAI".to_string(),
            website_url: "https://x.ai/cli".to_string(),
            category: "编程客户端".to_string(),
            recommended: true,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "openclaw".to_string(),
            client_id: "openclaw".to_string(),
            name: "OpenClaw".to_string(),
            description: "可连接聊天渠道的本地个人 AI 助手和自动化网关。".to_string(),
            publisher: "OpenClaw".to_string(),
            website_url: "https://openclaw.ai".to_string(),
            category: "智能助手".to_string(),
            recommended: false,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "hermes".to_string(),
            client_id: "hermes".to_string(),
            name: "Hermes Agent".to_string(),
            description: "Nous Research 开源智能代理，包含记忆、技能和消息网关。".to_string(),
            publisher: "Nous Research".to_string(),
            website_url: "https://hermes-agent.nousresearch.com".to_string(),
            category: "智能助手".to_string(),
            recommended: false,
            install_supported: true,
        },
        SoftwareCatalogSeed {
            id: "claude-desktop".to_string(),
            client_id: "claude-desktop".to_string(),
            name: "Claude Desktop".to_string(),
            description: "Anthropic 桌面客户端，AgentDock 可检测并同步 MCP 配置。".to_string(),
            publisher: "Anthropic".to_string(),
            website_url: "https://claude.ai/download".to_string(),
            category: "桌面客户端".to_string(),
            recommended: false,
            install_supported: false,
        },
    ]
}

async fn latest_client_version(
    client: &reqwest::Client,
    client_id: &str,
) -> Result<Option<String>, String> {
    match client_id {
        "codex" => fetch_npm_latest_version(client, "@openai/codex").await,
        "claude-code" => fetch_npm_latest_version(client, "@anthropic-ai/claude-code").await,
        "antigravity" => {
            let manifest: AntigravityManifest = client
                .get(antigravity_manifest_url()?)
                .send()
                .await
                .map_err(|err| err.to_string())?
                .error_for_status()
                .map_err(|err| err.to_string())?
                .json()
                .await
                .map_err(|err| err.to_string())?;
            Ok(Some(antigravity_bundle_version(&manifest.version)))
        }
        "opencode" => fetch_npm_latest_version(client, "opencode-ai").await,
        "grok" => fetch_npm_latest_version(client, "@xai-official/grok").await,
        "openclaw" => fetch_npm_latest_version(client, "openclaw").await,
        "hermes" => fetch_pypi_latest_version(client, "hermes-agent").await,
        _ => Ok(None),
    }
}

async fn fetch_npm_latest_version(
    client: &reqwest::Client,
    package: &str,
) -> Result<Option<String>, String> {
    let mut errors = Vec::new();
    let mut versions = Vec::new();
    for registry in [
        "https://registry.npmmirror.com",
        "https://registry.npmjs.org",
    ] {
        match client
            .get(npm_metadata_url(registry, package, "latest"))
            .send()
            .await
        {
            Ok(response) => match response.error_for_status() {
                Ok(response) => match response.json::<serde_json::Value>().await {
                    Ok(payload) => {
                        if let Some(version) =
                            payload.get("version").and_then(|value| value.as_str())
                        {
                            versions.push(version.to_string());
                            continue;
                        }
                        errors.push(format!("{} 返回的版本信息无效", registry));
                    }
                    Err(error) => errors.push(error.to_string()),
                },
                Err(error) => errors.push(error.to_string()),
            },
            Err(error) => errors.push(error.to_string()),
        }
    }
    versions.sort_by_key(|version| version_numbers(version));
    if let Some(version) = versions.pop() {
        Ok(Some(version))
    } else {
        Err(errors.join("；"))
    }
}

async fn fetch_pypi_latest_version(
    client: &reqwest::Client,
    package: &str,
) -> Result<Option<String>, String> {
    let url = format!("https://pypi.org/pypi/{}/json", package);
    let payload: serde_json::Value = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?
        .json()
        .await
        .map_err(|err| err.to_string())?;
    Ok(payload
        .get("info")
        .and_then(|info| info.get("version"))
        .and_then(|version| version.as_str())
        .map(str::to_string))
}

fn fallback_models(app_id: &str) -> Vec<String> {
    let models: &[&str] = match app_id {
        "claude-code" | "claude-desktop" => {
            &["claude-sonnet-5", "claude-opus-4-8", "claude-haiku-4-5"]
        }
        "antigravity" => &["gemini-3.5-pro", "gemini-3.5-flash", "gemini-3.1-pro"],
        "grok" => &[
            "grok-4.5",
            "grok-build",
            "grok-code-fast-1",
            "grok-4.1-fast",
        ],
        "opencode" | "openclaw" | "hermes" => &[
            "gpt-5.6-sol",
            "claude-sonnet-5",
            "gemini-3.5-pro",
            "deepseek-chat",
        ],
        _ => &["gpt-5.6-sol", "gpt-5.5-codex", "gpt-5.5"],
    };
    models.iter().map(|model| (*model).to_string()).collect()
}

fn parse_model_ids(payload: &serde_json::Value, protocol: ProviderModelProtocol) -> Vec<String> {
    let entries = payload
        .get("data")
        .and_then(serde_json::Value::as_array)
        .or_else(|| payload.get("models").and_then(serde_json::Value::as_array))
        .or_else(|| payload.as_array());
    let mut models = Vec::new();
    for entry in entries.into_iter().flatten() {
        if protocol == ProviderModelProtocol::Gemini {
            if let Some(methods) = entry
                .get("supportedGenerationMethods")
                .or_else(|| entry.get("supported_generation_methods"))
                .and_then(serde_json::Value::as_array)
            {
                let supports_generation = methods.iter().any(|method| {
                    matches!(
                        method.as_str(),
                        Some("generateContent" | "streamGenerateContent")
                    )
                });
                if !supports_generation {
                    continue;
                }
            }
        }
        let model = entry.as_str().or_else(|| {
            entry
                .get("id")
                .and_then(serde_json::Value::as_str)
                .or_else(|| entry.get("name").and_then(serde_json::Value::as_str))
        });
        if let Some(model) = model {
            let model = model.trim_start_matches("models/").trim();
            if !model.is_empty() && !models.iter().any(|item| item == model) {
                models.push(model.to_string());
            }
        }
    }
    models
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest = version_numbers(latest);
    let current = version_numbers(current);
    !latest.is_empty() && !current.is_empty() && latest > current
}

fn version_numbers(value: &str) -> Vec<u64> {
    let mut groups = Vec::new();
    let mut current = String::new();
    for character in value.chars() {
        if character.is_ascii_digit() {
            current.push(character);
        } else if !current.is_empty() {
            groups.push(current.parse::<u64>().unwrap_or(0));
            current.clear();
        }
    }
    if !current.is_empty() {
        groups.push(current.parse::<u64>().unwrap_or(0));
    }
    while groups.last() == Some(&0) {
        groups.pop();
    }
    groups
}

fn migrate_app_ids(apps: &mut Vec<String>) -> bool {
    let mut migrated = false;
    for app in apps.iter_mut() {
        if app == "gemini" {
            *app = "antigravity".to_string();
            migrated = true;
        }
    }
    apps.sort();
    apps.dedup();
    migrated
}

fn existing_directory_for_path(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_dir() {
        path.to_path_buf()
    } else if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.parent()?.to_path_buf()
    };
    loop {
        if current.is_dir() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
}

fn antigravity_manifest_url() -> Result<String, String> {
    let platform = match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "darwin_arm64",
        ("macos", "x86_64") => "darwin_amd64",
        ("windows", "aarch64") => "windows_arm64",
        ("windows", "x86_64") => "windows_amd64",
        ("linux", "aarch64") => "linux_arm64",
        ("linux", "x86_64") => "linux_amd64",
        (os, arch) => return Err(format!("Antigravity 暂不支持当前平台: {} {}", os, arch)),
    };
    Ok(format!(
        "https://antigravity-cli-auto-updater-974169037036.us-central1.run.app/manifests/{}.json",
        platform
    ))
}

fn opencode_platform_package() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH, linux_uses_musl()) {
        ("macos", "aarch64", _) => Ok("opencode-darwin-arm64"),
        ("macos", "x86_64", _) => Ok("opencode-darwin-x64"),
        ("windows", "aarch64", _) => Ok("opencode-windows-arm64"),
        ("windows", "x86_64", _) => Ok("opencode-windows-x64"),
        ("linux", "aarch64", true) => Ok("opencode-linux-arm64-musl"),
        ("linux", "x86_64", true) => Ok("opencode-linux-x64-musl"),
        ("linux", "aarch64", false) => Ok("opencode-linux-arm64"),
        ("linux", "x86_64", false) => Ok("opencode-linux-x64"),
        (os, arch, _) => Err(format!("OpenCode 暂不支持当前平台: {} {}", os, arch)),
    }
}

fn linux_uses_musl() -> bool {
    if env::consts::OS != "linux" {
        return false;
    }
    Path::new("/lib/libc.musl-x86_64.so.1").exists()
        || Path::new("/lib/libc.musl-aarch64.so.1").exists()
        || Command::new("ldd")
            .arg("--version")
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .to_ascii_lowercase()
                    .contains("musl")
                    || String::from_utf8_lossy(&output.stderr)
                        .to_ascii_lowercase()
                        .contains("musl")
            })
            .unwrap_or(false)
}

fn managed_node_asset() -> Result<(&'static str, &'static str), String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok((
            "node-v24.15.0-darwin-arm64.tar.gz",
            "372331b969779ab5d15b949884fc6eaf88d5afe87bde8ba881d6400b9100ffc4",
        )),
        ("macos", "x86_64") => Ok((
            "node-v24.15.0-darwin-x64.tar.gz",
            "ffd5ee293467927f3ee731a553eb88fd1f48cf74eebc2d74a6babe4af228673b",
        )),
        ("linux", "aarch64") => Ok((
            "node-v24.15.0-linux-arm64.tar.gz",
            "73afc234d558c24919875f51c2d1ea002a2ada4ea6f83601a383869fefa64eed",
        )),
        ("linux", "x86_64") => Ok((
            "node-v24.15.0-linux-x64.tar.gz",
            "44836872d9aec49f1e6b52a9a922872db9a2b02d235a616a5681b6a85fec8d89",
        )),
        ("windows", "aarch64") => Ok((
            "node-v24.15.0-win-arm64.zip",
            "c9eb7402eda26e2ba7e44b6727fc85a8de56c5095b1f71ebd3062892211aa116",
        )),
        ("windows", "x86_64") => Ok((
            "node-v24.15.0-win-x64.zip",
            "cc5149eabd53779ce1e7bdc5401643622d0c7e6800ade18928a767e940bb0e62",
        )),
        (os, arch) => Err(format!("托管 Node 暂不支持当前平台: {} {}", os, arch)),
    }
}

fn prepend_command_path(command: &mut Command, path: Option<&Path>) -> Result<(), String> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut entries = vec![path.to_path_buf()];
    if let Some(current) = env::var_os("PATH") {
        entries.extend(env::split_paths(&current));
    }
    let joined = env::join_paths(entries).map_err(|err| format!("生成托管 PATH 失败: {}", err))?;
    command.env("PATH", joined);
    Ok(())
}

fn command_failure_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let lines = detail.lines().rev().take(8).collect::<Vec<_>>();
    if lines.is_empty() {
        format!("进程退出状态 {}", output.status)
    } else {
        lines.into_iter().rev().collect::<Vec<_>>().join(" | ")
    }
}

fn find_file_named(root: &Path, expected_name: &str) -> Option<PathBuf> {
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let entries = fs::read_dir(directory).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                directories.push(path);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case(expected_name))
                .unwrap_or(false)
            {
                return Some(path);
            }
        }
    }
    None
}

fn write_node_client_launcher(
    root: &Path,
    client_id: &str,
    node_path: &Path,
    entry_path: &Path,
) -> Result<PathBuf, String> {
    let node_relative = node_path
        .strip_prefix(root)
        .map_err(|_| "Node 启动路径不在托管目录内".to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    let entry_relative = entry_path
        .strip_prefix(root)
        .map_err(|_| "客户端入口不在托管目录内".to_string())?
        .to_string_lossy()
        .replace('\\', "/");

    #[cfg(windows)]
    let (launcher, content) = {
        let launcher = root.join(format!("{}.cmd", client_id));
        let node = node_relative.replace('/', "\\");
        let entry = entry_relative.replace('/', "\\");
        (
            launcher,
            format!("@echo off\r\n\"%~dp0{}\" \"%~dp0{}\" %*\r\n", node, entry),
        )
    };
    #[cfg(not(windows))]
    let (launcher, content) = {
        let launcher = root.join(client_id);
        (
            launcher,
            format!(
                "#!/bin/sh\nROOT=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\nexec \"$ROOT/{}\" \"$ROOT/{}\" \"$@\"\n",
                node_relative, entry_relative
            ),
        )
    };
    fs::write(&launcher, content).map_err(|err| format!("写入客户端启动器失败: {}", err))?;
    make_executable(&launcher)?;
    Ok(launcher)
}

fn write_antigravity_launcher(
    root: &Path,
    agy_path: &Path,
    node_path: &Path,
    gemini_entry_path: &Path,
) -> Result<PathBuf, String> {
    let relative = |path: &Path| {
        path.strip_prefix(root)
            .map_err(|_| "Antigravity 启动路径不在托管目录内".to_string())
            .map(|path| path.to_string_lossy().replace('\\', "/"))
    };
    let agy = relative(agy_path)?;
    let node = relative(node_path)?;
    let gemini = relative(gemini_entry_path)?;

    #[cfg(windows)]
    let (launcher, content) = {
        let launcher = root.join("antigravity.cmd");
        let agy = agy.replace('/', "\\");
        let node = node.replace('/', "\\");
        let gemini = gemini.replace('/', "\\");
        (
            launcher,
            format!(
                "@echo off\r\nif not defined GOOGLE_GEMINI_BASE_URL goto agy\r\n\"%~dp0{}\" \"%~dp0{}\" %*\r\nexit /b %errorlevel%\r\n:agy\r\n\"%~dp0{}\" %*\r\n",
                node, gemini, agy
            ),
        )
    };
    #[cfg(not(windows))]
    let (launcher, content) = {
        let launcher = root.join("antigravity");
        (
            launcher,
            format!(
                "#!/bin/sh\nROOT=$(CDPATH= cd -- \"$(dirname -- \"$0\")\" && pwd)\nif [ -n \"${{GOOGLE_GEMINI_BASE_URL:-}}\" ]; then\n  exec \"$ROOT/{}\" \"$ROOT/{}\" \"$@\"\nfi\nexec \"$ROOT/{}\" \"$@\"\n",
                node, gemini, agy
            ),
        )
    };
    fs::write(&launcher, content).map_err(|err| format!("写入 Antigravity 启动器失败: {}", err))?;
    make_executable(&launcher)?;
    Ok(launcher)
}

fn save_managed_client_record(record: ManagedClientRecord) -> Result<(), String> {
    let dirs = agentdock_dirs()?;
    let mut clients = list_managed_clients()?;
    clients.retain(|client| client.id != record.id);
    clients.push(record);
    write_json(&managed_clients_path(&dirs), &clients)
}

fn codex_npm_platform_suffix() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("macos", "x86_64") => Ok("darwin-x64"),
        ("windows", "aarch64") => Ok("win32-arm64"),
        ("windows", "x86_64") => Ok("win32-x64"),
        ("linux", "aarch64") => Ok("linux-arm64"),
        ("linux", "x86_64") => Ok("linux-x64"),
        (os, arch) => Err(format!("Codex 暂不支持当前平台: {} {}", os, arch)),
    }
}

fn claude_platform_package() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("@anthropic-ai/claude-code-darwin-arm64"),
        ("macos", "x86_64") => Ok("@anthropic-ai/claude-code-darwin-x64"),
        ("windows", "aarch64") => Ok("@anthropic-ai/claude-code-win32-arm64"),
        ("windows", "x86_64") => Ok("@anthropic-ai/claude-code-win32-x64"),
        ("linux", "aarch64") => Ok("@anthropic-ai/claude-code-linux-arm64"),
        ("linux", "x86_64") => Ok("@anthropic-ai/claude-code-linux-x64"),
        (os, arch) => Err(format!("Claude Code 暂不支持当前平台: {} {}", os, arch)),
    }
}

fn grok_platform_package() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("@xai-official/grok-darwin-arm64"),
        ("macos", "x86_64") => Ok("@xai-official/grok-darwin-x64"),
        ("windows", "aarch64") => Ok("@xai-official/grok-win32-arm64"),
        ("windows", "x86_64") => Ok("@xai-official/grok-win32-x64"),
        ("linux", "aarch64") => Ok("@xai-official/grok-linux-arm64"),
        ("linux", "x86_64") => Ok("@xai-official/grok-linux-x64"),
        (os, arch) => Err(format!("Grok 暂不支持当前平台: {} {}", os, arch)),
    }
}

async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("下载安装包失败: {}", err))?
        .error_for_status()
        .map_err(|err| format!("下载安装包失败: {}", err))?;
    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|err| format!("读取安装包失败: {}", err))
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err("安装包 SHA-256 校验失败，已停止安装".to_string())
    }
}

fn verify_sha512_hex(bytes: &[u8], expected: &str) -> Result<(), String> {
    let actual = format!("{:x}", Sha512::digest(bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err("安装包 SHA-512 校验失败，已停止安装".to_string())
    }
}

fn verify_npm_integrity(bytes: &[u8], integrity: &str) -> Result<(), String> {
    let expected = integrity
        .strip_prefix("sha512-")
        .ok_or_else(|| "不支持的 npm 完整性校验格式".to_string())?;
    let actual = BASE64_STANDARD.encode(Sha512::digest(bytes));
    if actual == expected {
        Ok(())
    } else {
        Err("安装包 SHA-512 校验失败，已停止安装".to_string())
    }
}

fn extract_archive(bytes: &[u8], archive_name: &str, target_dir: &Path) -> Result<(), String> {
    if archive_name.ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|err| format!("打开 ZIP 安装包失败: {}", err))?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|err| format!("读取 ZIP 安装包失败: {}", err))?;
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| "ZIP 安装包包含不安全路径".to_string())?;
            let output = target_dir.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&output).map_err(|err| format!("创建安装目录失败: {}", err))?;
                continue;
            }
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent).map_err(|err| format!("创建安装目录失败: {}", err))?;
            }
            let mut file =
                fs::File::create(&output).map_err(|err| format!("写入安装文件失败: {}", err))?;
            std::io::copy(&mut entry, &mut file)
                .map_err(|err| format!("解压安装文件失败: {}", err))?;
        }
        return Ok(());
    }

    if archive_name.ends_with(".tar.gz") || archive_name.ends_with(".tgz") {
        let decoder = GzDecoder::new(Cursor::new(bytes));
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(target_dir)
            .map_err(|err| format!("解压安装包失败: {}", err))?;
        return Ok(());
    }

    Err(format!("不支持的安装包格式: {}", archive_name))
}

fn find_client_executable(root: &Path, client_id: &str) -> Result<PathBuf, String> {
    let prefix = if client_id == "claude-code" {
        "claude"
    } else {
        client_id
    };
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory).map_err(|err| format!("检查安装文件失败: {}", err))?
        {
            let entry = entry.map_err(|err| format!("检查安装文件失败: {}", err))?;
            let path = entry.path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let valid_name = name == prefix
                || name == format!("{}.exe", prefix)
                || name.starts_with(&format!("{}-", prefix));
            let ignored = name.ends_with(".sigstore")
                || name.ends_with(".sha256")
                || name.ends_with(".txt")
                || name.ends_with(".json");
            if valid_name && !ignored {
                return Ok(path);
            }
        }
    }
    Err(format!("安装包中没有找到 {} 启动文件", client_id))
}

fn bundled_client_payload_dir(client_id: &str) -> Option<PathBuf> {
    let platform = env::consts::OS;
    let exe = env::current_exe().ok()?;
    let mut roots = Vec::new();
    if let Some(parent) = exe.parent() {
        roots.push(parent.join("resources"));
        roots.push(parent.join("../Resources"));
        roots.push(parent.join("../../Resources"));
        roots.push(parent.to_path_buf());
    }

    roots
        .into_iter()
        .map(|root| root.join("installers").join(platform).join(client_id))
        .find(|candidate| candidate.is_dir())
}

fn copy_dir_all(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|err| format!("创建目录失败: {}", err))?;
    for entry in fs::read_dir(from).map_err(|err| format!("读取 payload 目录失败: {}", err))?
    {
        let entry = entry.map_err(|err| format!("读取 payload 项失败: {}", err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("读取 payload 类型失败: {}", err))?;
        let target = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)
                .map_err(|err| format!("复制 payload 文件失败: {}", err))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|err| format!("读取启动器权限失败: {}", err))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|err| format!("设置启动器权限失败: {}", err))
}

#[cfg(unix)]
fn make_private_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|err| format!("读取启动器权限失败: {}", err))?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).map_err(|err| format!("设置启动器权限失败: {}", err))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn default_providers() -> Vec<ProviderProfile> {
    Vec::new()
}

fn supported_provider_apps() -> [&'static str; 8] {
    [
        "claude-code",
        "claude-desktop",
        "codex",
        "antigravity",
        "grok",
        "opencode",
        "openclaw",
        "hermes",
    ]
}

fn default_skills() -> Vec<SkillRecord> {
    let now = now_rfc3339();
    vec![
        SkillRecord {
            id: "review".to_string(),
            name: "review".to_string(),
            description: "读取 diff，输出风险、回归和测试建议".to_string(),
            source: "built-in".to_string(),
            installed: true,
            apps: vec!["codex".to_string(), "claude-code".to_string()],
            updated_at: now.clone(),
        },
        SkillRecord {
            id: "browse".to_string(),
            name: "browse".to_string(),
            description: "浏览器 QA、截图、交互和控制台检查".to_string(),
            source: "built-in".to_string(),
            installed: false,
            apps: vec!["codex".to_string()],
            updated_at: now.clone(),
        },
        SkillRecord {
            id: "design-review".to_string(),
            name: "design-review".to_string(),
            description: "设计 QA 后给出可执行修复".to_string(),
            source: "built-in".to_string(),
            installed: false,
            apps: vec!["codex".to_string()],
            updated_at: now,
        },
    ]
}

fn default_mcp_servers() -> Vec<McpServerRecord> {
    let mut filesystem_env = BTreeMap::new();
    filesystem_env.insert("NODE_ENV".to_string(), "production".to_string());
    vec![
        McpServerRecord {
            id: "filesystem".to_string(),
            name: "filesystem".to_string(),
            description: "允许 AI 客户端访问指定的本地目录".to_string(),
            homepage: "https://github.com/modelcontextprotocol/servers".to_string(),
            docs: String::new(),
            tags: vec!["文件".to_string(), "官方".to_string()],
            transport: "stdio".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                ".".to_string(),
            ],
            env: filesystem_env,
            headers: BTreeMap::new(),
            cwd: String::new(),
            extra: BTreeMap::new(),
            apps: vec!["codex".to_string(), "claude-code".to_string()],
            enabled: true,
            updated_at: now_rfc3339(),
        },
        McpServerRecord {
            id: "browser-tools".to_string(),
            name: "browser-tools".to_string(),
            description: "连接本机浏览器自动化服务".to_string(),
            homepage: String::new(),
            docs: String::new(),
            tags: vec!["浏览器".to_string()],
            transport: "http".to_string(),
            command: "http://127.0.0.1:9321".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            extra: BTreeMap::new(),
            apps: vec!["codex".to_string()],
            enabled: false,
            updated_at: now_rfc3339(),
        },
    ]
}

fn normalize_base_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

fn ensure_v1_url(url: &str) -> String {
    let normalized = normalize_base_url(url);
    let is_origin_only = reqwest::Url::parse(&normalized)
        .map(|parsed| parsed.path().is_empty() || parsed.path() == "/")
        .unwrap_or_else(|_| {
            normalized
                .split_once("://")
                .map(|(_, rest)| !rest.contains('/'))
                .unwrap_or_else(|| !normalized.contains('/'))
        });
    if !is_origin_only || normalized.ends_with("/v1") {
        normalized
    } else {
        format!("{}/v1", normalized)
    }
}

fn default_gemini_model() -> String {
    "gemini-3.5-pro".to_string()
}

fn default_codex_model() -> String {
    "gpt-5.6-sol".to_string()
}

fn is_local_url(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
        .map(|host| host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1")
        .unwrap_or(false)
}

#[cfg(unix)]
fn protect_secret_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|err| format!("读取密钥文件权限失败: {}", err))?
        .permissions();
    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions).map_err(|err| format!("设置密钥文件权限失败: {}", err))
}

#[cfg(not(unix))]
fn protect_secret_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn slugify(value: &str) -> String {
    let slug = value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if slug.is_empty() {
        format!("provider-{}", OffsetDateTime::now_utc().unix_timestamp())
    } else {
        slug
    }
}

fn unique_provider_id(base: &str, providers: &[ProviderProfile]) -> String {
    if !providers.iter().any(|provider| provider.id == base) {
        return base.to_string();
    }
    for suffix in 2..10_000 {
        let candidate = format!("{}-{}", base, suffix);
        if !providers.iter().any(|provider| provider.id == candidate) {
            return candidate;
        }
    }
    format!("{}-{}", base, OffsetDateTime::now_utc().unix_timestamp())
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider_test_profile(
        id: &str,
        enabled_apps: &[&str],
        active_apps: &[&str],
    ) -> ProviderProfile {
        ProviderProfile {
            id: id.to_string(),
            name: id.to_string(),
            notes: String::new(),
            website_url: String::new(),
            preset_id: String::new(),
            provider_type: "openai".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            api_format: "responses".to_string(),
            settings_config: String::new(),
            enabled_apps: enabled_apps.iter().map(|app| (*app).to_string()).collect(),
            codex_model: default_codex_model(),
            gemini_model: default_gemini_model(),
            claude_sonnet_model: "claude-sonnet-5".to_string(),
            claude_haiku_model: "claude-haiku-4-5".to_string(),
            claude_opus_model: "claude-opus-4-8".to_string(),
            active: !active_apps.is_empty(),
            active_apps: active_apps.iter().map(|app| (*app).to_string()).collect(),
            api_key_configured: true,
            activation_reviewed: true,
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        }
    }

    fn cc_switch_test_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "agentdock-cc-switch-{}-{}-{}",
            name,
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ))
    }

    #[test]
    fn reads_cc_switch_sqlite_providers_in_read_only_mode() {
        let path = cc_switch_test_path("database").with_extension("db");
        let connection = rusqlite::Connection::open(&path).expect("create cc-switch fixture");
        connection
            .execute_batch(
                "CREATE TABLE providers (
                    id TEXT NOT NULL, app_type TEXT NOT NULL, name TEXT NOT NULL,
                    settings_config TEXT NOT NULL, website_url TEXT, category TEXT,
                    created_at INTEGER, sort_index INTEGER, notes TEXT, meta TEXT NOT NULL,
                    is_current BOOLEAN NOT NULL DEFAULT 0,
                    PRIMARY KEY (id, app_type)
                );",
            )
            .expect("create providers table");
        connection
            .execute(
                "INSERT INTO providers
                 (id, app_type, name, settings_config, website_url, category, created_at, sort_index, notes, meta, is_current)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0, '', ?7, 1)",
                rusqlite::params![
                    "ark-plan",
                    "claude",
                    "火山 AgentPlan",
                    r#"{"env":{"ANTHROPIC_BASE_URL":"https://ark.cn-beijing.volces.com/api/coding","ANTHROPIC_AUTH_TOKEN":"fixture-key","ANTHROPIC_MODEL":"ark-code-latest"}}"#,
                    "https://www.volcengine.com/activity/codingplan",
                    "cn_official",
                    r#"{"apiFormat":"anthropic"}"#,
                ],
            )
            .expect("insert provider");
        drop(connection);

        let providers = load_cc_switch_database(&path).expect("read cc-switch database");
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].app_id, "claude-code");
        assert!(providers[0].is_current);
        assert_eq!(providers[0].name, "火山 AgentPlan");
        let connection = rusqlite::Connection::open(&path).expect("reopen fixture");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM providers", [], |row| row.get(0))
            .expect("count unchanged providers");
        assert_eq!(count, 1);
        drop(connection);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn prepares_cc_switch_provider_without_leaking_its_key() {
        let candidate = CcSwitchProviderCandidate {
            source_id: "ark-plan".to_string(),
            source_app: "claude".to_string(),
            app_id: "claude-code".to_string(),
            name: "火山 AgentPlan".to_string(),
            settings_config: serde_json::json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://ark.cn-beijing.volces.com/api/coding",
                    "ANTHROPIC_AUTH_TOKEN": "fixture-key",
                    "ANTHROPIC_MODEL": "ark-code-latest",
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "ark-code-latest",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL": "ark-code-latest",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL": "ark-code-latest"
                }
            }),
            website_url: "https://www.volcengine.com/activity/codingplan".to_string(),
            category: "cn_official".to_string(),
            notes: String::new(),
            meta: serde_json::json!({ "apiFormat": "anthropic" }),
            is_current: true,
        };

        let prepared = prepare_cc_switch_provider(&candidate).expect("prepare provider");
        assert_eq!(prepared.api_key.as_deref(), Some("fixture-key"));
        assert!(!prepared.provider.settings_config.contains("fixture-key"));
        assert!(prepared
            .provider
            .settings_config
            .contains("${AGENTDOCK_API_KEY}"));
        assert_eq!(prepared.provider.active_apps, vec!["claude-code"]);
        assert_eq!(prepared.provider.codex_model, "ark-code-latest");
        assert_eq!(prepared.provider.claude_sonnet_model, "ark-code-latest");
        assert_eq!(prepared.provider.claude_haiku_model, "ark-code-latest");
        assert_eq!(prepared.provider.claude_opus_model, "ark-code-latest");
        assert_eq!(prepared.provider.preset_id, "volcengine-agentplan");
        assert!(provider_uses_anthropic_messages(&prepared.provider));
        let mut legacy_profile = prepared.provider.clone();
        legacy_profile.codex_model = default_codex_model();
        assert_eq!(provider_anthropic_model(&legacy_profile), "ark-code-latest");

        let preview = preview_provider_config(prepared.provider).expect("preview AgentPlan");
        let settings: serde_json::Value =
            serde_json::from_str(&preview.claude_env_json).expect("valid Claude settings");
        assert_eq!(settings["env"]["ANTHROPIC_MODEL"], "ark-code-latest");
        assert_eq!(
            settings["env"]["ANTHROPIC_DEFAULT_SONNET_MODEL"],
            "ark-code-latest"
        );
        assert_eq!(
            settings["env"]["ANTHROPIC_DEFAULT_FABLE_MODEL"],
            "ark-code-latest"
        );
        assert_eq!(
            settings["env"]["ANTHROPIC_DEFAULT_FABLE_MODEL_NAME"],
            "ark-code-latest"
        );
    }

    #[test]
    fn reads_legacy_cc_switch_json_and_builds_stable_ids() {
        let path = cc_switch_test_path("legacy").with_extension("json");
        fs::write(
            &path,
            r#"{
              "version": 2,
              "apps": {
                "codex": {
                  "current": "relay",
                  "providers": {
                    "relay": {
                      "name": "Relay",
                      "settingsConfig": {
                        "auth": {"OPENAI_API_KEY": "fixture-key"},
                        "config": "model_provider = \"custom\"\nmodel = \"gpt-test\"\n[model_providers.custom]\nbase_url = \"https://relay.example/v1\"\nwire_api = \"responses\""
                      }
                    }
                  }
                }
              }
            }"#,
        )
        .expect("write legacy fixture");
        let providers = load_cc_switch_legacy_json(&path).expect("read legacy config");
        assert_eq!(providers.len(), 1);
        let first_id = cc_switch_provider_id(&providers[0]);
        let second_id = cc_switch_provider_id(&providers[0]);
        assert_eq!(first_id, second_id);
        assert!(first_id.starts_with("cc-switch-codex-relay-"));
        let prepared = prepare_cc_switch_provider(&providers[0]).expect("prepare legacy provider");
        assert_eq!(prepared.provider.codex_model, "gpt-test");
        assert_eq!(prepared.provider.api_format, "responses");
        assert!(prepared.provider.active);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn normalizes_general_settings_and_keeps_a_visible_client() {
        let mut settings = AppSettings::default();
        settings.language = "invalid".to_string();
        settings.theme = "neon".to_string();
        settings.launch_on_startup = false;
        settings.silent_startup = true;
        settings.preferred_terminal = "missing-terminal".to_string();
        settings.client_order = vec![
            "codex".to_string(),
            "codex".to_string(),
            "unknown".to_string(),
        ];
        settings.visible_clients = vec!["unknown".to_string()];
        settings.skill_storage_location = "elsewhere".to_string();
        settings.skill_sync_method = "mirror".to_string();

        let settings = normalize_app_settings(settings);
        assert_eq!(settings.language, "zh-CN");
        assert_eq!(settings.theme, "system");
        assert!(!settings.silent_startup);
        assert_eq!(settings.preferred_terminal, default_terminal());
        assert_eq!(settings.client_order.len(), supported_provider_apps().len());
        assert_eq!(
            settings.client_order.first().map(String::as_str),
            Some("codex")
        );
        assert_eq!(settings.visible_clients, vec!["codex"]);
        assert_eq!(settings.skill_storage_location, "agentdock");
        assert_eq!(settings.skill_sync_method, "copy");
    }

    #[test]
    fn normalizes_and_deduplicates_recent_working_directories() {
        let root = env::temp_dir().join(format!(
            "agentdock-working-directories-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let first = root.join("first");
        let second = root.join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();

        let mut settings = AppSettings::default();
        settings.current_working_directory = second.display().to_string();
        settings.recent_working_directories = vec![
            first.display().to_string(),
            second.display().to_string(),
            first.display().to_string(),
            root.join("missing").display().to_string(),
        ];
        let settings = normalize_app_settings(settings);
        let canonical_first = fs::canonicalize(&first).unwrap().display().to_string();
        let canonical_second = fs::canonicalize(&second).unwrap().display().to_string();

        assert_eq!(settings.current_working_directory, canonical_second);
        assert_eq!(
            settings.recent_working_directories,
            vec![canonical_second, canonical_first]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn silent_startup_only_hides_system_launches() {
        let mut settings = AppSettings::default();
        settings.launch_on_startup = true;
        settings.silent_startup = true;
        assert!(!should_show_main_window(&settings, true));
        assert!(should_show_main_window(&settings, false));
        settings.silent_startup = false;
        assert!(should_show_main_window(&settings, true));
    }

    #[test]
    fn migrates_only_installed_skill_directories() {
        let root = env::temp_dir().join(format!(
            "agentdock-skill-migration-test-{}",
            std::process::id()
        ));
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(source.join("installed")).unwrap();
        fs::create_dir_all(source.join("available")).unwrap();
        fs::write(source.join("installed/SKILL.md"), "installed").unwrap();
        fs::write(source.join("available/SKILL.md"), "available").unwrap();
        let skills = vec![
            SkillRecord {
                id: "installed".to_string(),
                name: "installed".to_string(),
                description: String::new(),
                source: "local".to_string(),
                installed: true,
                apps: vec!["codex".to_string()],
                updated_at: now_rfc3339(),
            },
            SkillRecord {
                id: "available".to_string(),
                name: "available".to_string(),
                description: String::new(),
                source: "local".to_string(),
                installed: false,
                apps: vec!["codex".to_string()],
                updated_at: now_rfc3339(),
            },
        ];

        migrate_installed_skill_dirs(&source, &target, &skills).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("installed/SKILL.md")).unwrap(),
            "installed"
        );
        assert!(!target.join("available").exists());
        assert!(source.join("installed/SKILL.md").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_iterm_script_reuses_the_default_window_only_when_needed() {
        let launcher = Path::new("/tmp/AgentDock user's launcher.command");
        let command = macos_launcher_command(launcher);
        let script = macos_iterm_script();

        assert!(command.starts_with("exec sh "));
        assert!(command.contains("AgentDock user"));
        assert!(command.contains("user'\"'\"'s"));
        assert!(command.contains("launcher.command"));
        assert!(script.contains("if had_windows then"));
        assert!(script.contains("set launched_window to create window with default profile"));
        assert!(script
            .contains("tell current session of launched_window to write text launcher_command"));
        assert!(script
            .contains("tell current session of current window to write text launcher_command"));
        assert!(!script.contains("close every window"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn terminal_launch_confirmation_observes_consumed_launchers() {
        let root = env::temp_dir().join(format!(
            "agentdock-launch-confirmation-test-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let consumed = root.join("consumed.command");
        let pending = root.join("pending.command");
        fs::write(&pending, "launcher").unwrap();

        assert!(wait_for_launcher_consumption(
            &consumed,
            std::time::Duration::from_millis(10)
        ));
        assert!(!wait_for_launcher_consumption(
            &pending,
            std::time::Duration::from_millis(10)
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_terminal_fallback_requires_an_explicit_launch_error() {
        assert!(!should_fallback_to_macos_terminal("iterm2", false));
        assert!(should_fallback_to_macos_terminal("iterm2", true));
        assert!(!should_fallback_to_macos_terminal("terminal", true));
    }

    #[test]
    fn codex_launch_home_requires_both_config_files() {
        let root = env::temp_dir().join(format!(
            "agentdock-codex-launch-home-test-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        assert!(!codex_home_is_ready(&root));

        fs::write(root.join("config.toml"), "model = \"test\"").unwrap();
        assert!(!codex_home_is_ready(&root));

        fs::write(root.join("auth.json"), "{}").unwrap();
        assert!(codex_home_is_ready(&root));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn syncs_skill_files_by_copy() {
        let root =
            env::temp_dir().join(format!("agentdock-skill-copy-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.md");
        let target = root.join("target/SKILL.md");
        fs::write(&source, "skill-content").unwrap();

        sync_skill_file(&source, &target, "copy").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "skill-content");
        assert!(!fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn syncs_skill_files_by_symlink() {
        let root = env::temp_dir().join(format!(
            "agentdock-skill-symlink-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.md");
        let target = root.join("target/SKILL.md");
        fs::write(&source, "skill-content").unwrap();

        sync_skill_file(&source, &target, "symlink").unwrap();
        assert!(fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&target).unwrap(), "skill-content");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn imports_grok_toml_provider_fields() {
        let parsed = parse_provider_config_text(
            "grok",
            r#"[models]
default = "grok"
web_search = "grok"

[model."grok"]
model = "grok-4.5"
base_url = "https://xxx.cn/v1"
name = "Grok 4.5"
api_key = "sk-xx"
api_backend = "responses"
context_window = 1000000
supports_backend_search = true"#,
        )
        .expect("Grok config should parse");
        assert_eq!(parsed.base_url.as_deref(), Some("https://xxx.cn/v1"));
        assert_eq!(parsed.api_key.as_deref(), Some("sk-xx"));
        assert_eq!(parsed.model.as_deref(), Some("grok-4.5"));
        assert_eq!(parsed.api_format.as_deref(), Some("responses"));
        assert_eq!(parsed.source_format, "TOML");
    }

    #[test]
    fn imports_codex_toml_and_auth_json() {
        let config = parse_provider_config_text(
            "codex",
            r#"model_provider = "OpenAI"
model = "gpt-5.5"
review_model = "gpt-5.5"
model_reasoning_effort = "xhigh"
disable_response_storage = true
network_access = "enabled"
windows_wsl_setup_acknowledged = true

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://code.xxxxx.cn"
wire_api = "responses"
requires_openai_auth = true

[features]
goals = true"#,
        )
        .expect("Codex config should parse");
        assert_eq!(config.base_url.as_deref(), Some("https://code.xxxxx.cn"));
        assert_eq!(config.model.as_deref(), Some("gpt-5.5"));
        assert_eq!(config.api_format.as_deref(), Some("responses"));

        let auth = parse_provider_config_text("codex", r#"{"OPENAI_API_KEY":"sk-codex"}"#)
            .expect("Codex auth should parse");
        assert_eq!(auth.api_key.as_deref(), Some("sk-codex"));
    }

    #[test]
    fn writes_codex_config_and_auth_as_a_pair() {
        let root = env::temp_dir().join(format!(
            "agentdock-codex-provider-test-{}",
            std::process::id()
        ));
        let codex_home = root.join("codex-home");
        let backup = root.join("backup");
        fs::create_dir_all(&codex_home).unwrap();
        fs::create_dir_all(&backup).unwrap();
        let config_path = codex_home.join("config.toml");
        let auth_path = codex_home.join("auth.json");
        fs::write(
            &config_path,
            "model = \"old\"\n\n[mcp_servers.memory]\ncommand = \"npx\"\n",
        )
        .unwrap();
        fs::write(&auth_path, r#"{"OPENAI_API_KEY":"old-key"}"#).unwrap();

        let config = r#"model_provider = "OpenAI"
model = "gpt-5.5"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://code.xxxxx.cn"
wire_api = "responses"
requires_openai_auth = true"#;
        let settings = serde_json::json!({
            "auth": { "OPENAI_API_KEY": "stale-pasted-key", "KEEP": "value" },
            "config": config
        });
        let auth = codex_auth_for_provider(Some(&settings), "sk-xxxxx");
        let written =
            write_codex_config_pair(&config_path, &auth_path, config, &auth, &backup, "test")
                .expect("Codex config pair should be written");

        assert_eq!(
            written,
            vec![
                config_path.display().to_string(),
                auth_path.display().to_string()
            ]
        );
        let saved_auth: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&auth_path).unwrap()).unwrap();
        assert_eq!(saved_auth["OPENAI_API_KEY"], "sk-xxxxx");
        assert_eq!(saved_auth["KEEP"], "value");
        let saved_config: toml::Table = fs::read_to_string(&config_path).unwrap().parse().unwrap();
        assert_eq!(saved_config["model_provider"].as_str(), Some("OpenAI"));
        assert_eq!(
            saved_config["model_providers"]["OpenAI"]["base_url"].as_str(),
            Some("https://code.xxxxx.cn")
        );
        assert_eq!(
            saved_config["mcp_servers"]["memory"]["command"].as_str(),
            Some("npx")
        );
        assert!(backup.join("test__config.toml").exists());
        assert!(backup.join("test__auth.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn grok_fallback_models_include_grok_4_5_first() {
        let models = fallback_models("grok");
        assert_eq!(models.first().map(String::as_str), Some("grok-4.5"));
    }

    #[test]
    fn imports_claude_and_antigravity_env_json() {
        let claude = parse_provider_config_text(
            "claude-code",
            r#"{"env":{"ANTHROPIC_BASE_URL":"https://claude.example.com","ANTHROPIC_AUTH_TOKEN":"sk-ant","ANTHROPIC_MODEL":"claude-sonnet-test"}}"#,
        )
        .expect("Claude config should parse");
        assert_eq!(
            claude.base_url.as_deref(),
            Some("https://claude.example.com")
        );
        assert_eq!(claude.api_key.as_deref(), Some("sk-ant"));
        assert_eq!(claude.model.as_deref(), Some("claude-sonnet-test"));
        assert_eq!(claude.api_format.as_deref(), Some("anthropic"));

        let antigravity = parse_provider_config_text(
            "antigravity",
            r#"{"env":{"GOOGLE_GEMINI_BASE_URL":"https://gemini.example.com","GEMINI_API_KEY":"gem-key","GEMINI_MODEL":"gemini-test"}}"#,
        )
        .expect("Antigravity config should parse");
        assert_eq!(
            antigravity.base_url.as_deref(),
            Some("https://gemini.example.com")
        );
        assert_eq!(antigravity.api_key.as_deref(), Some("gem-key"));
        assert_eq!(antigravity.model.as_deref(), Some("gemini-test"));
        assert_eq!(antigravity.api_format.as_deref(), Some("gemini"));
    }

    #[test]
    fn imports_opencode_provider_json() {
        let parsed = parse_provider_config_text(
            "opencode",
            r#"{
  "provider": {
    "relay": {
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "https://open.example.com/v1", "apiKey": "sk-open" },
      "models": { "deepseek-chat": { "name": "DeepSeek" } }
    }
  },
  "model": "relay/deepseek-chat"
}"#,
        )
        .expect("OpenCode config should parse");
        assert_eq!(
            parsed.base_url.as_deref(),
            Some("https://open.example.com/v1")
        );
        assert_eq!(parsed.api_key.as_deref(), Some("sk-open"));
        assert_eq!(parsed.model.as_deref(), Some("deepseek-chat"));
    }

    #[test]
    fn imports_nested_openclaw_provider_json() {
        let parsed = parse_provider_config_text(
            "openclaw",
            r#"{
  "models": {
    "providers": {
      "relay": {
        "baseUrl": "https://claw.example.com/v1",
        "apiKey": "sk-claw",
        "api": "openai-completions",
        "models": [{ "id": "glm-5", "name": "GLM" }]
      }
    }
  },
  "agents": { "defaults": { "model": { "primary": "relay/glm-5" } } }
}"#,
        )
        .expect("OpenClaw config should parse");
        assert_eq!(
            parsed.base_url.as_deref(),
            Some("https://claw.example.com/v1")
        );
        assert_eq!(parsed.api_key.as_deref(), Some("sk-claw"));
        assert_eq!(parsed.model.as_deref(), Some("glm-5"));
        assert_eq!(parsed.api_format.as_deref(), Some("chat-completions"));
    }

    #[test]
    fn imports_hermes_json_and_environment_text() {
        let hermes = parse_provider_config_text(
            "hermes",
            r#"{"name":"relay","base_url":"https://hermes.example.com/v1","api_key":"sk-hermes","model":"qwen-coder","api_mode":"anthropic_messages"}"#,
        )
        .expect("Hermes config should parse");
        assert_eq!(
            hermes.base_url.as_deref(),
            Some("https://hermes.example.com/v1")
        );
        assert_eq!(hermes.api_key.as_deref(), Some("sk-hermes"));
        assert_eq!(hermes.model.as_deref(), Some("qwen-coder"));
        assert_eq!(hermes.api_format.as_deref(), Some("anthropic"));

        let env = parse_provider_config_text(
            "claude-desktop",
            "export ANTHROPIC_BASE_URL=https://env.example.com\nexport ANTHROPIC_AUTH_TOKEN=sk-env",
        )
        .expect("environment config should parse");
        assert_eq!(env.base_url.as_deref(), Some("https://env.example.com"));
        assert_eq!(env.api_key.as_deref(), Some("sk-env"));
    }

    #[test]
    fn normalizes_provider_urls() {
        assert_eq!(
            normalize_base_url(" https://api.example.com/v1/ "),
            "https://api.example.com/v1"
        );
        assert_eq!(
            ensure_v1_url("https://api.example.com"),
            "https://api.example.com/v1"
        );
        assert_eq!(
            ensure_v1_url("https://api.example.com/v1/"),
            "https://api.example.com/v1"
        );
        assert_eq!(
            ensure_v1_url("https://open.bigmodel.cn/api/paas/v4"),
            "https://open.bigmodel.cn/api/paas/v4"
        );
    }

    #[test]
    fn recognizes_only_loopback_urls_as_local() {
        assert!(is_local_url("http://127.0.0.1:8080/v1"));
        assert!(is_local_url("https://localhost/v1"));
        assert!(!is_local_url("https://localhost.example.com/v1"));
        assert!(!is_local_url("https://api.example.com/v1"));
    }

    #[test]
    fn validates_release_digests() {
        let bytes = b"hello";
        assert!(verify_sha256(
            bytes,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        )
        .is_ok());
        assert!(verify_sha256(bytes, "invalid").is_err());

        let integrity = format!("sha512-{}", BASE64_STANDARD.encode(Sha512::digest(bytes)));
        assert!(verify_npm_integrity(bytes, &integrity).is_ok());
        assert!(verify_npm_integrity(bytes, "sha512-invalid").is_err());
    }

    fn app_release(tag: &str, draft: bool, assets: Vec<GithubAppAsset>) -> GithubAppRelease {
        GithubAppRelease {
            tag_name: tag.to_string(),
            draft,
            assets,
        }
    }

    fn app_asset(name: &str, digest: Option<&str>) -> GithubAppAsset {
        GithubAppAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.test/{}", name),
            size: 42,
            digest: digest.map(str::to_string),
        }
    }

    #[test]
    fn selects_latest_non_draft_app_release() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let releases = vec![
            app_release(
                "v0.1.17",
                true,
                vec![app_asset("AgentDock_0.1.17_universal.dmg", Some(&digest))],
            ),
            app_release(
                "v0.1.16",
                false,
                vec![app_asset("AgentDock_0.1.16_universal.dmg", Some(&digest))],
            ),
            app_release(
                "v0.1.13",
                false,
                vec![app_asset("AgentDock_0.1.13_universal.dmg", Some(&digest))],
            ),
        ];

        let selected = select_app_update(&releases, "0.1.14", "macos", "aarch64")
            .unwrap()
            .expect("a newer release should be selected");
        assert_eq!(selected.version, "0.1.16");
        assert_eq!(selected.asset.name, "AgentDock_0.1.16_universal.dmg");
        assert_eq!(selected.sha256, "a".repeat(64));
        assert!(version_is_newer("v0.1.15", "0.1.14"));
        assert!(!version_is_newer("v0.1.14", "0.1.14"));
    }

    #[test]
    fn selects_universal_macos_app_assets() {
        assert_eq!(
            app_update_asset_name("0.1.15", "macos", "aarch64").unwrap(),
            "AgentDock_0.1.15_universal.dmg"
        );
        assert_eq!(
            app_update_asset_name("0.1.15", "macos", "x86_64").unwrap(),
            "AgentDock_0.1.15_universal.dmg"
        );
        assert!(app_update_asset_name("0.1.15", "linux", "aarch64").is_err());
    }

    #[test]
    fn app_updates_require_a_valid_github_digest() {
        let missing = app_asset("AgentDock_0.1.15_universal.dmg", None);
        assert!(github_asset_sha256(&missing).is_err());

        let invalid = app_asset(
            "AgentDock_0.1.15_universal.dmg",
            Some("sha256:not-a-digest"),
        );
        assert!(github_asset_sha256(&invalid).is_err());

        let release = app_release("v0.1.15", false, vec![missing]);
        assert!(select_app_update(&[release], "0.1.14", "macos", "aarch64").is_err());
    }

    #[test]
    fn app_update_helper_waits_replaces_rolls_back_and_restarts() {
        let script = app_update_helper_script();
        assert!(script.contains("kill -0 \"$APP_PID\""));
        assert!(script.contains("hdiutil attach"));
        assert!(script.contains("ditto \"$SOURCE_APP\" \"$STAGE_PATH\""));
        assert!(script.contains("mv \"$APP_PATH\" \"$BACKUP_PATH\""));
        assert!(script.contains("mv \"$BACKUP_PATH\" \"$APP_PATH\""));
        assert!(script.contains("open \"$APP_PATH\""));
        assert!(script.contains("rm -f \"$DMG_PATH\" \"$SCRIPT_PATH\""));

        #[cfg(unix)]
        {
            let mut child = Command::new("/bin/sh")
                .arg("-n")
                .stdin(Stdio::piped())
                .spawn()
                .expect("the shell syntax checker should start");
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
            assert!(child.wait().unwrap().success());
        }
    }

    #[test]
    fn decompresses_grok_platform_binary() {
        let root = env::temp_dir().join(format!("agentdock-grok-br-test-{}", std::process::id()));
        let bin = root.join("package/bin");
        fs::create_dir_all(&bin).unwrap();
        let payload = b"test-grok-binary";
        let mut compressor = brotli::CompressorReader::new(Cursor::new(payload), 4096, 5, 22);
        let mut compressed = Vec::new();
        compressor.read_to_end(&mut compressed).unwrap();
        let compressed_name = if cfg!(windows) {
            "grok.exe.br"
        } else {
            "grok.br"
        };
        fs::write(bin.join(compressed_name), compressed).unwrap();
        let output = decompress_grok_binary(&root).expect("decompressed Grok binary");
        assert_eq!(fs::read(output).unwrap(), payload);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_legacy_placeholder_launchers() {
        let root = env::temp_dir().join(format!("agentdock-test-{}", std::process::id()));
        let launcher = root.join("codex");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            &launcher,
            "Native client payload will be launched from this managed directory.",
        )
        .unwrap();
        let record = ManagedClientRecord {
            id: "codex".to_string(),
            name: "Codex".to_string(),
            installed: true,
            version: "managed-0.1.0".to_string(),
            install_dir: root.display().to_string(),
            launcher_path: launcher.display().to_string(),
            config_dir: root.join("config").display().to_string(),
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        };
        assert!(!managed_client_is_runnable(&record));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn managed_client_launcher_takes_priority_over_system_executable() {
        let root = env::temp_dir().join(format!(
            "agentdock-managed-client-priority-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let launcher = root.join(if cfg!(windows) {
            "antigravity.cmd"
        } else {
            "antigravity"
        });
        fs::write(&launcher, "managed launcher").unwrap();
        let record = ManagedClientRecord {
            id: "antigravity".to_string(),
            name: "Antigravity CLI".to_string(),
            installed: true,
            version: "1.1.3+gemini.0.40.0".to_string(),
            install_dir: root.display().to_string(),
            launcher_path: launcher.display().to_string(),
            config_dir: root.join("config").display().to_string(),
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        };
        let user_config_dir = root.join(".agy");
        let status = detect_client(
            "antigravity",
            "Antigravity CLI",
            &["node"],
            Some(user_config_dir.clone()),
            &[record],
        );
        assert_eq!(status.executable.as_deref(), launcher.to_str());
        assert_eq!(status.version.as_deref(), Some("1.1.3+gemini.0.40.0"));
        assert_eq!(status.config_path.as_deref(), user_config_dir.to_str());
        assert!(user_config_dir.is_dir());
        assert!(status.managed_by_agentdock);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finds_cli_clients_in_explicit_search_paths() {
        let root = env::temp_dir().join(format!(
            "agentdock-client-path-test-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let bin = root.join(".local/bin");
        fs::create_dir_all(&bin).unwrap();
        let executable = bin.join(if cfg!(windows) { "codex.exe" } else { "codex" });
        fs::write(&executable, "test executable").unwrap();

        assert_eq!(find_executable_in_paths("codex", &[bin]), Some(executable));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn isolates_official_grok_install_directory_from_other_clients() {
        let home = dirs_home().expect("home directory should be available");
        let grok_bin = home.join(".grok/bin");
        assert_eq!(client_command_search_paths("grok").first(), Some(&grok_bin));
        assert!(!client_command_search_paths("codex").contains(&grok_bin));
        assert!(!command_search_paths().contains(&grok_bin));
    }

    #[test]
    fn builds_protocol_aware_provider_model_endpoints() {
        assert_eq!(
            provider_model_endpoints(
                "https://proxy.example.com/api",
                ProviderModelProtocol::Gemini
            ),
            vec![
                "https://proxy.example.com/api/v1beta/models",
                "https://proxy.example.com/api/models"
            ]
        );
        assert_eq!(
            provider_model_endpoints(
                "https://proxy.example.com/v1beta",
                ProviderModelProtocol::Gemini
            ),
            vec!["https://proxy.example.com/v1beta/models"]
        );
        assert_eq!(
            provider_model_endpoints(
                "https://proxy.example.com/v1",
                ProviderModelProtocol::OpenAi
            ),
            vec!["https://proxy.example.com/v1/models"]
        );
        assert_eq!(
            provider_model_endpoints(
                "https://proxy.example.com/v1",
                ProviderModelProtocol::Gemini
            ),
            vec!["https://proxy.example.com/v1/models"]
        );
        assert_eq!(
            provider_model_endpoints("https://proxy.example.com", ProviderModelProtocol::OpenAi),
            vec![
                "https://proxy.example.com/v1/models",
                "https://proxy.example.com/models"
            ]
        );
    }

    #[test]
    fn parses_only_generation_capable_gemini_models() {
        let payload = serde_json::json!({
            "models": [
                {
                    "name": "models/gemini-pro",
                    "supportedGenerationMethods": ["generateContent"]
                },
                {
                    "name": "models/text-embedding-004",
                    "supportedGenerationMethods": ["embedContent"]
                },
                { "name": "models/proxy-model-without-capabilities" }
            ]
        });
        assert_eq!(
            parse_model_ids(&payload, ProviderModelProtocol::Gemini),
            vec!["gemini-pro", "proxy-model-without-capabilities"]
        );
    }

    #[test]
    fn parses_openai_model_ids_without_adding_recommendations() {
        let payload = serde_json::json!({
            "data": [
                { "id": "provider-chat-model" },
                { "id": "provider-reasoning-model" }
            ]
        });
        assert_eq!(
            parse_model_ids(&payload, ProviderModelProtocol::OpenAi),
            vec!["provider-chat-model", "provider-reasoning-model"]
        );
    }

    #[test]
    fn builds_anthropic_message_test_request_for_agentplan() {
        assert_eq!(
            anthropic_messages_endpoint("https://ark.cn-beijing.volces.com/api/coding"),
            "https://ark.cn-beijing.volces.com/api/coding/v1/messages"
        );
        assert_eq!(
            anthropic_messages_endpoint("https://proxy.example.com/v1"),
            "https://proxy.example.com/v1/messages"
        );
        let payload = anthropic_test_payload("ark-code-latest");
        assert_eq!(payload["model"], "ark-code-latest");
        assert_eq!(payload["max_tokens"], 1);
        assert_eq!(payload["messages"][0]["role"], "user");
    }

    #[test]
    fn builds_gemini_generation_test_endpoint() {
        assert_eq!(
            gemini_generate_endpoint("https://proxy.example.com/gemini", "gemini-pro"),
            "https://proxy.example.com/gemini/v1beta/models/gemini-pro:streamGenerateContent?alt=sse"
        );
        assert_eq!(
            gemini_generate_endpoint("https://proxy.example.com/v1beta", "gemini-pro"),
            "https://proxy.example.com/v1beta/models/gemini-pro:streamGenerateContent?alt=sse"
        );
        let payload = gemini_test_payload();
        assert_eq!(payload["contents"][0]["role"], "user");
        assert_eq!(payload["generationConfig"]["maxOutputTokens"], 1);
    }

    #[test]
    fn maps_managed_clients_to_standard_user_config_directories() {
        let home = Path::new("/Users/tester");
        assert_eq!(
            client_user_config_dir_in(home, "antigravity"),
            Some(home.join(".agy"))
        );
        assert_eq!(
            client_user_config_dir_in(home, "grok"),
            Some(home.join(".grok"))
        );
        assert_eq!(
            client_user_config_dir_in(home, "opencode"),
            Some(home.join(".config/opencode"))
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_supported_macos_client_app_bundles() {
        let root = env::temp_dir().join(format!(
            "agentdock-client-app-test-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let bundle = root.join("Codex.app");
        fs::create_dir_all(bundle.join("Contents")).unwrap();
        fs::write(bundle.join("Contents/Info.plist"), "test plist").unwrap();

        assert_eq!(
            find_macos_client_app_in_roots("codex", std::slice::from_ref(&root)),
            Some(bundle)
        );
        assert_eq!(
            find_macos_client_app_in_roots("grok", std::slice::from_ref(&root)),
            None
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn client_version_probe_returns_output() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'client 1.2.3'"]);
        let output =
            command_output_with_timeout(&mut command, std::time::Duration::from_millis(500))
                .unwrap();

        assert_eq!(String::from_utf8_lossy(&output.stdout), "client 1.2.3");
    }

    #[cfg(unix)]
    #[test]
    fn client_version_probe_has_a_timeout() {
        let mut command = Command::new("sh");
        command.args(["-c", "while :; do :; done"]);
        let started = Instant::now();
        let result =
            command_output_with_timeout(&mut command, std::time::Duration::from_millis(40));

        assert!(result.is_err());
        assert!(started.elapsed() < std::time::Duration::from_millis(500));
    }

    #[test]
    fn quotes_shell_values_without_exposing_commands() {
        assert_eq!(shell_quote("simple"), "'simple'");
        assert_eq!(shell_quote("key'with space"), "'key'\"'\"'with space'");
    }

    #[test]
    fn launch_requests_require_a_supported_client_and_unique_identifier() {
        assert!(validate_launch_request("codex", "018f8f48-4d45-7dc8-8a2b-9f54e557a780").is_ok());
        assert!(validate_launch_request("grok", "request_12345678").is_ok());
        assert!(validate_launch_request("unknown", "request_12345678").is_err());
        assert!(validate_launch_request("codex", "previous client").is_err());
    }

    #[test]
    fn managed_cli_proxy_preserves_client_arguments() {
        let args = vec![
            OsString::from("agentdock"),
            OsString::from("--agentdock-cli"),
            OsString::from("grok"),
            OsString::from("--model"),
            OsString::from("grok-4.5"),
        ];
        let (client_id, forwarded) = managed_cli_request(&args).unwrap().unwrap();
        assert_eq!(client_id, "grok");
        assert_eq!(
            forwarded,
            vec![OsString::from("--model"), OsString::from("grok-4.5")]
        );
        assert!(managed_cli_request(&[OsString::from("agentdock")]).is_none());
    }

    #[test]
    fn managed_cli_commands_use_familiar_names() {
        assert_eq!(managed_cli_command_names("codex"), &["codex"]);
        assert_eq!(managed_cli_command_names("claude-code"), &["claude"]);
        assert_eq!(
            managed_cli_command_names("antigravity"),
            &["antigravity", "agy"]
        );
        assert_eq!(managed_cli_command_names("grok"), &["grok"]);
        assert!(managed_cli_command_names("claude-desktop").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn managed_cli_shim_routes_through_agentdock() {
        let content = managed_cli_shim_content(
            Path::new("/Applications/AgentDock.app/Contents/MacOS/agentdock"),
            "grok",
        );
        assert!(content.contains(MANAGED_CLI_MARKER));
        assert!(content.contains("--agentdock-cli 'grok' \"$@\""));
        assert!(content.contains("exec '/Applications/AgentDock.app/Contents/MacOS/agentdock'"));
    }

    #[test]
    fn managed_shell_path_block_is_idempotent_and_preserves_user_content() {
        let block = concat!(
            "# >>> AgentDock CLI >>>\n",
            "export PATH=\"$HOME/.agentdock/bin:$PATH\"\n",
            "# <<< AgentDock CLI <<<"
        );
        let first = upsert_managed_path_block("export EDITOR=vim\n", block).unwrap();
        let second = upsert_managed_path_block(&first, block).unwrap();
        assert_eq!(first, second);
        assert!(second.starts_with("export EDITOR=vim\n"));
        assert_eq!(second.matches(MANAGED_PATH_BLOCK_START).count(), 1);
        assert!(upsert_managed_path_block(MANAGED_PATH_BLOCK_START, block).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn launchers_are_unique_and_bind_the_requested_client_and_directory() {
        let runtime = Path::new("/tmp/agentdock-runtime");
        let codex_path = launch_script_path(runtime, "codex", "request-codex-123", "command");
        let grok_path = launch_script_path(runtime, "grok", "request-grok-456", "command");
        let second_codex_path =
            launch_script_path(runtime, "codex", "request-codex-789", "command");
        assert_ne!(codex_path, grok_path);
        assert_ne!(codex_path, second_codex_path);
        assert!(codex_path
            .to_string_lossy()
            .contains("launch-codex-request-codex-123"));

        let working_directory = Path::new("/tmp/project with 'quote'");
        let content =
            unix_launcher_content("/usr/local/bin/codex", &BTreeMap::new(), working_directory)
                .unwrap();
        assert!(content.contains("cd -- '/tmp/project with '\"'\"'quote'\"'\"''"));
        assert!(content.contains("/bin/rm -f -- \"$0\""));
        assert!(content.contains("exec '/usr/local/bin/codex' \"$@\""));
        assert!(!content.contains("/.grok/bin/grok"));
    }

    #[test]
    fn working_directory_validation_rejects_missing_or_relative_paths() {
        assert!(validate_working_directory("").is_err());
        assert!(validate_working_directory("relative/project").is_err());
        assert!(validate_working_directory("/path/that/does/not/exist").is_err());
    }

    #[test]
    fn antigravity_bundle_marks_legacy_agy_install_as_outdated() {
        let bundle = antigravity_bundle_version("1.1.3");
        assert_eq!(bundle, "1.1.3+gemini.0.40.0");
        assert!(version_is_newer(&bundle, "1.1.3"));
        assert!(!version_is_newer(&bundle, &bundle));
    }

    #[test]
    fn antigravity_launcher_routes_custom_providers_to_gemini() {
        let root = env::temp_dir().join(format!(
            "agentdock-antigravity-launcher-test-{}",
            std::process::id()
        ));
        let agy = root.join(if cfg!(windows) { "agy.exe" } else { "agy" });
        let node = root
            .join("runtime")
            .join(if cfg!(windows) { "node.exe" } else { "node" });
        let gemini = root.join("gemini-cli/package/bundle/gemini.js");
        fs::create_dir_all(node.parent().unwrap()).unwrap();
        fs::create_dir_all(gemini.parent().unwrap()).unwrap();
        fs::write(&agy, b"agy").unwrap();
        fs::write(&node, b"node").unwrap();
        fs::write(&gemini, b"gemini").unwrap();

        let launcher = write_antigravity_launcher(&root, &agy, &node, &gemini)
            .expect("hybrid Antigravity launcher should be written");
        assert!(antigravity_proxy_runtime_ready(
            launcher.to_string_lossy().as_ref()
        ));
        assert!(!antigravity_proxy_runtime_ready(
            agy.to_string_lossy().as_ref()
        ));
        let content = fs::read_to_string(launcher).unwrap();
        assert!(content.contains("GOOGLE_GEMINI_BASE_URL"));
        assert!(content.contains("gemini-cli"));
        assert!(content.contains("agy"));
        #[cfg(not(windows))]
        {
            assert!(content.contains("if [ -n \"${GOOGLE_GEMINI_BASE_URL:-}\" ]"));
            assert!(content.contains("exec \"$ROOT/runtime/node\""));
            assert!(content.contains("exec \"$ROOT/agy\""));
        }
        #[cfg(windows)]
        {
            assert!(content.contains("if not defined GOOGLE_GEMINI_BASE_URL goto agy"));
            assert!(content.contains(":agy"));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn antigravity_gemini_auth_uses_api_key_without_dropping_settings() {
        let root = env::temp_dir().join(format!(
            "agentdock-antigravity-auth-test-{}-{}",
            std::process::id(),
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        let settings_path = root.join(".gemini/settings.json");
        let backup_path = root.join("backup/settings.json");
        fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        fs::create_dir_all(backup_path.parent().unwrap()).unwrap();
        fs::write(
            &settings_path,
            r#"{"theme":"AgentDock","security":{"auth":{"useExternal":false}}}"#,
        )
        .unwrap();

        assert!(merge_antigravity_gemini_settings(&settings_path, Some(&backup_path)).unwrap());
        let saved: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert_eq!(saved["theme"], "AgentDock");
        assert_eq!(saved["security"]["auth"]["useExternal"], false);
        assert_eq!(saved["security"]["auth"]["selectedType"], "gemini-api-key");
        assert_eq!(saved["general"]["maxAttempts"], 1);
        assert_eq!(saved["general"]["retryFetchErrors"], false);
        assert!(backup_path.is_file());
        assert!(!merge_antigravity_gemini_settings(&settings_path, None).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn allocates_scoped_provider_ids_without_overwriting() {
        let mut providers = default_providers();
        providers.push(ProviderProfile {
            id: "codex-deepseek".to_string(),
            name: "DeepSeek".to_string(),
            notes: String::new(),
            website_url: String::new(),
            preset_id: "deepseek".to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            api_format: "chat-completions".to_string(),
            enabled_apps: vec!["codex".to_string()],
            codex_model: "deepseek-chat".to_string(),
            gemini_model: default_gemini_model(),
            claude_sonnet_model: String::new(),
            claude_haiku_model: String::new(),
            claude_opus_model: String::new(),
            active: false,
            active_apps: Vec::new(),
            settings_config: String::new(),
            api_key_configured: true,
            activation_reviewed: true,
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        });
        assert_eq!(
            unique_provider_id("codex-deepseek", &providers),
            "codex-deepseek-2"
        );
        assert_eq!(
            unique_provider_id("claude-code-deepseek", &providers),
            "claude-code-deepseek"
        );
    }

    #[test]
    fn validates_codex_json_and_embedded_toml() {
        let valid = serde_json::json!({
            "auth": { "OPENAI_API_KEY": "${AGENTDOCK_API_KEY}" },
            "config": "model = \"gpt-5.5\"\n"
        });
        assert!(validate_provider_settings_config("codex", &valid.to_string()).is_ok());

        let invalid = serde_json::json!({
            "auth": {},
            "config": "model = [not valid toml"
        });
        assert!(validate_provider_settings_config("codex", &invalid.to_string()).is_err());
        assert!(validate_provider_settings_config("claude-code", "[]").is_err());
    }

    #[test]
    fn builds_native_grok_provider_config_without_embedding_secrets() {
        let provider = ProviderProfile {
            id: "grok-deepseek".to_string(),
            name: "DeepSeek".to_string(),
            notes: String::new(),
            website_url: String::new(),
            preset_id: "deepseek".to_string(),
            provider_type: "openai".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            api_format: "chat-completions".to_string(),
            settings_config: String::new(),
            enabled_apps: vec!["grok".to_string()],
            codex_model: "deepseek-chat".to_string(),
            gemini_model: default_gemini_model(),
            claude_sonnet_model: String::new(),
            claude_haiku_model: String::new(),
            claude_opus_model: String::new(),
            active: true,
            active_apps: vec!["grok".to_string()],
            api_key_configured: true,
            activation_reviewed: true,
            created_at: now_rfc3339(),
            updated_at: now_rfc3339(),
        };
        let config = grok_provider_toml(&provider).expect("Grok config");
        let parsed: toml::Table = config.parse().expect("valid TOML");
        assert_eq!(parsed["models"]["default"].as_str(), Some("agentdock"));
        assert_eq!(
            parsed["model"]["agentdock"]["model"].as_str(),
            Some("deepseek-chat")
        );
        assert_eq!(
            parsed["model"]["agentdock"]["api_backend"].as_str(),
            Some("chat_completions")
        );
        assert!(!config.contains("api_key ="));
        let wrapped = serde_json::json!({ "config": config });
        assert!(validate_provider_settings_config("grok", &wrapped.to_string()).is_ok());
    }

    #[test]
    fn grok_launch_uses_the_default_alias_from_custom_config() {
        let custom = serde_json::json!({
            "config": r#"[models]
default = "grok"

[model.grok]
model = "grok-4.5"
base_url = "https://relay.example/v1"
api_backend = "responses""#,
            "env": {
                "GROK_DEFAULT_MODEL": "agentdock",
                "XAI_API_KEY": "${AGENTDOCK_API_KEY}"
            }
        });

        assert_eq!(
            grok_default_model_from_settings(Some(&custom)).as_deref(),
            Some("grok")
        );
        assert_eq!(
            grok_default_model_from_settings(None).as_deref(),
            Some("agentdock")
        );
    }

    #[test]
    fn grok_launch_does_not_force_an_alias_missing_from_custom_config() {
        let custom = serde_json::json!({
            "config": "[model.grok]\nmodel = \"grok-4.5\"\n"
        });

        assert_eq!(grok_default_model_from_settings(Some(&custom)), None);
        assert_eq!(
            grok_default_model_from_config("[models]\ndefault = \"bad\\nmodel\"\n"),
            None
        );
    }

    #[test]
    fn grok_mcp_sync_preserves_provider_model_config() {
        let root = env::temp_dir().join(format!("agentdock-grok-mcp-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("config.toml");
        fs::write(
            &path,
            "[models]\ndefault = \"agentdock\"\n\n[model.agentdock]\nmodel = \"deepseek-chat\"\n",
        )
        .unwrap();
        let server = McpServerRecord {
            id: "memory".to_string(),
            name: "Memory".to_string(),
            description: String::new(),
            homepage: String::new(),
            docs: String::new(),
            tags: Vec::new(),
            transport: "stdio".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-memory".to_string(),
            ],
            env: BTreeMap::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            extra: BTreeMap::new(),
            apps: vec!["grok".to_string()],
            enabled: true,
            updated_at: now_rfc3339(),
        };
        let mut written = Vec::new();
        let mut errors = Vec::new();
        sync_toml_mcp_projection(&path, true, "grok", &[server], &mut written, &mut errors);
        assert!(errors.is_empty());
        let value: toml::Table = fs::read_to_string(&path).unwrap().parse().unwrap();
        assert_eq!(value["models"]["default"].as_str(), Some("agentdock"));
        assert_eq!(
            value["mcp_servers"]["memory"]["command"].as_str(),
            Some("npx")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn diagnostic_score_penalizes_each_affected_category_once() {
        let report = finalize_diagnostics(vec![
            diagnostic_check("ok", "系统", "pass", "正常", "", ""),
            diagnostic_check("warn", "客户端", "warning", "警告", "", ""),
            diagnostic_check("warn-2", "客户端", "warning", "另一个警告", "", ""),
            diagnostic_check("error", "供应商", "error", "错误", "", ""),
            diagnostic_check("error-2", "供应商", "error", "另一个错误", "", ""),
        ]);
        assert_eq!(report.passed, 1);
        assert_eq!(report.warnings, 2);
        assert_eq!(report.failed, 2);
        assert_eq!(report.score, 75);
    }

    #[test]
    fn diagnostics_only_test_active_providers_for_installed_clients() {
        let installed = HashSet::from(["codex".to_string()]);
        let inactive = provider_test_profile("inactive", &["codex"], &[]);
        let active = provider_test_profile("active", &["codex"], &["codex"]);
        let other_client = provider_test_profile("other-client", &["openclaw"], &["openclaw"]);

        assert!(!provider_is_active_for_diagnostics(&inactive, &installed));
        assert!(provider_is_active_for_diagnostics(&active, &installed));
        assert!(!provider_is_active_for_diagnostics(
            &other_client,
            &installed
        ));
    }

    #[test]
    fn deleting_a_provider_does_not_activate_an_inactive_fallback() {
        let mut providers = vec![
            provider_test_profile("selected", &["openclaw"], &["openclaw"]),
            provider_test_profile("model-name", &["openclaw"], &[]),
        ];

        assert!(remove_provider_profile(&mut providers, "selected"));
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "model-name");
        assert!(!providers[0].active);
        assert!(providers[0].active_apps.is_empty());
    }

    #[test]
    fn provider_keys_are_loaded_only_for_an_explicit_edit() {
        let providers = vec![provider_test_profile("provider", &["codex"], &["codex"])];
        let secrets = BTreeMap::from([("provider".to_string(), "saved-secret".to_string())]);

        assert_eq!(
            provider_api_key_for_edit("provider", &providers, &secrets).unwrap(),
            "saved-secret"
        );
        assert!(provider_api_key_for_edit("missing", &providers, &secrets).is_err());
    }

    #[test]
    fn legacy_cc_switch_activation_is_reconciled_only_once() {
        let candidate = CcSwitchProviderCandidate {
            source_id: "anthropic-custom".to_string(),
            source_app: "openclaw".to_string(),
            app_id: "openclaw".to_string(),
            name: "claude-sonnet-model".to_string(),
            settings_config: serde_json::json!({}),
            website_url: String::new(),
            category: String::new(),
            notes: String::new(),
            meta: serde_json::json!({}),
            is_current: false,
        };
        let id = cc_switch_provider_id(&candidate);
        let mut provider = provider_test_profile(&id, &["openclaw"], &["openclaw"]);
        provider.activation_reviewed = false;
        let mut providers = vec![provider];

        assert!(reconcile_cc_switch_activations(
            &mut providers,
            std::slice::from_ref(&candidate)
        ));
        assert!(!providers[0].active);
        assert!(providers[0].active_apps.is_empty());
        assert!(providers[0].activation_reviewed);

        providers[0].active = true;
        providers[0].active_apps.push("openclaw".to_string());
        assert!(!reconcile_cc_switch_activations(
            &mut providers,
            &[candidate]
        ));
        assert!(providers[0].active);
    }

    #[test]
    fn materializes_api_key_placeholders_recursively() {
        let mut value = serde_json::json!({
            "env": {
                "TOKEN": "${AGENTDOCK_API_KEY}",
                "HEADER": "Bearer ${AGENTDOCK_API_KEY}"
            }
        });
        replace_json_placeholder(&mut value, "secret-value");
        assert_eq!(value["env"]["TOKEN"], "secret-value");
        assert_eq!(value["env"]["HEADER"], "Bearer secret-value");
    }

    #[test]
    fn builds_encoded_npm_registry_urls() {
        assert_eq!(
            npm_metadata_url("https://registry.npmmirror.com", "@openai/codex", "latest"),
            "https://registry.npmmirror.com/@openai%2Fcodex/latest"
        );
    }

    #[test]
    fn imports_opencode_mcp_into_unified_format() {
        let value = serde_json::json!({
            "type": "local",
            "command": ["npx", "-y", "@upstash/context7-mcp"],
            "environment": { "API_TOKEN": "test" },
            "timeout": 45
        });
        let record = mcp_record_from_value("Context 7", &value, "opencode", "opencode")
            .expect("valid OpenCode MCP server");
        assert_eq!(record.id, "Context 7");
        assert_eq!(record.transport, "stdio");
        assert_eq!(record.command, "npx");
        assert_eq!(record.args, vec!["-y", "@upstash/context7-mcp"]);
        assert_eq!(record.env.get("API_TOKEN"), Some(&"test".to_string()));
        assert_eq!(record.extra.get("timeout"), Some(&serde_json::json!(45)));
        assert_eq!(record.apps, vec!["opencode"]);
    }

    #[tokio::test]
    #[ignore = "downloads official Agy, Node.js, and Gemini CLI packages"]
    async fn downloads_antigravity_hybrid_bundle() {
        let root = env::temp_dir().join(format!(
            "agentdock-antigravity-download-test-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();

        let (launcher, version, message) = download_antigravity_cli(&root)
            .await
            .expect("Antigravity hybrid bundle should download");
        assert!(antigravity_proxy_runtime_ready(
            launcher.to_string_lossy().as_ref()
        ));
        assert!(version.contains("+gemini.0.40.0"));
        assert!(message.contains("完整性校验通过"));

        let node = find_file_named(
            &root.join("runtime"),
            if cfg!(windows) { "node.exe" } else { "node" },
        )
        .expect("managed Node executable");
        let gemini = find_file_named(&root.join("gemini-cli"), "gemini.js")
            .expect("managed Gemini CLI entry");
        let output = Command::new(node)
            .arg(gemini)
            .arg("--version")
            .output()
            .expect("managed Gemini CLI should launch");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0.40.0");
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    #[ignore = "downloads the current official Grok platform package"]
    async fn downloads_current_grok_bundle() {
        let root = env::temp_dir().join(format!(
            "agentdock-grok-download-test-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();

        let (launcher, version, message) = download_grok_from_npm(&root)
            .await
            .expect("Grok platform bundle should download");
        let detected = command_version(launcher.to_string_lossy().as_ref())
            .expect("downloaded Grok should report a version");
        assert!(version_is_newer(&version, "0.2.101"));
        assert!(detected.contains(&version));
        assert!(message.contains("SHA-512"));
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn discovers_tools_from_stdio_mcp_server() {
        let node = find_executable("node").expect("Node.js is required by the desktop build");
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp-tools-server.mjs");
        let server = McpServerRecord {
            id: "test-tools".to_string(),
            name: "Test tools".to_string(),
            description: String::new(),
            homepage: String::new(),
            docs: String::new(),
            tags: Vec::new(),
            transport: "stdio".to_string(),
            command: node.display().to_string(),
            args: vec![fixture.display().to_string()],
            env: BTreeMap::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            extra: BTreeMap::new(),
            apps: vec!["codex".to_string()],
            enabled: true,
            updated_at: now_rfc3339(),
        };
        let dirs = agentdock_dirs().expect("AgentDock directories");
        let tools = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            discover_mcp_tools(&dirs, &server),
        )
        .await
        .expect("MCP discovery should not time out")
        .expect("MCP discovery should succeed");

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "search_notes");
        assert_eq!(tools[0].input_schema["required"][0], "query");
        assert_eq!(tools[0].input_schema["properties"]["limit"]["default"], 20);
    }

    #[test]
    fn projects_remote_mcp_headers_to_codex_toml() {
        let record = McpServerRecord {
            id: "remote".to_string(),
            name: "Remote".to_string(),
            description: String::new(),
            homepage: String::new(),
            docs: String::new(),
            tags: Vec::new(),
            transport: "http".to_string(),
            command: "https://mcp.example.com".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            headers: BTreeMap::from([("Authorization".to_string(), "Bearer key".to_string())]),
            cwd: String::new(),
            extra: BTreeMap::from([("timeout".to_string(), serde_json::json!(30))]),
            apps: vec!["codex".to_string()],
            enabled: true,
            updated_at: now_rfc3339(),
        };
        let value = mcp_toml_projection(&record);
        assert_eq!(
            value.get("url").and_then(toml::Value::as_str),
            Some("https://mcp.example.com")
        );
        assert_eq!(
            value
                .get("http_headers")
                .and_then(toml::Value::as_table)
                .and_then(|headers| headers.get("Authorization"))
                .and_then(toml::Value::as_str),
            Some("Bearer key")
        );
        assert_eq!(
            value.get("timeout").and_then(toml::Value::as_integer),
            Some(30)
        );
    }

    #[test]
    fn parses_usage_timestamps_and_known_pricing() {
        let timestamp = parse_json_timestamp(&serde_json::json!(1_750_000_000_000_i64))
            .expect("millisecond timestamp");
        assert_eq!(timestamp.unix_timestamp(), 1_750_000_000);
        let cost = estimate_model_cost("gpt-5.6-sol", 1_000_000, 1_000_000, 1_000_000)
            .expect("known pricing");
        assert!((cost - 35.5).abs() < f64::EPSILON);
        assert!(estimate_model_cost("unknown-model", 10, 10, 0).is_none());
    }

    #[test]
    fn mcp_json_sync_preserves_unrelated_client_settings() {
        let root = env::temp_dir().join(format!("agentdock-mcp-sync-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("settings.json");
        fs::write(
            &path,
            r#"{"theme":"dark","mcpServers":{"legacy":{"command":"old"}}}"#,
        )
        .unwrap();
        let server = McpServerRecord {
            id: "中文服务".to_string(),
            name: "中文服务".to_string(),
            description: String::new(),
            homepage: String::new(),
            docs: String::new(),
            tags: Vec::new(),
            transport: "stdio".to_string(),
            command: "npx".to_string(),
            args: vec!["server".to_string()],
            env: BTreeMap::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            extra: BTreeMap::from([("timeout".to_string(), serde_json::json!(30))]),
            apps: vec!["claude-code".to_string()],
            enabled: true,
            updated_at: now_rfc3339(),
        };
        let mut written = Vec::new();
        let mut errors = Vec::new();
        sync_json_mcp_projection(
            &path,
            true,
            "mcpServers",
            "claude-code",
            "standard",
            &[server],
            &mut written,
            &mut errors,
        );
        assert!(errors.is_empty());
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(value["mcpServers"]["中文服务"]["timeout"], 30);
        assert_eq!(value["theme"], "dark");
        assert_eq!(value["mcpServers"]["中文服务"]["command"], "npx");
        assert!(value["mcpServers"].get("legacy").is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
