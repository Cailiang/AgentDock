const locales = ["zh-TW", "en-US", "ja-JP", "de-DE"];

// Each row is: key, Traditional Chinese, English, Japanese, German.
const rows = [
  ["跳到主要内容", "跳到主要內容", "Skip to main content", "メインコンテンツへ移動", "Zum Hauptinhalt springen"],
  ["主导航", "主導覽", "Main navigation", "メインナビゲーション", "Hauptnavigation"],
  ["打开设置", "開啟設定", "Open settings", "設定を開く", "Einstellungen öffnen"],
  ["当前功能列表", "目前功能清單", "Current feature list", "現在の機能一覧", "Aktuelle Funktionsliste"],
  ["正在检测本机", "正在偵測本機", "Checking this device", "このデバイスを確認中", "Dieses Gerät wird geprüft"],
  ["刷新", "重新整理", "Refresh", "更新", "Aktualisieren"],
  ["刷新当前列表", "重新整理目前清單", "Refresh current list", "現在の一覧を更新", "Aktuelle Liste aktualisieren"],
  ["正在连接本机服务", "正在連線本機服務", "Connecting to local service", "ローカルサービスに接続中", "Verbindung zum lokalen Dienst"],
  ["预览模式", "預覽模式", "Preview mode", "プレビューモード", "Vorschaumodus"],
  ["正在检测", "正在偵測", "Checking", "確認中", "Wird geprüft"],
  ["刷新全部状态", "重新整理全部狀態", "Refresh all status", "すべての状態を更新", "Gesamtstatus aktualisieren"],
  ["选择客户端", "選擇客戶端", "Choose a client", "クライアントを選択", "Client auswählen"],
  ["正在检测本机客户端", "正在偵測本機客戶端", "Checking installed clients", "インストール済みクライアントを確認中", "Installierte Clients werden geprüft"],
  ["扫描本机", "掃描本機", "Scan device", "デバイスをスキャン", "Gerät scannen"],
  ["支持的客户端", "支援的客戶端", "Supported clients", "対応クライアント", "Unterstützte Clients"],
  ["检查新版本", "檢查新版本", "Check for updates", "更新を確認", "Nach Updates suchen"],
  ["正在读取软件目录", "正在讀取軟體目錄", "Loading software catalog", "ソフトウェアカタログを読み込み中", "Softwarekatalog wird geladen"],
  ["内置目录", "內建目錄", "Built-in catalog", "内蔵カタログ", "Integrierter Katalog"],
  ["同步到客户端", "同步至客戶端", "Sync to clients", "クライアントへ同期", "Mit Clients synchronisieren"],
  ["新增服务器", "新增伺服器", "Add server", "サーバーを追加", "Server hinzufügen"],
  ["导入已有", "匯入現有設定", "Import existing", "既存設定をインポート", "Vorhandene importieren"],
  ["刷新统计", "重新整理統計", "Refresh usage", "統計を更新", "Statistik aktualisieren"],
  ["统计时间范围", "統計時間範圍", "Usage time range", "統計期間", "Statistikzeitraum"],
  ["近 7 天", "近 7 天", "Last 7 days", "過去 7 日", "Letzte 7 Tage"],
  ["近 30 天", "近 30 天", "Last 30 days", "過去 30 日", "Letzte 30 Tage"],
  ["近 90 天", "近 90 天", "Last 90 days", "過去 90 日", "Letzte 90 Tage"],
  ["使用趋势", "使用趨勢", "Usage trend", "使用量の推移", "Nutzungstrend"],
  ["按天统计 Token", "按天統計 Token", "Daily token totals", "日別 Token 集計", "Token pro Tag"],
  ["趋势指标", "趨勢指標", "Trend metric", "推移指標", "Trendkennzahl"],
  ["请求", "請求", "Requests", "リクエスト", "Anfragen"],
  ["成本", "成本", "Cost", "コスト", "Kosten"],
  ["用量构成", "用量構成", "Usage breakdown", "使用量の内訳", "Nutzungsaufteilung"],
  ["按 Token 总量排序", "按 Token 總量排序", "Sorted by total tokens", "Token 合計順", "Nach Token-Gesamtmenge"],
  ["统计维度", "統計維度", "Breakdown dimension", "集計軸", "Aufschlüsselung"],
  ["供应商", "供應商", "Providers", "プロバイダー", "Anbieter"],
  ["模型", "模型", "Models", "モデル", "Modelle"],
  ["重新检测", "重新偵測", "Run again", "再診断", "Erneut prüfen"],
  ["导出诊断包", "匯出診斷包", "Export diagnostics", "診断データを出力", "Diagnose exportieren"],
  ["打开数据目录", "開啟資料目錄", "Open data directory", "データフォルダーを開く", "Datenverzeichnis öffnen"],
  ["健康分", "健康分數", "Health score", "健全性スコア", "Zustandswert"],
  ["等待检测", "等待偵測", "Waiting for checks", "診断待ち", "Wartet auf Prüfung"],
  ["通过", "通過", "Passed", "合格", "Bestanden"],
  ["警告", "警告", "Warnings", "警告", "Warnungen"],
  ["错误", "錯誤", "Errors", "エラー", "Fehler"],
  ["尚未运行主动检查", "尚未執行主動檢查", "Active checks have not run", "アクティブ診断は未実行です", "Aktive Prüfungen wurden noch nicht ausgeführt"],
  ["检查结果", "檢查結果", "Check results", "診断結果", "Prüfergebnisse"],
  ["密钥已排除", "已排除金鑰", "Secrets excluded", "シークレットを除外済み", "Geheimnisse ausgeschlossen"],
  ["本机操作记录", "本機操作記錄", "Local operation log", "ローカル操作ログ", "Lokales Vorgangsprotokoll"],
  ["仅保留本次运行期间的操作", "僅保留本次執行期間的操作", "Only operations from this session are kept", "このセッションの操作のみ保持します", "Nur Vorgänge dieser Sitzung werden gespeichert"],
  ["清空", "清除", "Clear", "消去", "Leeren"],
  ["设置分类", "設定分類", "Settings categories", "設定カテゴリ", "Einstellungskategorien"],
  ["预设供应商", "預設供應商", "Provider presets", "プロバイダープリセット", "Anbietervorlagen"],
  ["搜索预设", "搜尋預設", "Search presets", "プリセットを検索", "Vorlagen suchen"],
  ["搜索供应商预设", "搜尋供應商預設", "Search provider presets", "プロバイダープリセットを検索", "Anbietervorlagen suchen"],
  ["选择预设后继续填写 API Key", "選擇預設後繼續填寫 API Key", "Choose a preset, then enter the API Key", "プリセットを選び、API Key を入力してください", "Vorlage auswählen und anschließend den API-Schlüssel eingeben"],
  ["自定义配置", "自訂設定", "Custom configuration", "カスタム設定", "Eigene Konfiguration"],
  ["供应商名称", "供應商名稱", "Provider name", "プロバイダー名", "Anbietername"],
  ["备注", "備註", "Notes", "メモ", "Notizen"],
  ["例如：公司专用账号", "例如：公司專用帳號", "Example: company account", "例：会社用アカウント", "Beispiel: Firmenkonto"],
  ["请求地址", "請求位址", "API endpoint", "API エンドポイント", "API-Endpunkt"],
  ["编辑时留空会保留已有密钥", "編輯時留空會保留現有金鑰", "Leave blank to keep the saved key", "空欄なら保存済みのキーを維持します", "Leer lassen, um den gespeicherten Schlüssel beizubehalten"],
  ["显示或隐藏 API Key", "顯示或隱藏 API Key", "Show or hide API Key", "API Key の表示を切り替え", "API-Schlüssel ein- oder ausblenden"],
  ["默认模型", "預設模型", "Default model", "既定モデル", "Standardmodell"],
  ["正在读取模型", "正在讀取模型", "Loading models", "モデルを読み込み中", "Modelle werden geladen"],
  ["刷新模型列表", "重新整理模型清單", "Refresh model list", "モデル一覧を更新", "Modellliste aktualisieren"],
  ["按客户端读取推荐模型", "按客戶端讀取推薦模型", "Recommended models for this client", "クライアント推奨モデル", "Für diesen Client empfohlene Modelle"],
  ["API 协议", "API 協定", "API protocol", "API プロトコル", "API-Protokoll"],
  ["自动识别", "自動識別", "Auto detect", "自動判定", "Automatisch erkennen"],
  ["高级模型映射", "進階模型對應", "Advanced model mapping", "高度なモデル割り当て", "Erweiterte Modellzuordnung"],
  ["Codex 模型", "Codex 模型", "Codex model", "Codex モデル", "Codex-Modell"],
  ["配置文件内容", "設定檔內容", "Configuration files", "設定ファイル", "Konfigurationsdateien"],
  ["保存后将按此内容生成客户端配置，粘贴现有配置会自动识别地址和密钥", "儲存後將依此內容產生客戶端設定；貼上現有設定可自動識別位址與金鑰", "These files are written to the client. Pasted configuration is parsed for endpoints and keys.", "この内容でクライアント設定を書き込みます。既存設定を貼り付けると接続先とキーを自動検出します。", "Diese Dateien werden in den Client geschrieben. Eingefügte Konfigurationen werden nach Endpunkten und Schlüsseln durchsucht."],
  ["自动生成", "自動產生", "Generated", "自動生成", "Automatisch erzeugt"],
  ["从表单重新生成", "從表單重新產生", "Regenerate from form", "フォームから再生成", "Aus Formular neu erzeugen"],
  ["认证配置", "認證設定", "Authentication", "認証設定", "Authentifizierung"],
  ["认证配置内容", "認證設定內容", "Authentication configuration", "認証設定の内容", "Authentifizierungskonfiguration"],
  ["客户端配置", "客戶端設定", "Client configuration", "クライアント設定", "Client-Konfiguration"],
  ["客户端配置内容", "客戶端設定內容", "Client configuration content", "クライアント設定の内容", "Inhalt der Client-Konfiguration"],
  ["取消", "取消", "Cancel", "キャンセル", "Abbrechen"],
  ["添加并使用", "新增並使用", "Add and use", "追加して使用", "Hinzufügen und verwenden"],
  ["MCP 工具浏览器", "MCP 工具瀏覽器", "MCP tool browser", "MCP ツールブラウザー", "MCP-Werkzeugbrowser"],
  ["返回", "返回", "Back", "戻る", "Zurück"],
  ["返回 MCP 列表", "返回 MCP 清單", "Back to MCP list", "MCP 一覧へ戻る", "Zurück zur MCP-Liste"],
  ["MCP 工具", "MCP 工具", "MCP tools", "MCP ツール", "MCP-Werkzeuge"],
  ["等待连接", "等待連線", "Waiting to connect", "接続待ち", "Wartet auf Verbindung"],
  ["重新读取工具", "重新讀取工具", "Reload tools", "ツールを再読み込み", "Werkzeuge neu laden"],
  ["正在读取工具信息", "正在讀取工具資訊", "Loading tool information", "ツール情報を読み込み中", "Werkzeuginformationen werden geladen"],
  ["工具列表", "工具清單", "Tool list", "ツール一覧", "Werkzeugliste"],
  ["MCP 服务器编辑器", "MCP 伺服器編輯器", "MCP server editor", "MCP サーバーエディター", "MCP-Server-Editor"],
  ["新增 MCP 服务器", "新增 MCP 伺服器", "Add MCP server", "MCP サーバーを追加", "MCP-Server hinzufügen"],
  ["预设配置", "預設設定", "Preset configuration", "プリセット設定", "Vorlagenkonfiguration"],
  ["标题", "標題", "ID", "ID", "ID"],
  ["例如：context7", "例如：context7", "Example: context7", "例：context7", "Beispiel: context7"],
  ["名称", "名稱", "Name", "名前", "Name"],
  ["留空时使用标题", "留空時使用標題", "Uses the ID when blank", "空欄の場合は ID を使用", "Leer: ID wird verwendet"],
  ["启用客户端", "啟用客戶端", "Enabled clients", "有効にするクライアント", "Aktivierte Clients"],
  ["附加信息", "附加資訊", "Additional information", "追加情報", "Zusätzliche Angaben"],
  ["介绍", "介紹", "Description", "説明", "Beschreibung"],
  ["这个服务器能做什么", "這個伺服器能做什麼", "What this server provides", "このサーバーの機能", "Funktion dieses Servers"],
  ["标签", "標籤", "Tags", "タグ", "Tags"],
  ["主页", "首頁", "Homepage", "ホームページ", "Homepage"],
  ["文档", "文件", "Documentation", "ドキュメント", "Dokumentation"],
  ["JSON 配置", "JSON 設定", "JSON configuration", "JSON 設定", "JSON-Konfiguration"],
  ["使用配置向导", "使用設定精靈", "Use configuration wizard", "設定ウィザードを使用", "Konfigurationsassistent verwenden"],
  ["MCP JSON 配置", "MCP JSON 設定", "MCP JSON configuration", "MCP JSON 設定", "MCP-JSON-Konfiguration"],
  ["添加", "新增", "Add", "追加", "Hinzufügen"],
  ["确认操作", "確認操作", "Confirm action", "操作の確認", "Vorgang bestätigen"],
  ["确认删除", "確認刪除", "Delete", "削除", "Löschen"],
  ["MCP 配置向导", "MCP 設定精靈", "MCP configuration wizard", "MCP 設定ウィザード", "MCP-Konfigurationsassistent"],
  ["关闭", "關閉", "Close", "閉じる", "Schließen"],
  ["关闭配置向导", "關閉設定精靈", "Close configuration wizard", "設定ウィザードを閉じる", "Konfigurationsassistent schließen"],
  ["填写服务器信息后生成标准 MCP JSON 配置。", "填寫伺服器資訊後產生標準 MCP JSON 設定。", "Enter server details to generate standard MCP JSON.", "サーバー情報から標準 MCP JSON を生成します。", "Aus Serverangaben wird Standard-MCP-JSON erzeugt."],
  ["传输类型", "傳輸類型", "Transport", "トランスポート", "Transport"],
  ["例如：my-server", "例如：my-server", "Example: my-server", "例：my-server", "Beispiel: my-server"],
  ["Args，每行一个参数", "Args，每行一個參數", "Args, one per line", "Args（1 行に 1 つ）", "Args, ein Wert pro Zeile"],
  ["Env，每行 KEY=VALUE", "Env，每行 KEY=VALUE", "Env, one KEY=VALUE per line", "Env（1 行に KEY=VALUE）", "Env, ein KEY=VALUE pro Zeile"],
  ["Headers，每行 KEY: VALUE", "Headers，每行 KEY: VALUE", "Headers, one KEY: VALUE per line", "Headers（1 行に KEY: VALUE）", "Header, ein KEY: VALUE pro Zeile"],
  ["配置预览", "設定預覽", "Configuration preview", "設定プレビュー", "Konfigurationsvorschau"],
  ["应用配置", "套用設定", "Apply configuration", "設定を適用", "Konfiguration anwenden"],

  ["处理中", "處理中", "Working", "処理中", "Wird verarbeitet"],
  ["该操作正在进行中", "此操作正在進行中", "This operation is already running", "この操作は実行中です", "Dieser Vorgang läuft bereits"],
  ["已安装", "已安裝", "Installed", "インストール済み", "Installiert"],
  ["未安装", "未安裝", "Not installed", "未インストール", "Nicht installiert"],
  ["可更新", "可更新", "Update available", "更新あり", "Update verfügbar"],
  ["可一键安装", "可一鍵安裝", "One-click install", "ワンクリック導入", "Ein-Klick-Installation"],
  ["未检测到", "未偵測到", "Not detected", "未検出", "Nicht erkannt"],
  ["没有匹配的客户端", "沒有符合的客戶端", "No matching clients", "一致するクライアントはありません", "Keine passenden Clients"],
  ["筛选", "篩選", "Filter", "絞り込み", "Filter"],
  ["全部软件", "全部軟體", "All software", "すべてのソフトウェア", "Alle Software"],
  ["推荐", "推薦", "Recommended", "おすすめ", "Empfohlen"],
  ["软件目录", "軟體目錄", "Software catalog", "ソフトウェアカタログ", "Softwarekatalog"],
  ["全部 Skills", "全部 Skills", "All Skills", "すべての Skills", "Alle Skills"],
  ["可安装", "可安裝", "Available", "インストール可能", "Verfügbar"],
  ["Skill 列表", "Skill 清單", "Skill list", "Skill 一覧", "Skill-Liste"],
  ["已添加", "已新增", "Added", "追加済み", "Hinzugefügt"],
  ["还没有 MCP 服务器", "還沒有 MCP 伺服器", "No MCP servers yet", "MCP サーバーはまだありません", "Noch keine MCP-Server"],
  ["时间范围", "時間範圍", "Time range", "期間", "Zeitraum"],
  ["分组方式", "分組方式", "Group by", "グループ化", "Gruppieren nach"],
  ["按客户端", "按客戶端", "By client", "クライアント別", "Nach Client"],
  ["按供应商", "按供應商", "By provider", "プロバイダー別", "Nach Anbieter"],
  ["按模型", "按模型", "By model", "モデル別", "Nach Modell"],
  ["检查状态", "檢查狀態", "Check status", "診断状態", "Prüfstatus"],
  ["全部项目", "全部項目", "All checks", "すべての項目", "Alle Prüfungen"],
  ["诊断报告不包含 API Key、MCP 环境变量值或请求头内容", "診斷報告不包含 API Key、MCP 環境變數值或請求標頭內容", "Diagnostic reports exclude API keys, MCP environment values, and headers", "診断レポートに API Key、MCP 環境変数値、ヘッダー値は含まれません", "Diagnoseberichte enthalten keine API-Schlüssel, MCP-Umgebungswerte oder Header"],
  ["本机", "本機", "Local device", "ローカル", "Lokales Gerät"],
  ["正在检测客户端", "正在偵測客戶端", "Checking clients", "クライアントを確認中", "Clients werden geprüft"],
  ["跟随客户端", "跟隨客戶端", "Client default", "クライアントに従う", "Client-Vorgabe"],
  ["尚未配置", "尚未設定", "Not configured", "未設定", "Nicht konfiguriert"],
  ["打开配置位置", "開啟設定位置", "Open configuration folder", "設定フォルダーを開く", "Konfigurationsordner öffnen"],
  ["正在升级", "正在升級", "Updating", "更新中", "Wird aktualisiert"],
  ["最新版", "最新版本", "latest version", "最新版", "neueste Version"],
  ["卸载", "解除安裝", "Uninstall", "アンインストール", "Deinstallieren"],
  ["正在安装", "正在安裝", "Installing", "インストール中", "Wird installiert"],
  ["当前仅支持本机检测", "目前僅支援本機偵測", "Detection only", "現在は検出のみ対応", "Nur Erkennung unterstützt"],
  ["未检测到可执行程序", "未偵測到可執行程式", "Executable not detected", "実行ファイルが見つかりません", "Ausführbare Datei nicht erkannt"],
  ["客户端就绪状态", "客戶端就緒狀態", "Client readiness", "クライアント準備状況", "Client-Bereitschaft"],
  ["客户端可用", "客戶端可用", "Client ready", "クライアント利用可能", "Client bereit"],
  ["等待安装", "等待安裝", "Waiting to install", "インストール待ち", "Wartet auf Installation"],
  ["可执行文件与配置目录已检测", "已偵測到可執行檔與設定目錄", "Executable and configuration folder detected", "実行ファイルと設定フォルダーを検出済み", "Programm und Konfigurationsordner erkannt"],
  ["安装时会自动处理下载源和依赖", "安裝時會自動處理下載來源與相依套件", "Downloads and dependencies are handled automatically", "ダウンロード元と依存関係を自動処理します", "Downloads und Abhängigkeiten werden automatisch verwaltet"],
  ["添加供应商后即可调用模型", "新增供應商後即可呼叫模型", "Add a provider to use models", "プロバイダーを追加するとモデルを利用できます", "Anbieter hinzufügen, um Modelle zu verwenden"],
  ["可在 MCP 页面启用扩展工具", "可在 MCP 頁面啟用擴充工具", "Enable extension tools on the MCP page", "MCP ページで拡張ツールを有効にできます", "Erweiterungswerkzeuge auf der MCP-Seite aktivieren"],
  ["还没有配置供应商", "還沒有設定供應商", "No providers configured", "プロバイダーは未設定です", "Keine Anbieter konfiguriert"],
  ["添加供应商", "新增供應商", "Add provider", "プロバイダーを追加", "Anbieter hinzufügen"],
  ["配置摘要", "設定摘要", "Configuration summary", "設定概要", "Konfigurationsübersicht"],
  ["由 AgentDock 管理并写入真实客户端配置", "由 AgentDock 管理並寫入實際客戶端設定", "Managed by AgentDock and written to the real client configuration", "AgentDock が管理し、実際のクライアント設定へ書き込みます", "Von AgentDock verwaltet und in die echte Client-Konfiguration geschrieben"],
  ["查看完整文件", "查看完整檔案", "View full files", "ファイル全体を表示", "Vollständige Dateien anzeigen"],
  ["等待生成", "等待產生", "Waiting to generate", "生成待ち", "Wartet auf Erzeugung"],
  ["配置位置", "設定位置", "Configuration location", "設定場所", "Konfigurationsort"],
  ["安装后自动创建", "安裝後自動建立", "Created after installation", "インストール後に作成", "Wird nach der Installation erstellt"],
  ["跟随供应商", "跟隨供應商", "Provider default", "プロバイダーに従う", "Anbieter-Vorgabe"],
  ["一键安装", "一鍵安裝", "Install", "インストール", "Installieren"],
  ["AgentDock 会优先使用国内可用镜像，自动回退备用源并校验官方包完整性。", "AgentDock 會優先使用中國大陸可用鏡像，自動切換備用來源並驗證官方套件完整性。", "AgentDock prefers reachable mirrors, falls back automatically, and verifies official package integrity.", "AgentDock は利用可能なミラーを優先し、自動で代替元へ切り替え、公式パッケージを検証します。", "AgentDock bevorzugt erreichbare Spiegel, wechselt automatisch zu Alternativen und prüft offizielle Pakete."],
  ["安装后重新扫描即可配置供应商。", "安裝後重新掃描即可設定供應商。", "Install it, scan again, and then configure providers.", "インストール後に再スキャンしてプロバイダーを設定できます。", "Nach der Installation erneut scannen und Anbieter konfigurieren."],
  ["当前使用", "目前使用", "Active", "使用中", "Aktiv"],
  ["客户端登录", "客戶端登入", "Client login", "クライアントログイン", "Client-Anmeldung"],
  ["密钥已保存", "金鑰已儲存", "Key saved", "キー保存済み", "Schlüssel gespeichert"],
  ["缺少密钥", "缺少金鑰", "Key missing", "キーがありません", "Schlüssel fehlt"],
  ["官方直连", "官方直連", "Official endpoint", "公式接続", "Offizieller Endpunkt"],
  ["重新应用", "重新套用", "Reapply", "再適用", "Erneut anwenden"],
  ["使用", "使用", "Use", "使用", "Verwenden"],
  ["测试连接", "測試連線", "Test connection", "接続テスト", "Verbindung testen"],
  ["编辑", "編輯", "Edit", "編集", "Bearbeiten"],
  ["删除", "刪除", "Delete", "削除", "Löschen"],
  ["安装", "安裝", "Install", "インストール", "Installieren"],
  ["升级中", "升級中", "Updating", "更新中", "Wird aktualisiert"],
  ["更新", "更新", "Update", "更新", "Aktualisieren"],
  ["管理", "管理", "Manage", "管理", "Verwalten"],
  ["安装中", "安裝中", "Installing", "インストール中", "Wird installiert"],
  ["仅检测", "僅偵測", "Detection only", "検出のみ", "Nur Erkennung"],
  ["等待检查", "等待檢查", "Waiting for check", "確認待ち", "Wartet auf Prüfung"],
  ["当前筛选条件下没有软件", "目前篩選條件下沒有軟體", "No software matches this filter", "この条件に一致するソフトウェアはありません", "Keine Software entspricht diesem Filter"],
  ["当前筛选条件下没有 Skill", "目前篩選條件下沒有 Skill", "No Skills match this filter", "この条件に一致する Skill はありません", "Keine Skills entsprechen diesem Filter"],
  ["个服务器", "個伺服器", "servers", "サーバー", "Server"],
  ["停用", "停用", "Disable", "無効化", "Deaktivieren"],
  ["启用", "啟用", "Enable", "有効化", "Aktivieren"],
  ["打开文档", "開啟文件", "Open documentation", "ドキュメントを開く", "Dokumentation öffnen"],
  ["没有匹配的 MCP 服务器", "沒有符合的 MCP 伺服器", "No matching MCP servers", "一致する MCP サーバーはありません", "Keine passenden MCP-Server"],
  ["导入已有配置", "匯入現有設定", "Import existing configuration", "既存設定をインポート", "Vorhandene Konfiguration importieren"],
  ["Token 总量", "Token 總量", "Total tokens", "Token 合計", "Tokens gesamt"],
  ["请求次数", "請求次數", "Requests", "リクエスト数", "Anfragen"],
  ["本机可识别的完整响应", "本機可識別的完整回應", "Complete responses found locally", "ローカルで識別できた完全な応答", "Lokal erkannte vollständige Antworten"],
  ["已识别请求均已计价", "已識別請求均已計價", "All recognized requests are priced", "認識済みリクエストはすべて価格計算済み", "Alle erkannten Anfragen wurden bepreist"],
  ["暂未发现本地会话记录", "暫未發現本機工作階段記錄", "No local session history found", "ローカルセッション履歴がありません", "Keine lokalen Sitzungen gefunden"],
  ["未分类", "未分類", "Uncategorized", "未分類", "Nicht kategorisiert"],
  ["等待运行诊断", "等待執行診斷", "Waiting to run diagnostics", "診断の実行待ち", "Wartet auf Diagnose"],
  ["点击重新检测后，将检查目录、客户端、供应商、MCP 和统计数据。", "點擊重新偵測後，將檢查目錄、客戶端、供應商、MCP 和統計資料。", "Run diagnostics to check directories, clients, providers, MCP, and usage data.", "再診断を実行すると、フォルダー、クライアント、プロバイダー、MCP、統計を確認します。", "Diagnose starten, um Verzeichnisse, Clients, Anbieter, MCP und Nutzungsdaten zu prüfen."],
  ["基本正常", "基本正常", "Mostly healthy", "ほぼ正常", "Weitgehend in Ordnung"],
  ["需要处理", "需要處理", "Needs attention", "対応が必要", "Handlungsbedarf"],
  ["正在执行完整诊断", "正在執行完整診斷", "Running full diagnostics", "完全診断を実行中", "Vollständige Diagnose läuft"],
  ["正在连接供应商并检查本机配置", "正在連線供應商並檢查本機設定", "Checking providers and local configuration", "プロバイダーとローカル設定を確認中", "Anbieter und lokale Konfiguration werden geprüft"],
  ["检查通过", "檢查通過", "Passed", "合格", "Bestanden"],
  ["发现警告", "發現警告", "Warnings found", "警告あり", "Warnungen gefunden"],
  ["发现错误", "發現錯誤", "Errors found", "エラーあり", "Fehler gefunden"],
  ["不需要处理", "不需要處理", "No action needed", "対応不要", "Keine Aktion erforderlich"],
  ["操作记录为空", "操作記錄為空", "No operations recorded", "操作ログは空です", "Keine Vorgänge protokolliert"],
  ["连接失败", "連線失敗", "Connection failed", "接続失敗", "Verbindung fehlgeschlagen"],
  ["正在连接", "正在連線", "Connecting", "接続中", "Verbindung wird hergestellt"],
  ["未能读取工具列表", "無法讀取工具清單", "Could not load tools", "ツール一覧を取得できません", "Werkzeuge konnten nicht geladen werden"],
  ["无法读取工具", "無法讀取工具", "Unable to load tools", "ツールを読み込めません", "Werkzeuge können nicht geladen werden"],
  ["重新连接", "重新連線", "Reconnect", "再接続", "Erneut verbinden"],
  ["正在初始化服务器并读取工具", "正在初始化伺服器並讀取工具", "Starting server and loading tools", "サーバーを起動してツールを読み込み中", "Server wird gestartet und Werkzeuge werden geladen"],
  ["正在读取工具", "正在讀取工具", "Loading tools", "ツールを読み込み中", "Werkzeuge werden geladen"],
  ["首次连接时，服务器可能需要准备运行依赖。", "首次連線時，伺服器可能需要準備執行相依套件。", "The server may need to prepare dependencies on first connection.", "初回接続では、サーバーが依存関係を準備する場合があります。", "Bei der ersten Verbindung muss der Server möglicherweise Abhängigkeiten vorbereiten."],
  ["服务器没有返回工具", "伺服器沒有回傳工具", "The server returned no tools", "サーバーからツールが返されませんでした", "Der Server hat keine Werkzeuge zurückgegeben"],
  ["没有可展示的工具", "沒有可顯示的工具", "No tools to display", "表示できるツールがありません", "Keine Werkzeuge zum Anzeigen"],
  ["服务器已连接，但没有返回工具定义。", "伺服器已連線，但沒有回傳工具定義。", "The server connected but returned no tool definitions.", "サーバーには接続しましたが、ツール定義がありません。", "Der Server ist verbunden, hat aber keine Werkzeugdefinitionen geliefert."],
  ["只读", "唯讀", "Read only", "読み取り専用", "Schreibgeschützt"],
  ["会修改数据", "會修改資料", "Modifies data", "データを変更", "Ändert Daten"],
  ["可重复调用", "可重複呼叫", "Idempotent", "繰り返し実行可能", "Idempotent"],
  ["可访问外部服务", "可存取外部服務", "External access", "外部アクセス", "Externer Zugriff"],
  ["必填", "必填", "Required", "必須", "Erforderlich"],
  ["可选", "選填", "Optional", "任意", "Optional"],
  ["暂无参数说明", "暫無參數說明", "No parameter description", "パラメーター説明なし", "Keine Parameterbeschreibung"],
  ["此工具不需要参数", "此工具不需要參數", "This tool has no parameters", "このツールにパラメーターはありません", "Dieses Werkzeug hat keine Parameter"],
  ["服务器没有提供工具说明", "伺服器未提供工具說明", "No tool description provided", "ツール説明はありません", "Keine Werkzeugbeschreibung vorhanden"],
  ["参数", "參數", "Parameters", "パラメーター", "Parameter"],
  ["完整输入参数 Schema", "完整輸入參數 Schema", "Full input schema", "完全な入力 Schema", "Vollständiges Eingabeschema"],
  ["输出结果 Schema", "輸出結果 Schema", "Output schema", "出力 Schema", "Ausgabeschema"],
  ["自定义", "自訂", "Custom", "カスタム", "Benutzerdefiniert"],
  ["保存", "儲存", "Save", "保存", "Speichern"],
  ["编辑 MCP 服务器", "編輯 MCP 伺服器", "Edit MCP server", "MCP サーバーを編集", "MCP-Server bearbeiten"],
  ["请填写服务器标题", "請填寫伺服器標題", "Enter a server ID", "サーバー ID を入力してください", "Server-ID eingeben"],
  ["请填写 Command", "請填寫 Command", "Enter a command", "Command を入力してください", "Command eingeben"],
  ["请填写 URL", "請填寫 URL", "Enter a URL", "URL を入力してください", "URL eingeben"],
  ["已手动修改", "已手動修改", "Manually edited", "手動編集済み", "Manuell bearbeitet"],
  ["自定义供应商", "自訂供應商", "Custom provider", "カスタムプロバイダー", "Eigener Anbieter"],
  ["正在读取客户端模型", "正在讀取客戶端模型", "Loading client models", "クライアントモデルを読み込み中", "Client-Modelle werden geladen"],
  ["正在读取供应商模型", "正在讀取供應商模型", "Loading provider models", "プロバイダーモデルを読み込み中", "Anbietermodelle werden geladen"],
  ["暂无可用模型", "暫無可用模型", "No models available", "利用可能なモデルなし", "Keine Modelle verfügbar"],
  ["正在读取模型列表", "正在讀取模型清單", "Loading model list", "モデル一覧を読み込み中", "Modellliste wird geladen"],
  ["官方供应商使用客户端登录，无需填写 API Key", "官方供應商使用客戶端登入，無需填寫 API Key", "Official providers use the client login; no API Key is required", "公式プロバイダーはクライアントログインを使用するため API Key は不要です", "Offizielle Anbieter verwenden die Client-Anmeldung; kein API-Schlüssel erforderlich"],
  ["请求地址已预设，只需填写 API Key", "請求位址已預設，只需填寫 API Key", "The endpoint is preset; enter the API Key", "接続先は設定済みです。API Key を入力してください", "Der Endpunkt ist voreingestellt; API-Schlüssel eingeben"],
  ["确认请求地址后填写 API Key", "確認請求位址後填寫 API Key", "Confirm the endpoint, then enter the API Key", "接続先を確認し、API Key を入力してください", "Endpunkt prüfen und API-Schlüssel eingeben"],
  ["填写供应商提供的请求地址和 API Key", "填寫供應商提供的請求位址與 API Key", "Enter the endpoint and API Key supplied by the provider", "プロバイダーの接続先と API Key を入力してください", "Endpunkt und API-Schlüssel des Anbieters eingeben"],
  ["请填写供应商名称", "請填寫供應商名稱", "Enter a provider name", "プロバイダー名を入力してください", "Anbieternamen eingeben"],
  ["请完整填写供应商名称、请求地址和 API Key", "請完整填寫供應商名稱、請求位址與 API Key", "Enter the provider name, endpoint, and API Key", "プロバイダー名、接続先、API Key を入力してください", "Anbietername, Endpunkt und API-Schlüssel eingeben"],

  ["count.installedRatio", "已安裝 {installed}/{total}", "{installed}/{total} installed", "{installed}/{total} インストール済み", "{installed}/{total} installiert"],
  ["count.tools", "{count} 個工具", "{count} tools", "{count} ツール", "{count} Werkzeuge"],
  ["count.installed", "{count} 個已安裝", "{count} installed", "{count} インストール済み", "{count} installiert"],
  ["count.enabled", "{count} 個已啟用", "{count} enabled", "{count} 有効", "{count} aktiviert"],
  ["count.checks", "{count} 項檢查", "{count} checks", "{count} 項目", "{count} Prüfungen"],
  ["range.days", "近 {days} 天", "Last {days} days", "過去 {days} 日", "Letzte {days} Tage"],
  ["clients.summary", "已安裝 {installed}/{total}，選擇客戶端後設定供應商", "{installed}/{total} installed. Select a client to configure providers.", "{installed}/{total} インストール済み。クライアントを選んでプロバイダーを設定します。", "{installed}/{total} installiert. Client auswählen und Anbieter konfigurieren."],
  ["client.updateTo", "更新至 {version}", "Update to {version}", "{version} に更新", "Auf {version} aktualisieren"],
  ["client.checkUpdate", "檢查 {name} 更新", "Check {name} for updates", "{name} の更新を確認", "Nach Updates für {name} suchen"],
  ["client.uninstall", "解除安裝 {name}", "Uninstall {name}", "{name} をアンインストール", "{name} deinstallieren"],
  ["client.launch", "啟動 {name}", "Launch {name}", "{name} を起動", "{name} starten"],
  ["client.install", "一鍵安裝 {name}", "Install {name}", "{name} をインストール", "{name} installieren"],
  ["client.managed", " · AgentDock 管理", " · AgentDock managed", " · AgentDock 管理", " · Von AgentDock verwaltet"],
  ["client.systemInstall", " · 本機安裝", " · System installation", " · システムインストール", " · Systeminstallation"],
  ["count.providers", "{count} 個供應商", "{count} providers", "{count} プロバイダー", "{count} Anbieter"],
  ["provider.current", "目前使用 {name}", "Using {name}", "現在 {name} を使用中", "Aktuell {name}"],
  ["count.mcp", "{count} 個 MCP", "{count} MCP servers", "{count} MCP", "{count} MCP-Server"],
  ["mcp.syncedTo", "已同步至 {name}", "Synced to {name}", "{name} に同期済み", "Mit {name} synchronisiert"],
  ["provider.configured", "已設定 {count} 個，使用操作會立即寫入 {name}", "{count} configured. Using one writes it to {name} immediately.", "{count} 件設定済み。使用すると {name} へすぐ書き込みます。", "{count} konfiguriert. Die Auswahl wird sofort in {name} geschrieben."],
  ["client.notInstalled", "{name} 尚未安裝", "{name} is not installed", "{name} は未インストールです", "{name} ist nicht installiert"],
  ["software.summary", "{total} 個工具，已安裝 {installed} 個", "{total} tools, {installed} installed", "{total} ツール、{installed} インストール済み", "{total} Werkzeuge, {installed} installiert"],
  ["software.updates", "，{count} 個可更新", ", {count} updates", "、{count} 更新あり", ", {count} Updates"],
  ["software.filtered", " · 目前顯示 {count} 個", " · {count} shown", " · {count} 件表示", " · {count} angezeigt"],
  ["provider.empty", "尚未為 {name} 設定供應商", "No providers configured for {name}", "{name} のプロバイダーは未設定です", "Keine Anbieter für {name} konfiguriert"],
  ["mcp.serverCount", "{count} 個伺服器", "{count} servers", "{count} サーバー", "{count} Server"],
  ["stats.range", "{from} 至 {to}", "{from} to {to}", "{from} から {to}", "{from} bis {to}"],
  ["stats.tokenParts", "輸入 {input} · 輸出 {output} · 快取 {cached}", "Input {input} · Output {output} · Cached {cached}", "入力 {input} · 出力 {output} · キャッシュ {cached}", "Eingabe {input} · Ausgabe {output} · Cache {cached}"],
  ["stats.unpriced", "{count} 次請求缺少公開價格，未計入", "{count} requests have no public price and are excluded", "{count} 件は公開価格がなく未計上", "{count} Anfragen ohne öffentlichen Preis wurden nicht einbezogen"],
  ["stats.sources", "資料來源：{sources}", "Sources: {sources}", "データソース：{sources}", "Quellen: {sources}"],
  ["stats.errors", "；{count} 個檔案讀取失敗，詳見診斷記錄", "; {count} files could not be read; see diagnostics", "；{count} ファイルを読み込めません。診断ログを確認してください", "; {count} Dateien konnten nicht gelesen werden; siehe Diagnose"],
  ["stats.disclaimer", "。成本依客戶端記錄或內建公開單價估算。", ". Cost is estimated from client records or built-in public pricing.", "。コストはクライアント記録または公開価格からの推定です。", ". Kosten werden aus Client-Daten oder öffentlichen Preisen geschätzt."],
  ["stats.daily", "按天統計{metric}", "Daily {metric}", "日別{metric}", "{metric} pro Tag"],
  ["stats.chartLabel", "{days} 天{metric}趨勢", "{metric} trend for {days} days", "{days} 日間の{metric}推移", "{metric}-Trend für {days} Tage"],
  ["diagnostics.resultCount", "{shown}/{total} 項", "{shown}/{total} checks", "{shown}/{total} 項目", "{shown}/{total} Prüfungen"],
  ["diagnostics.detectedAt", "偵測於 {time}", "Checked at {time}", "{time} に診断", "Geprüft um {time}"],
  ["logs.count", "{shown}/{total} 筆", "{shown}/{total} entries", "{shown}/{total} 件", "{shown}/{total} Einträge"],
  ["mcp.toolsTitle", "{name} 的工具", "Tools from {name}", "{name} のツール", "Werkzeuge von {name}"],
  ["mcp.toolsSummary", "{count} 個工具 · {latency} ms", "{count} tools · {latency} ms", "{count} ツール · {latency} ms", "{count} Werkzeuge · {latency} ms"],
  ["provider.editorTitle", "{action} {name} 供應商", "{action} {name} provider", "{name} プロバイダーを{action}", "{name}-Anbieter {action}"],
  ["provider.app", "{name} 供應商", "{name} provider", "{name} プロバイダー", "{name}-Anbieter"],
  ["provider.models", "{source} · {count} 個模型", "{source} · {count} models", "{source} · {count} モデル", "{source} · {count} Modelle"],
  ["provider.importSource", "{format} 設定匯入", "Imported {format} configuration", "{format} 設定からインポート", "Aus {format}-Konfiguration importiert"],
  ["provider.importedFields", "已識別 {fields}", "Detected {fields}", "{fields} を検出", "{fields} erkannt"],
  ["mcp.openTools", "查看 {name} 的工具清單", "View tools from {name}", "{name} のツール一覧を表示", "Werkzeuge von {name} anzeigen"],
  ["mcp.toggleApp", "{action} {name}", "{action} {name}", "{name} を{action}", "{name} {action}"],
  ["mcp.openDocs", "開啟 {name} 文件", "Open {name} documentation", "{name} のドキュメントを開く", "Dokumentation für {name} öffnen"],
  ["mcp.defaultValue", "預設值 {value}", "Default {value}", "既定値 {value}", "Standard {value}"],
  ["mcp.enumValues", "可選值 {value}", "Allowed values {value}", "候補値 {value}", "Zulässige Werte {value}"],
  ["mcp.minimum", "最小 {value}", "Minimum {value}", "最小 {value}", "Minimum {value}"],
  ["mcp.maximum", "最大 {value}", "Maximum {value}", "最大 {value}", "Maximum {value}"],
  ["mcp.pattern", "格式 {value}", "Pattern {value}", "形式 {value}", "Muster {value}"],
  ["confirm.uninstallClient", "解除安裝 {name}", "Uninstall {name}", "{name} をアンインストール", "{name} deinstallieren"],
  ["confirm.uninstallClientMessage", "只會刪除 AgentDock 管理的客戶端檔案，供應商設定會保留。", "Only AgentDock-managed client files are removed. Provider configuration is kept.", "AgentDock 管理のクライアントファイルのみ削除し、プロバイダー設定は保持します。", "Nur von AgentDock verwaltete Client-Dateien werden entfernt. Anbieterkonfigurationen bleiben erhalten."],
  ["confirm.uninstall", "確認解除安裝", "Uninstall", "アンインストール", "Deinstallieren"],
  ["confirm.deleteProvider", "刪除供應商", "Delete provider", "プロバイダーを削除", "Anbieter löschen"],
  ["confirm.deleteProviderMessage", "確定刪除「{name}」？已寫入客戶端的設定不會自動清除。", "Delete “{name}”? Configuration already written to clients is not removed automatically.", "「{name}」を削除しますか？クライアントへ書き込んだ設定は自動削除されません。", "„{name}“ löschen? Bereits in Clients geschriebene Konfigurationen werden nicht automatisch entfernt."],
  ["confirm.uninstallSkill", "解除安裝 Skill", "Uninstall Skill", "Skill をアンインストール", "Skill deinstallieren"],
  ["confirm.uninstallSkillMessage", "確定解除安裝「{name}」？解除安裝前會自動備份。", "Uninstall “{name}”? A backup is created first.", "「{name}」をアンインストールしますか？先にバックアップを作成します。", "„{name}“ deinstallieren? Zuvor wird eine Sicherung erstellt."],
  ["confirm.deleteMcp", "刪除 MCP 伺服器", "Delete MCP server", "MCP サーバーを削除", "MCP-Server löschen"],
  ["confirm.deleteMcpMessage", "確定刪除「{name}」？它也會從已啟用客戶端的設定中移除。", "Delete “{name}”? It is also removed from enabled client configurations.", "「{name}」を削除しますか？有効なクライアント設定からも削除されます。", "„{name}“ löschen? Der Server wird auch aus aktivierten Client-Konfigurationen entfernt."],
  ["action.add", "新增", "Add", "追加", "hinzufügen"],
  ["action.edit", "編輯", "Edit", "編集", "bearbeiten"],
  ["action.saved", "{name} 已儲存", "{name} saved", "{name} を保存しました", "{name} gespeichert"],
  ["action.installed", "{name} 已安裝", "{name} installed", "{name} をインストールしました", "{name} installiert"],
  ["action.uninstalled", "{name} 已解除安裝", "{name} uninstalled", "{name} をアンインストールしました", "{name} deinstalliert"],
  ["action.using", "正在使用 {name}", "Using {name}", "{name} を使用中", "{name} wird verwendet"],
  ["action.applied", "{provider} 已套用至 {client}", "{provider} applied to {client}", "{provider} を {client} に適用しました", "{provider} auf {client} angewendet"],
  ["action.importMcp", "已匯入 {imported} 個伺服器，關聯 {linked} 個客戶端設定", "Imported {imported} servers and linked {linked} client configurations", "{imported} サーバーをインポートし、{linked} クライアント設定に関連付けました", "{imported} Server importiert und {linked} Client-Konfigurationen verknüpft"],
  ["action.noMcp", "沒有發現新的 MCP 設定", "No new MCP configuration found", "新しい MCP 設定はありません", "Keine neue MCP-Konfiguration gefunden"],
  ["action.statusRefreshed", "狀態已重新整理", "Status refreshed", "状態を更新しました", "Status aktualisiert"],
  ["action.settingsReloaded", "設定已重新讀取", "Settings reloaded", "設定を再読み込みしました", "Einstellungen neu geladen"],
  ["action.catalogRefreshed", "版本資訊已更新", "Version information updated", "バージョン情報を更新しました", "Versionsinformationen aktualisiert"],
  ["action.configRegenerated", "設定檔已從表單重新產生", "Configuration regenerated from the form", "フォームから設定を再生成しました", "Konfiguration aus dem Formular neu erzeugt"],
  ["action.statsRefreshed", "統計已重新整理", "Usage refreshed", "統計を更新しました", "Statistik aktualisiert"],
  ["action.diagnosticsComplete", "診斷已完成", "Diagnostics complete", "診断が完了しました", "Diagnose abgeschlossen"],
  ["action.diagnosticsExported", "診斷包已匯出", "Diagnostics exported", "診断データを出力しました", "Diagnose exportiert"],
  ["action.logsCleared", "記錄已清除", "Logs cleared", "ログを消去しました", "Protokoll geleert"]
  ,["重新检测客户端", "重新偵測客戶端", "Check clients again", "クライアントを再確認", "Clients erneut prüfen"]
  ,["检查软件更新", "檢查軟體更新", "Check software updates", "ソフトウェア更新を確認", "Software-Updates prüfen"]
  ,["同步 Skills", "同步 Skills", "Sync Skills", "Skills を同期", "Skills synchronisieren"]
  ,["添加 MCP 服务器", "新增 MCP 伺服器", "Add MCP server", "MCP サーバーを追加", "MCP-Server hinzufügen"]
  ,["尚未检测", "尚未偵測", "Not checked", "未診断", "Nicht geprüft"]
  ,["重新运行诊断", "重新執行診斷", "Run diagnostics again", "診断を再実行", "Diagnose erneut ausführen"]
  ,["重新读取设置", "重新讀取設定", "Reload settings", "設定を再読み込み", "Einstellungen neu laden"]
  ,["status.updateSuffix", " · 可更新", " · Update available", " · 更新あり", " · Update verfügbar"]
  ,["status.installedVersion", "已安裝 {version}", "Installed {version}", "{version} インストール済み", "Version {version} installiert"]
  ,["provider.test", "測試 {name}", "Test {name}", "{name} をテスト", "{name} testen"]
  ,["provider.edit", "編輯 {name}", "Edit {name}", "{name} を編集", "{name} bearbeiten"]
  ,["provider.delete", "刪除 {name}", "Delete {name}", "{name} を削除", "{name} löschen"]
  ,["当前范围没有用量数据", "目前範圍沒有用量資料", "No usage data in this range", "この期間の使用量データはありません", "Keine Nutzungsdaten in diesem Zeitraum"]
  ,["本次运行还没有操作记录", "本次執行尚無操作記錄", "No operations in this session", "このセッションの操作はありません", "Keine Vorgänge in dieser Sitzung"]
  ,["运行一次诊断以查看本机状态", "執行診斷以查看本機狀態", "Run diagnostics to check this device", "診断を実行してデバイス状態を確認", "Diagnose ausführen, um dieses Gerät zu prüfen"]
  ,["供应商连接检查可能需要几秒钟", "供應商連線檢查可能需要幾秒鐘", "Provider connection checks may take a few seconds", "プロバイダー接続の確認には数秒かかる場合があります", "Anbieterverbindungen können einige Sekunden dauern"]
  ,["将检查目录、客户端、供应商、MCP 和统计数据", "將檢查目錄、客戶端、供應商、MCP 與統計資料", "Directories, clients, providers, MCP, and usage data will be checked", "フォルダー、クライアント、プロバイダー、MCP、統計を確認します", "Verzeichnisse, Clients, Anbieter, MCP und Nutzungsdaten werden geprüft"]
  ,["运行良好", "運作良好", "Healthy", "正常", "In Ordnung"]
  ,["没有匹配的检查项", "沒有符合的檢查項目", "No matching checks", "一致する診断項目はありません", "Keine passenden Prüfungen"]
  ,["调整左侧状态筛选或搜索内容", "調整左側狀態篩選或搜尋內容", "Change the status filter or search query", "左側の状態フィルターまたは検索条件を変更してください", "Statusfilter oder Suche anpassen"]
  ,["auth.json 不是有效 JSON", "auth.json 不是有效的 JSON", "auth.json is not valid JSON", "auth.json は有効な JSON ではありません", "auth.json ist kein gültiges JSON"]
  ,["auth.json 必须是 JSON 对象", "auth.json 必須是 JSON 物件", "auth.json must be a JSON object", "auth.json は JSON オブジェクトである必要があります", "auth.json muss ein JSON-Objekt sein"]
  ,["config.toml 不能为空", "config.toml 不可為空", "config.toml cannot be empty", "config.toml は空にできません", "config.toml darf nicht leer sein"]
  ,["配置文件内容必须是 JSON 对象", "設定檔內容必須是 JSON 物件", "Configuration content must be a JSON object", "設定内容は JSON オブジェクトである必要があります", "Konfigurationsinhalt muss ein JSON-Objekt sein"]
  ,["保存并使用", "儲存並使用", "Save and use", "保存して使用", "Speichern und verwenden"]
  ,["provider.appliedFiles", "{client} 已使用供應商 {provider}，寫入 {count} 個設定檔", "{client} now uses {provider}; {count} configuration files written", "{client} で {provider} を使用し、{count} 設定ファイルを書き込みました", "{client} verwendet jetzt {provider}; {count} Konfigurationsdateien geschrieben"]
  ,["provider.added", "{name} 已新增並啟用", "{name} added and enabled", "{name} を追加して有効にしました", "{name} hinzugefügt und aktiviert"]
  ,["error.invalidJson", "{name} 不是有效的 JSON", "{name} is not valid JSON", "{name} は有効な JSON ではありません", "{name} ist kein gültiges JSON"]
  ,["error.invalidFormat", "{label} 格式錯誤：{error}", "Invalid {label}: {error}", "{label} の形式エラー：{error}", "Ungültiges Format für {label}: {error}"]
  ,["error.mustBeObject", "{label} 必須是 JSON 物件", "{label} must be a JSON object", "{label} は JSON オブジェクトである必要があります", "{label} muss ein JSON-Objekt sein"]
  ,["error.unsupportedTransport", "不支援的傳輸類型：{transport}", "Unsupported transport: {transport}", "未対応のトランスポート：{transport}", "Nicht unterstützter Transport: {transport}"]
  ,["stdio 配置必须包含 command", "stdio 設定必須包含 command", "stdio configuration must include command", "stdio 設定には command が必要です", "stdio-Konfiguration muss command enthalten"]
  ,["error.missingUrl", "{transport} 設定必須包含 url", "{transport} configuration must include url", "{transport} 設定には url が必要です", "{transport}-Konfiguration muss url enthalten"]
  ,["env 必须是 JSON 对象", "env 必須是 JSON 物件", "env must be a JSON object", "env は JSON オブジェクトである必要があります", "env muss ein JSON-Objekt sein"]
  ,["headers 必须是 JSON 对象", "headers 必須是 JSON 物件", "headers must be a JSON object", "headers は JSON オブジェクトである必要があります", "headers muss ein JSON-Objekt sein"]
  ,["error.serverExists", "伺服器 ID「{id}」已存在", "Server ID “{id}” already exists", "サーバー ID「{id}」は既に存在します", "Server-ID „{id}“ ist bereits vorhanden"]
  ,["推荐客户端已处理", "推薦客戶端已處理", "Recommended clients processed", "推奨クライアントを処理しました", "Empfohlene Clients verarbeitet"]
  ,["action.uninstalling", "正在解除安裝 {name}", "Uninstalling {name}", "{name} をアンインストール中", "{name} wird deinstalliert"]
  ,["diagnostics.exportPath", "診斷包已匯出：{path}", "Diagnostics exported: {path}", "診断データを出力しました：{path}", "Diagnose exportiert: {path}"]
  ,["error.actionNotImplemented", "按鈕動作尚未實作：{action}", "Button action not implemented: {action}", "ボタン操作が未実装です：{action}", "Schaltflächenaktion nicht implementiert: {action}"]
  ,["读取网页并转换为适合模型处理的内容", "讀取網頁並轉換為適合模型處理的內容", "Fetch web pages and convert them for model use", "Web ページを取得してモデル向けに変換", "Webseiten abrufen und für Modelle aufbereiten"]
  ,["获取时区时间并进行时间转换", "取得時區時間並進行時間轉換", "Read and convert time across time zones", "タイムゾーンの時刻取得と変換", "Zeit über Zeitzonen hinweg lesen und umrechnen"]
  ,["使用知识图谱保存跨会话记忆", "使用知識圖譜保存跨工作階段記憶", "Store cross-session memory in a knowledge graph", "知識グラフでセッション間メモリを保存", "Sitzungsübergreifendes Wissen in einem Wissensgraphen speichern"]
  ,["为复杂问题提供分步骤推理工具", "為複雜問題提供分步推理工具", "Provide step-by-step reasoning tools for complex problems", "複雑な問題の段階的推論ツール", "Schrittweise Denkwerkzeuge für komplexe Probleme"]
  ,["查询开发库的最新文档和代码示例", "查詢開發程式庫的最新文件與程式碼範例", "Search current library documentation and code examples", "最新のライブラリ文書とコード例を検索", "Aktuelle Bibliotheksdokumentation und Codebeispiele durchsuchen"]
  ,["客户端", "客戶端", "Clients", "クライアント", "Clients"]
  ,["推荐软件", "推薦軟體", "Software", "おすすめ", "Software"]
  ,["统计", "統計", "Usage", "統計", "Statistik"]
  ,["诊断", "診斷", "Diagnostics", "診断", "Diagnose"]
  ,["设置", "設定", "Settings", "設定", "Einstellungen"]
  ,["筛选当前列表", "篩選目前清單", "Filter current list", "現在の一覧を絞り込む", "Aktuelle Liste filtern"]
  ,["界面语言", "介面語言", "Interface language", "表示言語", "Oberflächensprache"]
  ,["外观主题", "外觀主題", "Appearance", "外観テーマ", "Darstellung"]
  ,["首选终端", "偏好終端機", "Preferred terminal", "優先ターミナル", "Bevorzugtes Terminal"]
  ,["Skills 存储位置", "Skills 儲存位置", "Skills storage", "Skills の保存場所", "Skills-Speicherort"]
  ,["Skills 同步方式", "Skills 同步方式", "Skills sync method", "Skills の同期方法", "Skills-Synchronisierung"]
  ,["供应商编辑器", "供應商編輯器", "Provider editor", "プロバイダーエディター", "Anbieter-Editor"]
  ,["返回客户端", "返回客戶端", "Back to client", "クライアントへ戻る", "Zurück zum Client"]
  ,["命令提示符", "命令提示字元", "Command Prompt", "コマンドプロンプト", "Eingabeaufforderung"]
  ,["主力线路", "主要線路", "Primary route", "メインルート", "Primäre Route"]
  ,["AI 编程客户端", "AI 程式設計客戶端", "AI coding client", "AI コーディングクライアント", "KI-Coding-Client"]
  ,["software.manageClient", "安裝和管理 {name} 客戶端", "Install and manage the {name} client", "{name} クライアントをインストール・管理", "{name}-Client installieren und verwalten"]
  ,["代码变更风险审查", "程式碼變更風險審查", "Review code change risks", "コード変更のリスクレビュー", "Risiken von Codeänderungen prüfen"]
  ,["浏览器交互和页面检查", "瀏覽器互動與頁面檢查", "Browser interaction and page inspection", "ブラウザー操作とページ検証", "Browserinteraktion und Seitenprüfung"]
  ,["界面视觉检查", "介面視覺檢查", "Visual interface review", "画面のビジュアルレビュー", "Visuelle Oberflächenprüfung"]
  ,["访问指定的本地目录", "存取指定的本機目錄", "Access selected local directories", "指定したローカルフォルダーへアクセス", "Auf ausgewählte lokale Verzeichnisse zugreifen"]
  ,["Codex 会话", "Codex 工作階段", "Codex sessions", "Codex セッション", "Codex-Sitzungen"]
  ,["Claude Code 会话", "Claude Code 工作階段", "Claude Code sessions", "Claude Code セッション", "Claude-Code-Sitzungen"]
  ,["OpenCode 会话", "OpenCode 工作階段", "OpenCode sessions", "OpenCode セッション", "OpenCode-Sitzungen"]
  ,["Grok 会话", "Grok 工作階段", "Grok sessions", "Grok セッション", "Grok-Sitzungen"]
  ,["系统", "系統", "System", "システム", "System"]
  ,["桌面运行环境", "桌面執行環境", "Desktop runtime", "デスクトップ実行環境", "Desktop-Laufzeit"]
  ,["数据目录可写", "資料目錄可寫入", "Data directory is writable", "データフォルダーは書き込み可能", "Datenverzeichnis ist beschreibbar"]
  ,["配置目录可写", "設定目錄可寫入", "Configuration directory is writable", "設定フォルダーは書き込み可能", "Konfigurationsverzeichnis ist beschreibbar"]
  ,["运行目录可写", "執行目錄可寫入", "Runtime directory is writable", "ランタイムフォルダーは書き込み可能", "Laufzeitverzeichnis ist beschreibbar"]
  ,["检查目录权限或磁盘剩余空间", "檢查目錄權限或磁碟剩餘空間", "Check directory permissions and free disk space", "フォルダー権限と空き容量を確認してください", "Verzeichnisrechte und freien Speicher prüfen"]
  ,["无法检查客户端最新版本", "無法檢查客戶端最新版本", "Could not check latest client versions", "クライアントの最新版を確認できません", "Neueste Client-Versionen konnten nicht geprüft werden"]
  ,["检查网络后重新诊断", "檢查網路後重新診斷", "Check the network and run diagnostics again", "ネットワークを確認して再診断してください", "Netzwerk prüfen und Diagnose erneut ausführen"]
  ,["版本未知", "版本未知", "Unknown version", "バージョン不明", "Unbekannte Version"]
  ,["AgentDock 托管", "AgentDock 管理", "AgentDock managed", "AgentDock 管理", "Von AgentDock verwaltet"]
  ,["系统安装", "系統安裝", "System installation", "システムインストール", "Systeminstallation"]
  ,["启动路径未知", "啟動路徑未知", "Unknown launch path", "起動パス不明", "Unbekannter Startpfad"]
  ,["未知", "未知", "Unknown", "不明", "Unbekannt"]
  ,["前往客户端页面点击更新", "前往客戶端頁面點擊更新", "Open the Clients page and select Update", "クライアントページで更新を選択してください", "Auf der Client-Seite Aktualisieren wählen"]
  ,["客户端已安装，但还没有可读取的配置位置", "客戶端已安裝，但尚無可讀取的設定位置", "The client is installed but has no readable configuration location", "クライアントはインストール済みですが、設定場所を読み取れません", "Der Client ist installiert, aber es gibt keinen lesbaren Konfigurationsort"]
  ,["添加并启用一个供应商后会自动生成", "新增並啟用供應商後會自動產生", "Add and enable a provider to generate it automatically", "プロバイダーを追加して有効にすると自動生成されます", "Nach dem Hinzufügen und Aktivieren eines Anbieters wird sie automatisch erstellt"]
  ,["这是推荐客户端，可由 AgentDock 自动安装", "這是推薦客戶端，可由 AgentDock 自動安裝", "This recommended client can be installed by AgentDock", "推奨クライアントです。AgentDock からインストールできます", "Dieser empfohlene Client kann von AgentDock installiert werden"]
  ,["前往客户端页面点击一键安装", "前往客戶端頁面點擊一鍵安裝", "Open the Clients page and select Install", "クライアントページでインストールを選択してください", "Auf der Client-Seite Installieren wählen"]
  ,["还没有可用供应商", "還沒有可用供應商", "No usable providers", "利用可能なプロバイダーがありません", "Keine nutzbaren Anbieter"]
  ,["客户端无法在没有登录或 API Key 的情况下调用模型", "客戶端無法在未登入或沒有 API Key 的情況下呼叫模型", "Clients cannot use models without a login or API Key", "ログインまたは API Key がないとモデルを利用できません", "Clients können Modelle ohne Anmeldung oder API-Schlüssel nicht verwenden"]
  ,["在已安装客户端中添加供应商", "在已安裝客戶端中新增供應商", "Add a provider to an installed client", "インストール済みクライアントにプロバイダーを追加してください", "Einem installierten Client einen Anbieter hinzufügen"]
  ,["未关联客户端", "未關聯客戶端", "No linked clients", "関連クライアントなし", "Keine verknüpften Clients"]
  ,["连接测试通过", "連線測試通過", "Connection test passed", "接続テスト成功", "Verbindungstest bestanden"]
  ,["编辑供应商的请求地址或 API Key 后重试", "編輯供應商的請求位址或 API Key 後重試", "Check the provider endpoint or API Key and retry", "プロバイダーの接続先または API Key を修正して再試行してください", "Anbieter-Endpunkt oder API-Schlüssel prüfen und erneut versuchen"]
  ,["检查供应商配置后重试", "檢查供應商設定後重試", "Check the provider configuration and retry", "プロバイダー設定を確認して再試行してください", "Anbieterkonfiguration prüfen und erneut versuchen"]
  ,["启动时将使用客户端自身的登录状态", "啟動時將使用客戶端自身的登入狀態", "The client will use its own login at launch", "起動時はクライアント自身のログインを使用します", "Beim Start wird die eigene Client-Anmeldung verwendet"]
  ,["启动时将使用客户端自身的登录状态或默认配置", "啟動時將使用客戶端自身的登入狀態或預設設定", "The client will use its own login or default configuration at launch", "起動時はクライアント自身のログインまたは既定設定を使用します", "Beim Start wird die eigene Anmeldung oder Standardkonfiguration verwendet"]
  ,["在客户端页面选择一个供应商", "在客戶端頁面選擇一個供應商", "Select a provider on the Clients page", "クライアントページでプロバイダーを選択してください", "Auf der Client-Seite einen Anbieter auswählen"]
  ,["MCP 配置可用", "MCP 設定可用", "MCP configuration is ready", "MCP 設定は利用可能", "MCP-Konfiguration ist bereit"]
  ,["尚未添加 MCP 服务器；这不会影响客户端基础功能", "尚未新增 MCP 伺服器；這不影響客戶端基本功能", "No MCP servers added; core client features are unaffected", "MCP サーバーは未追加ですが、基本機能には影響しません", "Keine MCP-Server hinzugefügt; die Client-Grundfunktionen bleiben verfügbar"]
  ,["没有关联任何客户端", "未關聯任何客戶端", "No clients linked", "クライアントが関連付けられていません", "Keine Clients verknüpft"]
  ,["缺少启动命令", "缺少啟動命令", "Launch command missing", "起動コマンドがありません", "Startbefehl fehlt"]
  ,["打开 MCP 页面修正配置", "開啟 MCP 頁面修正設定", "Open the MCP page to correct the configuration", "MCP ページで設定を修正してください", "MCP-Seite öffnen und Konfiguration korrigieren"]
  ,["本地统计数据可读取", "本機統計資料可讀取", "Local usage data is readable", "ローカル統計データを読み取れます", "Lokale Nutzungsdaten sind lesbar"]
  ,["尚未发现客户端会话记录", "尚未發現客戶端工作階段記錄", "No client session history found", "クライアントのセッション履歴がありません", "Keine Client-Sitzungen gefunden"]
  ,["部分统计数据无法读取", "部分統計資料無法讀取", "Some usage data could not be read", "一部の統計データを読み込めません", "Einige Nutzungsdaten konnten nicht gelesen werden"]
  ,["检查对应会话目录的读取权限", "檢查對應工作階段目錄的讀取權限", "Check read permissions for the session directories", "セッションフォルダーの読み取り権限を確認してください", "Leserechte der Sitzungsverzeichnisse prüfen"]
  ,["统计检查失败", "統計檢查失敗", "Usage check failed", "統計の確認に失敗しました", "Nutzungsprüfung fehlgeschlagen"]
  ,["重新检测或查看诊断包", "重新偵測或查看診斷包", "Run diagnostics again or inspect the export", "再診断するか診断データを確認してください", "Diagnose erneut ausführen oder Export prüfen"]
  ,["diagnostics.clientReady", "{name} 可正常啟動", "{name} can launch", "{name} は起動可能", "{name} kann gestartet werden"]
  ,["diagnostics.clientUpdate", "{name} 有新版本", "{name} has an update", "{name} に更新があります", "Für {name} ist ein Update verfügbar"]
  ,["diagnostics.versions", "目前 {current}，最新 {latest}", "Current {current}, latest {latest}", "現在 {current}、最新 {latest}", "Aktuell {current}, neueste {latest}"]
  ,["diagnostics.configReady", "{name} 設定位置可用", "{name} configuration location is available", "{name} の設定場所を利用できます", "Konfigurationsort für {name} ist verfügbar"]
  ,["diagnostics.configMissing", "{name} 尚未產生設定", "{name} configuration has not been generated", "{name} の設定はまだ生成されていません", "Konfiguration für {name} wurde noch nicht erstellt"]
  ,["diagnostics.providerReady", "{name} 連線正常", "{name} connection is healthy", "{name} の接続は正常です", "Verbindung zu {name} ist in Ordnung"]
  ,["diagnostics.providerUnavailable", "{name} 無法使用", "{name} is unavailable", "{name} は利用できません", "{name} ist nicht verfügbar"]
  ,["diagnostics.providerFailed", "{name} 檢查失敗", "{name} check failed", "{name} の確認に失敗しました", "Prüfung von {name} fehlgeschlagen"]
  ,["diagnostics.providerMissing", "{name} 沒有目前供應商", "{name} has no active provider", "{name} に有効なプロバイダーがありません", "{name} hat keinen aktiven Anbieter"]
  ,["diagnostics.mcpValid", "{name} 設定有效", "{name} configuration is valid", "{name} の設定は有効です", "Konfiguration von {name} ist gültig"]
  ,["diagnostics.mcpInvalid", "{name} 設定無效", "{name} configuration is invalid", "{name} の設定は無効です", "Konfiguration von {name} ist ungültig"]
  ,["diagnostics.enabledClients", "已啟用至 {count} 個客戶端", "Enabled for {count} clients", "{count} クライアントで有効", "Für {count} Clients aktiviert"]
  ,["diagnostics.sources", "已識別：{sources}", "Detected: {sources}", "検出：{sources}", "Erkannt: {sources}"]
  ,["action.clientInstalled", "{name} 已安裝或更新", "{name} installed or updated", "{name} をインストールまたは更新しました", "{name} installiert oder aktualisiert"]
  ,["action.clientLaunched", "{name} 已啟動", "{name} launched", "{name} を起動しました", "{name} gestartet"]
  ,["action.providerDeleted", "{name} 已刪除", "{name} deleted", "{name} を削除しました", "{name} gelöscht"]
  ,["action.skillsSynced", "Skills 已同步", "Skills synchronized", "Skills を同期しました", "Skills synchronisiert"]
  ,["action.mcpDeleted", "{name} 已刪除", "{name} deleted", "{name} を削除しました", "{name} gelöscht"]
  ,["action.mcpSynced", "MCP 設定已同步", "MCP configuration synchronized", "MCP 設定を同期しました", "MCP-Konfiguration synchronisiert"]
  ,["appUpdate.updateTo", "更新至 {version}", "Update to {version}", "{version} に更新", "Auf {version} aktualisieren"]
  ,["appUpdate.installing", "正在升級", "Updating", "更新中", "Aktualisierung"]
  ,["appUpdate.restartHint", "升級至 {version} 並自動重新啟動", "Update to {version} and restart automatically", "{version} に更新して自動的に再起動", "Auf {version} aktualisieren und automatisch neu starten"]
  ,["appUpdate.previewComplete", "預覽模式已完成升級流程模擬", "The update flow simulation is complete", "プレビューモードで更新フローを完了しました", "Die Aktualisierung wurde im Vorschaumodus simuliert"]
  ,["appUpdate.checkFailed", "檢查 AgentDock 更新失敗：{error}", "Failed to check for AgentDock updates: {error}", "AgentDock の更新確認に失敗しました：{error}", "AgentDock-Aktualisierung konnte nicht geprüft werden: {error}"]
  ,["settings.import", "設定移轉", "Configuration migration", "設定の移行", "Konfigurationsmigration"]
  ,["settings.importDesc", "從本機既有工具匯入供應商設定，寫入前會備份原設定", "Import provider settings from existing local tools; current settings are backed up first", "既存のローカルツールからプロバイダー設定をインポートし、書き込み前に現在の設定をバックアップします", "Anbietereinstellungen aus vorhandenen lokalen Tools importieren; aktuelle Einstellungen werden vorher gesichert"]
  ,["配置迁移", "設定移轉", "Configuration migration", "設定の移行", "Konfigurationsmigration"]
  ,["从本机已有工具导入供应商配置，原配置会在写入前备份", "從本機既有工具匯入供應商設定，寫入前會備份原設定", "Import provider settings from existing local tools; current settings are backed up first", "既存のローカルツールからプロバイダー設定をインポートし、書き込み前に現在の設定をバックアップします", "Anbietereinstellungen aus vorhandenen lokalen Tools importieren; aktuelle Einstellungen werden vorher gesichert"]
  ,["尚未检查本机配置", "尚未檢查本機設定", "Local configuration has not been checked", "ローカル設定は未確認です", "Lokale Konfiguration wurde noch nicht geprüft"]
  ,["检查并导入", "檢查並匯入", "Check and import", "確認してインポート", "Prüfen und importieren"]
  ,["正在检查 cc-switch 配置", "正在檢查 cc-switch 設定", "Checking cc-switch configuration", "cc-switch 設定を確認中", "cc-switch-Konfiguration wird geprüft"]
  ,["未检测到 cc-switch 供应商配置", "未偵測到 cc-switch 供應商設定", "No cc-switch provider configuration found", "cc-switch のプロバイダー設定が見つかりません", "Keine cc-switch-Anbieterkonfiguration gefunden"]
  ,["发现 cc-switch 配置", "發現 cc-switch 設定", "cc-switch configuration found", "cc-switch 設定が見つかりました", "cc-switch-Konfiguration gefunden"]
  ,["导入配置", "匯入設定", "Import configuration", "設定をインポート", "Konfiguration importieren"]
  ,["已暂不导入 cc-switch 配置", "已暫不匯入 cc-switch 設定", "cc-switch import was skipped for now", "cc-switch のインポートを今回は見送りました", "cc-switch-Import wurde vorerst übersprungen"]
  ,["ccSwitch.detectedStatus", "偵測到 {count} 個可匯入供應商 · {source}", "Found {count} importable providers · {source}", "インポート可能なプロバイダー {count} 件 · {source}", "{count} importierbare Anbieter gefunden · {source}"]
  ,["ccSwitch.importedStatus", "已匯入 {imported} 個，更新 {updated} 個", "Imported {imported}, updated {updated}", "{imported} 件をインポート、{updated} 件を更新", "{imported} importiert, {updated} aktualisiert"]
  ,["ccSwitch.importPrompt", "在 {source} 中發現 {count} 個可匯入供應商（{apps}）。匯入前會備份 AgentDock 現有設定；不會修改 cc-switch，也不會立即覆蓋客戶端設定。", "Found {count} importable providers in {source} ({apps}). AgentDock backs up its current settings first; cc-switch and live client configurations are not modified.", "{source} にインポート可能なプロバイダーが {count} 件見つかりました（{apps}）。AgentDock の現在の設定を先にバックアップし、cc-switch やクライアントの実設定は変更しません。", "In {source} wurden {count} importierbare Anbieter gefunden ({apps}). AgentDock sichert zuerst seine aktuellen Einstellungen; cc-switch und aktive Client-Konfigurationen werden nicht geändert."]
  ,["ccSwitch.importResult", "已從 cc-switch 匯入 {imported} 個供應商，更新 {updated} 個", "Imported {imported} providers from cc-switch and updated {updated}", "cc-switch から {imported} 件をインポートし、{updated} 件を更新しました", "{imported} Anbieter aus cc-switch importiert und {updated} aktualisiert"]
  ,["confirm.unsavedProviderTitle", "放棄未儲存的供應商設定？", "Discard unsaved provider configuration?", "未保存のプロバイダー設定を破棄しますか？", "Nicht gespeicherte Anbieterkonfiguration verwerfen?"]
  ,["confirm.unsavedProviderMessage", "{current} 的供應商設定尚未儲存。切換至 {next} 後，這些變更將會遺失。", "The provider configuration for {current} has not been saved. Switching to {next} will discard these changes.", "{current} のプロバイダー設定は保存されていません。{next} に切り替えると、これらの変更は失われます。", "Die Anbieterkonfiguration für {current} wurde nicht gespeichert. Beim Wechsel zu {next} gehen diese Änderungen verloren."]
  ,["confirm.discardAndSwitch", "放棄並切換", "Discard and switch", "破棄して切り替える", "Verwerfen und wechseln"]
  ,["客户端不支持", "客戶端不支援", "Unsupported by client", "クライアント未対応", "Vom Client nicht unterstützt"]
  ,["本机服务连接失败", "本機服務連線失敗", "Local service connection failed", "ローカルサービスへの接続に失敗しました", "Verbindung zum lokalen Dienst fehlgeschlagen"]
];

export const EXTENDED_UI_COPY = Object.fromEntries(
  locales.map((locale, localeIndex) => [
    locale,
    Object.fromEntries(rows.map((row) => [row[0], row[localeIndex + 1]]))
  ])
);
