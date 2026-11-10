# Rust Git CLI

[中文](README.md) | [English](README.en.md) | [日本語](README.ja.md) | Deutsch

Ein intelligentes Git-Commit-Tool, das mit KI zweisprachige (Chinesisch/Englisch) Commit-Nachrichten generiert.

## Funktionen

- **KI-gesteuert** - Unterstützt OpenAI, Anthropic und benutzerdefinierte Endpunkte (z.B. DeepSeek)
- **Zweisprachige Commits** - Generiert automatisch Chinesisch/Englische Commit-Nachrichten nach Conventional Commits
- **Intelligentes Staging** - Erkennt nicht gestagete Änderungen und fordert zur Bestätigung auf
- **Interaktive Benutzeroberfläche** - Farbige Ausgabe, Diff-Vorschau, Commit-Bestätigung
- **KI-Changelog** - Commits interaktiv auswählen, KI-Changelog-Zusammenfassung generieren, in Zwischenablage kopieren
- **KI-Review** - KI-Review-Bericht für aktuelle Änderungen (Risiken/Vorschläge/Tests)
- **Konfigurationsverwaltung** - `config` zeigt, findet, bearbeitet und ändert die Konfiguration; `config doctor` prüft Schlüssel und Endpunkt
- **Skriptfähig** - `commit --dry-run` gibt nur die Nachricht aus, `log --export` schreibt sie in eine Datei, `hook install` bringt KI in ein normales `git commit`

## Installation

```bash
# Aus Quellcode bauen
git clone https://github.com/duanyongcheng/rust-git-cli.git
cd rust-git-cli
cargo build --release

# Im System installieren
cargo install --path .
```

**Voraussetzungen**: Rust 1.70+, Git 2.0+

## Schnellstart

### 1. Konfiguration initialisieren

```bash
rust-git-cli init                 # Globale Konfiguration erstellen (~/.config/rust-git-cli/config.toml)
rust-git-cli init --local         # Projektkonfiguration erstellen (.rust-git-cli.toml)
```

### 2. API-Schlüssel einrichten

```bash
# Empfohlen: Umgebungsvariablen (api_key_env enthält einen NAMEN, nicht den Schlüssel)
export OPENAI_API_KEY="your-api-key"
# oder
export ANTHROPIC_API_KEY="your-api-key"

# Alternativ direkt in die Konfigurationsdatei (Modus 600; `config` maskiert sie)
rust-git-cli config set ai.api_key "your-api-key"
```

> `api_key_env` erwartet einen Variablennamen wie `OPENAI_API_KEY`, nicht den Schlüssel selbst. `config set` lehnt den Schlüssel dort ab, `config doctor` weist darauf hin.

### 3. Verwendung

```bash
rust-git-cli                      # Repository-Status prüfen (Standard)
rust-git-cli commit               # KI-Commit-Nachricht generieren
rust-git-cli commit --show-diff   # Diff vor Generierung anzeigen
rust-git-cli commit --dry-run     # Nur die Nachricht ausgeben, nichts stagen, nichts committen
rust-git-cli commit --debug       # Debug-Modus
rust-git-cli review               # Aktuelle Änderungen mit KI prüfen
rust-git-cli review --staged      # Nur gestagete Änderungen prüfen
rust-git-cli review --base main   # Gesamten Branch gegen main prüfen
rust-git-cli config doctor        # Konfiguration, Schlüssel und Endpunkt prüfen
```

Der Name ist lang, ein Alias hilft:

```bash
alias vgit=rust-git-cli     # in ~/.zshrc oder ~/.bashrc eintragen
# oder ohne Shell-Konfiguration: ln -sf "$(command -v rust-git-cli)" ~/.local/bin/vgit
```

## Befehle

| Befehl | Beschreibung |
|--------|--------------|
| `status` | Repository-Status prüfen (Standard) |
| `commit` | KI-Commit generieren und ausführen |
| `review` | Aktuelle Änderungen mit KI prüfen |
| `diff` | Code-Änderungen anzeigen |
| `log` | Commit-Verlauf anzeigen, unterstützt KI-Changelog und Datei-Export |
| `config` | Konfiguration anzeigen, finden, bearbeiten, ändern; `doctor` prüft selbst |
| `hook` | `prepare-commit-msg`-Hook installieren/entfernen, damit `git commit` KI nutzt |
| `init` | Konfigurationsdatei initialisieren |

### Globale Optionen

```bash
rust-git-cli [OPTIONS] [COMMAND]

Optionen:
  -p, --path <PATH>    Zu prüfender Pfad (Standard: aktuelles Verzeichnis)
  -v, --verbose        Ausführliche Ausgabe
  -h, --help           Hilfe anzeigen
  -V, --version        Version anzeigen
```

### commit Optionen

```bash
rust-git-cli commit [OPTIONS]

Optionen:
  --api-key <KEY>      API-Schlüssel temporär angeben
  --model <MODEL>      KI-Modell angeben (z.B. gpt-4, deepseek-v3)
  --base-url <URL>     Benutzerdefinierter API-Endpunkt
  --auto               Bestätigung überspringen und direkt committen
  --dry-run            Generierte Nachricht nur ausgeben, nichts stagen oder committen
  --show-diff          Diff vor Generierung anzeigen
  --debug              Rohe KI-Antwort anzeigen
```

### log Optionen

```bash
rust-git-cli log [OPTIONS]

Optionen:
  -n, --count <N>      Anzahl der anzuzeigenden Commits (Standard: 10)
  --grep <PATTERN>     Nach Inhalt filtern
  --author <NAME>      Nach Autor filtern
  --since <DATE>       Startdatum (z.B. "2024-01-01" oder "1 week ago")
  --until <DATE>       Enddatum
  --full               Vollständige Commit-Nachricht anzeigen
  --export <FILE>      Changelog in eine Datei schreiben (ohne KI-Zusammenfassung nur die Commit-Liste)
  --api-key <KEY>      API-Schlüssel temporär angeben (für Changelog-Generierung)
  --model <MODEL>      KI-Modell angeben
  --base-url <URL>     Benutzerdefinierter API-Endpunkt
  --debug              Rohe KI-Antwort anzeigen
```

### diff Optionen

```bash
rust-git-cli diff [OPTIONS]

Optionen:
  --staged             Nur gestagete Änderungen anzeigen
```

### review Optionen

```bash
rust-git-cli review [OPTIONS]

Optionen:
  --api-key <KEY>      API-Schlüssel temporär angeben
  --model <MODEL>      KI-Modell angeben
  --base-url <URL>     Benutzerdefinierter API-Endpunkt
  --staged             Nur gestagete Änderungen prüfen
  --unstaged           Nur nicht gestagete Änderungen prüfen
  --base <REF>         Gegen eine Referenz vergleichen und den gesamten Branch prüfen (auch bereits committete, nicht gemergte Änderungen)
  --debug              Rohe KI-Antwort anzeigen
```

### config Befehl

Die Konfigurationsverwaltung funktioniert auch außerhalb eines Repositories.

```bash
rust-git-cli config                  # Wirksame Werte und ihre Quelldatei anzeigen
rust-git-cli config path             # Tatsächlich gelesene Datei plus verdeckte Kandidaten
rust-git-cli config edit             # Mit $VISUAL / $EDITOR öffnen
rust-git-cli config open             # Mit der Standardanwendung öffnen
rust-git-cli config set ai.model deepseek-chat   # Einen Schlüssel ändern, Kommentare bleiben
rust-git-cli config doctor           # Prüft Parsbarkeit, Schlüsselquelle, Endpunkt, Diff-Budget
rust-git-cli config doctor --offline # Netzwerkprüfung überspringen
```

`config show` maskiert `api_key` (`--show-secrets` zeigt ihn) und prüft, ob die von `api_key_env` benannte Variable in dieser Shell existiert. `config set` validiert vor dem Schreiben: ein Schlüssel in `api_key_env`, ein falsch geschriebener Provider oder ein unbekanntes Format werden abgelehnt statt später still zu wirken.

### hook Befehl

```bash
rust-git-cli hook install     # .git/hooks/prepare-commit-msg schreiben
rust-git-cli hook status      # Aktuellen Status anzeigen
rust-git-cli hook uninstall   # Entfernen (nur die eigene Datei)
```

Nach der Installation ruft ein `git commit` ohne Nachricht `rust-git-cli commit --dry-run` auf und trägt das Ergebnis ein. Nachrichten aus `-m` / `-F` / Merge / Squash bleiben unangetastet, `VGIT_NO_HOOK=1` umgeht den Hook, und fremde Hooks werden nie angefasst.

### init Optionen

```bash
rust-git-cli init [OPTIONS]

Optionen:
  --local              Konfiguration im aktuellen Verzeichnis statt Home erstellen
  --force              Bestehende Konfiguration erzwungen überschreiben
```

## Konfiguration

Suchreihenfolge für Konfigurationsdateien:
1. `./.rust-git-cli.toml` (Projektebene)
2. `~/.config/rust-git-cli/config.toml` (Benutzerebene)
3. `~/.rust-git-cli.toml` (Benutzerebene Fallback)

### Beispielkonfiguration

```toml
[ai]
provider = "openai"                      # openai (deckt jeden kompatiblen Endpunkt ab) oder anthropic
model = "gpt-4"                          # Modellname
api_key_env = "OPENAI_API_KEY"           # Variablenname, nicht der Schlüssel selbst
# api_key = "sk-..."                     # Direkte Einstellung (nicht empfohlen, funktioniert aber)
# base_url = "https://api.deepseek.com/v1"  # Benutzerdefinierter Endpunkt
max_tokens = 2000                        # Startwert; verdoppelt sich bei abgeschnittenen Antworten

[commit]
format = "conventional"                  # conventional | plain | english
include_emoji = false                    # Emoji des Commit-Typs voranstellen
max_diff_size = 4000                     # Maximale Diff-Zeichen an die KI (Review nutzt das Doppelte)
auto_stage = false                       # Alle Änderungen automatisch stagen
```

### API-Schlüssel Priorität

1. Kommandozeilenargument `--api-key`
2. Konfigurationsdatei `api_key`
3. Umgebungsvariable (angegeben durch `api_key_env`)
4. Interaktive Eingabe

## Commit-Nachrichten-Format

Generierte Commit-Nachrichten folgen der [Conventional Commits](https://www.conventionalcommits.org/) Spezifikation:

```
feat(auth): 添加用户认证功能
Add user authentication feature

实现了JWT令牌验证
Implement JWT token validation
添加了用户登录接口
Add user login endpoint
```

### Commit-Typen

| Typ | Beschreibung |
|-----|--------------|
| `feat` | Neues Feature |
| `fix` | Fehlerbehebung |
| `docs` | Dokumentation |
| `style` | Code-Formatierung |
| `refactor` | Code-Refactoring |
| `test` | Tests |
| `chore` | Build/Tooling |
| `perf` | Performance |

### Ausgabeformate

`commit.format` steuert die Darstellung; alle drei behalten den zweisprachigen Body:

| Wert | Ausgabe |
|------|---------|
| `conventional` | `feat(auth): 添加用户认证功能` + `Add user authentication feature` |
| `plain` | `添加用户认证功能` + `Add user authentication feature` (ohne Typ-Präfix) |
| `english` | `feat(auth): Add user authentication feature` (nur Englisch) |

`commit.include_emoji = true` stellt dem Betreff ein Emoji voran: `feat` ✨, `fix` 🐛, `docs` 📝, `style` 💄, `refactor` ♻️, `perf` ⚡️, `test` ✅, `build` 📦, `ci` 👷, `chore` 🔧, `revert` ⏪. Unbekannte Typen bekommen keins, statt jede Zeile zu dekorieren.

## Arbeitsablauf

```bash
# 1. Status prüfen
$ rust-git-cli status

# 2. Änderungen anzeigen
$ rust-git-cli diff

# 3. Commit generieren
$ rust-git-cli commit

# Wenn nicht gestagete Änderungen erkannt werden:
# Unstaged changes detected:
# ──────────────────────────────────────────────────
#   M src/main.rs
#   ? src/new_file.rs
# ──────────────────────────────────────────────────
# Do you want to stage all changes (git add .)? (Y/n)

# Nach KI-Generierung, Aktion wählen:
# - Accept and commit: Akzeptieren und committen
# - Edit message: Vor dem Commit bearbeiten
# - Regenerate: Neu generieren
# - Cancel: Abbrechen

# 4. Changelog generieren
$ rust-git-cli log -n 20

# Commits interaktiv auswählen (Leertaste zum Auswählen, Enter zum Bestätigen)
# Nach Auswahl KI-Changelog-Zusammenfassung generieren
# Unterstützt Kopieren in die Zwischenablage
```

## Fehlerbehebung

| Problem | Lösung |
|---------|--------|
| API-Verbindung fehlgeschlagen | Netzwerk prüfen, API-Schlüssel verifizieren, `--debug` für Details |
| JSON-Parsing-Fehler | `--debug` für Rohantwort, anderes Modell versuchen |
| Konfiguration funktioniert nicht | Zuerst `config doctor`; ein Parse-Fehler warnt jetzt, statt still auf Standardwerte zurückzufallen |
| „API key not set“, obwohl einer gesetzt wurde | Der Schlüssel steckt vermutlich in `api_key_env`, das einen Variablennamen erwartet. Nutze `config set ai.api_key <key>` |
| Commit fehlgeschlagen | Git-Benutzer konfiguriert sicherstellen (`git config user.name/email`) |

```bash
# Debug-Modus
rust-git-cli commit --debug

# Hilfe anzeigen
rust-git-cli --help
rust-git-cli commit --help
```

## Entwicklung

```bash
cargo build                       # Bauen
cargo test                        # Testen
cargo fmt                         # Formatieren
cargo clippy -- -D warnings       # Linten
```

### Projektstruktur

```
src/
├── main.rs          # Einstiegspunkt und Befehlsverteilung
├── cli.rs           # CLI-Definitionen (clap)
├── config.rs        # Konfigurationsverwaltung
├── git.rs           # Git-Operationen (git2)
├── ui.rs            # Interaktive Benutzeroberfläche (dialoguer)
└── ai/
    ├── mod.rs       # KI-Client-Abstraktion
    ├── openai.rs    # OpenAI-Implementierung
    └── anthropic.rs # Anthropic-Implementierung
```

## Lizenz

MIT License

## Danksagungen

- [git2-rs](https://github.com/rust-lang/git2-rs) - Git-Operationen
- [clap](https://github.com/clap-rs/clap) - Kommandozeilen-Parsing
- [dialoguer](https://github.com/console-rs/dialoguer) - Interaktive Benutzeroberfläche
- [colored](https://github.com/colored-rs/colored) - Farbige Ausgabe
