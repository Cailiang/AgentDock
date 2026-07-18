<div align="center">

<img src="public/agentdock-logo.svg" width="104" height="104" alt="AgentDock-Logo">

# AgentDock

### Die einsteigerfreundliche Desktop-Zentrale für KI-Coding-Clients

[![Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml/badge.svg)](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-4f6f68)](https://github.com/Cailiang/AgentDock/actions)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-2f5f55)](LICENSE)

[English](README.md) | [简体中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch

</div>

AgentDock vereint Installation und Verwaltung von KI-Coding-Clients, Anbietern, Skills und MCP-Servern in einer nativen Desktop-Anwendung. Es richtet sich an Nutzer, die Codex, Claude Code, Grok oder andere Agenten verwenden möchten, ohne Laufzeitumgebungen manuell zu installieren oder JSON-, TOML- und Umgebungsdateien zu bearbeiten.

> AgentDock `0.1.17` ist eine frühe Vorschauversion. Sichern Sie wichtige Client-Konfigurationen, bevor Sie Anbieter wechseln oder MCP synchronisieren.

## Warum AgentDock?

KI-Coding-Clients verwenden unterschiedliche Installationsverfahren, Konfigurationsformate, Modellprotokolle und MCP-Strukturen. Für erfahrene Entwickler ist das beherrschbar, für neue Nutzer entsteht jedoch eine hohe Einstiegshürde.

AgentDock stellt den Einsteigerablauf in den Vordergrund:

1. Bereits installierte Clients erkennen.
2. Einen Client mit einem Klick installieren oder aktualisieren.
3. Offizielle Anmeldung, Anbieter-Vorlage oder eigene kompatible API hinzufügen.
4. Verbindung testen, erzeugte Konfiguration prüfen und den Client starten.

Endnutzer müssen Node.js, npm oder Python nicht separat installieren und keine Konfigurationsdateien manuell bearbeiten. Benötigte Laufzeitumgebungen werden im AgentDock-Datenverzeichnis verwaltet.

## Hauptfunktionen

### Client-Verwaltung

- Systeminstallationen und von AgentDock verwaltete Installationen erkennen.
- Verwaltete Clients installieren, aktualisieren, starten und deinstallieren.
- Für Festlandchina geeignete npm-/PyPI-Spiegel bevorzugen und bei Fehlern auf offizielle Quellen zurückfallen.
- Das passende Paket für Betriebssystem und CPU-Architektur auswählen.
- Pakete prüfen, wenn die Quelle einen Digest oder npm-Integritätswert veröffentlicht.

### Anbieter-Verwaltung

- Anbieter für jeden unterstützten Client getrennt verwalten.
- Vorlagen, offizielle Anmeldung und vollständig eigene Endpunkte verwenden.
- Modelllisten abrufen und das Standardmodell über eine Auswahlliste festlegen.
- Je nach Client OpenAI Responses, Chat Completions, Anthropic Messages und Gemini-kompatible Protokolle unterstützen.
- Verbindung testen, Konfiguration anzeigen und bearbeiten, Anbieter wechseln und bestehende Dateien vor dem Schreiben sichern.

### Skills und MCP

- Skills installieren, entfernen, pro Client aktivieren und mit echten Client-Verzeichnissen synchronisieren.
- MCP-Server aus Vorlagen oder Rohkonfigurationen hinzufügen.
- Vorhandene MCP-Konfigurationen aus unterstützten Clients importieren.
- `stdio`-, HTTP- und SSE-Server zwischen Clients synchronisieren, ohne unabhängige Einstellungen zu ersetzen.
- Verbindung zu MCP-Servern herstellen und Werkzeuge, Beschreibungen, Annotationen sowie Ein-/Ausgabe-Schemas anzeigen.

### Allgemeine Einstellungen

- Zwischen vereinfachtem Chinesisch, traditionellem Chinesisch, Englisch, Japanisch und Deutsch wechseln.
- Helles, dunkles oder an das System angepasstes Erscheinungsbild verwenden.
- Start bei Anmeldung, stillen Start und Minimieren in den Infobereich konfigurieren.
- Unter macOS im Hintergrund nach GitHub-Releases suchen und über den Verbindungsstatus aktualisieren und neu starten.
- Das bevorzugte Terminal zum Starten von Kommandozeilen-Clients festlegen.
- Sichtbare Clients und ihre Reihenfolge in der Client-Liste konfigurieren.
- Skills im AgentDock-Datenverzeichnis oder unter `~/.agents/skills` speichern und per Dateikopie oder symbolischem Link synchronisieren.

### Nutzung und Diagnose

- Lokale Sitzungen von Codex, Claude Code, OpenCode und Grok lesen.
- Tokens, Anfragen, berechenbare Kosten und Trends für 7/30/90 Tage anzeigen.
- Nach Client, Anbieter oder Modell aufschlüsseln.
- Verzeichnisrechte, Installationen, Updates, Anbieter-Verbindungen, MCP-Konfiguration und Nutzungsquellen prüfen.
- Bereinigte Diagnoseberichte ohne konfigurierte Geheimwerte exportieren.

## Unterstützte Clients

| Client | Erkennung | Installation / Update | Anbieter | MCP |
| --- | :---: | :---: | :---: | :---: |
| Codex | Ja | Ja | Ja | Ja |
| Claude Code | Ja | Ja | Ja | Ja |
| Antigravity CLI (Agy) | Ja | Ja | Ja | Ja |
| Grok | Ja | Ja | Ja | Ja |
| OpenCode | Ja | Ja | Ja | Ja |
| OpenClaw | Ja | Ja | Ja | Ja |
| Hermes Agent | Ja | Ja | Ja | Ja |
| Claude Desktop | Ja | Nein | Ja | Ja |

Claude Desktop wird erkannt und kann Anbieter- oder MCP-Konfigurationen erhalten. AgentDock lädt die Desktop-Anwendung selbst jedoch nicht herunter und deinstalliert sie nicht.

## Download und Installation

Versionierte Vorschaupakete für Windows, macOS und Linux werden als Vorabversionen auf der [Releases](https://github.com/Cailiang/AgentDock/releases)-Seite veröffentlicht. Erfolgreiche [Desktop Build](https://github.com/Cailiang/AgentDock/actions/workflows/desktop-build.yml)-Läufe behalten zusätzlich ihre Build-Artefakte.

- **Windows:** `.msi` oder `.exe`
- **macOS:** `.dmg` oder `.app`
- **Linux:** `.deb`, `.rpm` oder `.AppImage`

Vorschaupakete können unsigniert oder nicht notarisiert sein und eine Sicherheitswarnung des Betriebssystems auslösen. Für die produktive Verteilung sind Signaturzertifikate der jeweiligen Plattform erforderlich.

## Daten und Sicherheit

- API-Schlüssel werden im lokalen AgentDock-Konfigurationsverzeichnis gespeichert und nicht in dieses Repository eingecheckt.
- Unter Unix erhalten geheime Dateien eingeschränkte Berechtigungen. Die Vorschauversion verwendet noch keinen System-Schlüsselbund oder Anmeldedatentresor.
- Nutzungsstatistiken werden aus lokalen Client-Sitzungen berechnet und nicht von AgentDock hochgeladen.
- Netzwerkzugriff wird für Softwareinformationen und Downloads, Anbietertests und Modellsuche sowie konfigurierte MCP-Verbindungen verwendet.
- Diagnoseexporte entfernen API-Schlüssel, URL-Zugangsdaten, MCP-Umgebungswerte und Header-Werte. Prüfen Sie Berichte dennoch vor dem Teilen.

Informationen zur Meldung von Schwachstellen finden Sie in [SECURITY.md](SECURITY.md).

## Entwicklung

Voraussetzungen:

- Node.js 20.19 oder neuer
- Stabile Rust-Toolchain
- [Tauri-2-Voraussetzungen](https://v2.tauri.app/start/prerequisites/) für die jeweilige Plattform

```bash
npm ci
npm run dev
```

Desktop-Pakete bauen:

```bash
npm run build
```

Entwicklungsprüfungen ausführen:

```bash
npm run build:ui
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Desktop-Pakete werden unter `src-tauri/target/release/bundle/` erzeugt.

## FAQ

<details>
<summary><strong>Müssen Nutzer Node.js, npm, Python oder Rust installieren?</strong></summary>

Nein. Dies sind Entwicklungsabhängigkeiten. AgentDock lädt native Pakete oder verwaltet benötigte Laufzeitumgebungen in seinem eigenen Datenverzeichnis.

</details>

<details>
<summary><strong>Warum kann AgentDock einen erkannten System-Client nicht deinstallieren?</strong></summary>

AgentDock entfernt nur Clients aus seinem verwalteten Verzeichnis. Bestehende Systeminstallationen bleiben unangetastet, damit keine Software oder Dateien anderer Installationsprogramme gelöscht werden.

</details>

<details>
<summary><strong>Wo werden AgentDock-Daten gespeichert?</strong></summary>

In den plattformspezifischen Anwendungsdaten- und Konfigurationsverzeichnissen. Öffnen Sie **Diagnose** und wählen Sie **Datenverzeichnis öffnen**, um den aktiven Pfad anzuzeigen.

</details>

<details>
<summary><strong>Lädt AgentDock API-Schlüssel oder Nutzungsverläufe hoch?</strong></summary>

Nein. Für diese Daten sind weder Telemetrie noch Upload-Funktionen implementiert. Ein API-Schlüssel wird nur beim Testen oder Verwenden eines Anbieters an den vom Nutzer gewählten Endpunkt gesendet.

</details>

## Danksagung

Die Anbieter- und MCP-Abläufe von AgentDock wurden von [cc-switch](https://github.com/farion1231/cc-switch) inspiriert. Den MIT-Hinweis finden Sie in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Lizenz

AgentDock-eigener Quellcode und eigene Assets stehen unter der [MIT-Lizenz](LICENSE), Copyright (c) 2026 Cailiang.

Namen, Logos und Marken von Drittanbieter-Clients dienen nur zur Kennzeichnung der Kompatibilität und sind nicht Teil der MIT-Lizenz von AgentDock. Siehe [ASSET_NOTICES.md](ASSET_NOTICES.md).
