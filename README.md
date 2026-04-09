<div align="center">

# Claude Code Installer for Windows

### One command to install Claude Code on Windows.
### Auto-repairs, auto-launches, and never asks for admin.

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2B%20%7C%20Windows%2011-0078d4)](https://www.microsoft.com/windows)
[![PowerShell](https://img.shields.io/badge/PowerShell-5.1%2B%20%7C%207%2B-5391FE?logo=powershell&logoColor=white)](https://learn.microsoft.com/en-us/powershell/)
[![Tauri](https://img.shields.io/badge/GUI-Tauri%20v2-FFC131?logo=tauri&logoColor=white)](./claude-installer-gui)
[![No Admin](https://img.shields.io/badge/admin%20required-no-brightgreen)](#requirements)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-blueviolet.svg)](https://github.com/wasintoh/claudecodeinstaller/pulls)

**Two flavors, same goal:** a [PowerShell one-liner](./claude-installer-ps) for instructors, and a [double-click GUI](./claude-installer-gui) for students who can't open a terminal.

</div>

---

```powershell
irm https://raw.githubusercontent.com/wasintoh/claudecodeinstaller/main/claude-installer-ps/install-claude-code.ps1 | iex
```

**That's it.** Copy → paste into PowerShell → press Enter. When it's done, Claude Code opens in a new terminal window, ready to use.

---

## Why this exists

After running multiple Claude Code bootcamps for **Thai non-developers** — CEOs, marketers, business owners, solopreneurs — the **#1 reason students drop out on Day 1** is the Windows installation process.

Real things that happened in class:

- Students had never opened PowerShell in their life.
- "I installed Git but `git` still doesn't work" (PATH wasn't checked during install).
- Node.js 14 from 2019 still running, silently breaking every modern npm package.
- "`claude: command not found`" — install succeeded but PATH wasn't refreshed.
- Cryptic `Claude Code on Windows requires git-bash` error with no clear fix.
- Students installed everything, then didn't know they had to open a **new** terminal.

**This installer solves all of it.** One command handles everything — install, repair, and launch.

---

## What it does

| | |
|---|---|
| **Installs Git for Windows** | Per-user install (`/CURRENTUSER`), no admin required |
| **Installs Node.js LTS** | Latest version for `npx`, `create-next-app`, MCP servers |
| **Installs Claude Code** | Uses official Anthropic installer (`claude.ai/install.ps1`) |
| **Configures npm global** | Sets prefix to `~/.npm-global` and adds it to PATH |
| **Auto-fixes PATH** | The #1 source of "it doesn't work" — handled for every component |
| **Tests after install** | Runs `claude --version` and classifies any failure |
| **Auto-repairs 5 failure modes** | PATH missing, git-bash missing, SmartScreen block, and more |
| **Auto-launches Claude Code** | Opens a new PowerShell window with `claude` running — no manual step |
| **Includes uninstaller** | Tracks what it installed, preserves pre-existing tools |
| **Dry-run mode** | `-DryRun` shows what would happen without installing |

---

## Install

There are **two ways** to install, depending on the user's comfort level.

### Option A — PowerShell one-liner (recommended for anyone comfortable copy-pasting)

1. Open PowerShell (press `Win` + `X`, then click **Windows PowerShell** or **Terminal**)
2. Paste this command and press Enter:

```powershell
irm https://raw.githubusercontent.com/wasintoh/claudecodeinstaller/main/claude-installer-ps/install-claude-code.ps1 | iex
```

3. Wait ~2 minutes. A new terminal window will open with Claude Code running when it's done.

### Option B — GUI installer (build it yourself)

A double-click `.exe` for absolute beginners, built with [Tauri v2](https://v2.tauri.app/) (Rust + React). Supports **Thai and English**, ships as an NSIS installer, and includes the same post-install test + auto-repair + auto-launch flow as the PowerShell version.

> **Heads up:** no pre-built `.exe` is shipped in this repo. The GUI source is here for instructors and contributors who want to **compile and distribute their own signed binary**. If you're a student, use **Option A** (the one-liner) — it's easier.

#### Why no pre-built .exe?

- **Code signing** — unsigned `.exe` files trigger Windows SmartScreen warnings that scare non-technical users more than the PowerShell command does.
- **Classroom fit** — instructors typically want to customize branding, language, or which components to install. Compiling lets you own that.
- **Trust** — forcing a rebuild from source is more transparent than shipping an opaque binary.

#### Compile on Windows (~10 minutes)

**Prerequisites** (install once):

1. **Node.js 20+** — https://nodejs.org
2. **Rust** (stable) — https://rustup.rs (pick default options)
3. **Microsoft Visual Studio Build Tools** — https://visualstudio.microsoft.com/visual-cpp-build-tools/
   - During install, check **"Desktop development with C++"**
4. **WebView2 Runtime** — pre-installed on Windows 11; [download for Windows 10](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

**Build steps**:

```powershell
# 1. Clone the repo
git clone https://github.com/wasintoh/claudecodeinstaller.git
cd claudecodeinstaller/claude-installer-gui

# 2. Install frontend dependencies (~2 min)
npm install

# 3. Build the installer (~5–8 min first time, cached after)
npm run tauri build
```

**Your `.exe` will appear at**:

```
claude-installer-gui/src-tauri/target/release/bundle/
├── nsis/Claude Code Installer_1.0.0_x64-setup.exe      ← recommended (smaller, modern)
└── msi/Claude Code Installer_1.0.0_x64_en-US.msi       ← enterprise/GPO-friendly
```

Double-click either one to install, or distribute to your students via Google Drive, LINE, or USB.

#### Optional: code signing

Unsigned binaries show a SmartScreen warning ("Windows protected your PC"). To remove it:

1. Get a code signing certificate (DigiCert, Sectigo, or free via [SignPath.io](https://signpath.io) for open source projects)
2. Update `claude-installer-gui/src-tauri/tauri.conf.json`:
   ```json
   "windows": {
     "certificateThumbprint": "YOUR_THUMBPRINT",
     "digestAlgorithm": "sha256",
     "timestampUrl": "http://timestamp.digicert.com"
   }
   ```
3. Rebuild — the `.exe` will now be signed and trusted.

#### Optional: automated CI builds

This repo includes a [GitHub Actions workflow](.github/workflows/build-gui.yml) that auto-builds the GUI on Windows runners. Push to `main` or create a `v*` tag to trigger a build — the `.exe` will be uploaded as a workflow artifact (or attached to a Release for tags). Use this if you don't want to maintain a Windows build machine.

### Advanced install (saved PowerShell file)

Download the script first, then run it with flags:

```powershell
# Download
Invoke-WebRequest `
  -Uri https://raw.githubusercontent.com/wasintoh/claudecodeinstaller/main/claude-installer-ps/install-claude-code.ps1 `
  -OutFile install-claude-code.ps1

# See what would happen without installing (dry-run)
.\install-claude-code.ps1 -DryRun

# Skip Node.js (if you manage it with nvm, fnm, volta, etc.)
.\install-claude-code.ps1 -SkipNode

# Uninstall
.\install-claude-code.ps1 -Uninstall
```

---

## How it works

The installer runs **6 phases**, each with clear progress indicators:

```
  [1/6] Running pre-flight checks...         Windows version, internet, RAM, disk
  [2/6] Checking Git for Windows...          Install via GitHub Releases API
  [3/6] Checking Node.js...                  Install latest LTS from nodejs.org
  [4/6] Checking Claude Code...              Official Anthropic installer
  [5/6] Verifying Claude Code works...       Run `claude --version` + auto-repair
  [6/6] Launching Claude Code...             Open new window ready to use
```

Each phase **detects existing installations** and skips them. Only missing components are installed. Your existing setup is respected.

---

## Auto-repair engine

After installing, the script runs `claude --version` to verify everything actually works. If it fails, the error is diagnosed and repaired automatically — up to 3 attempts per failure mode.

| Error detected | Auto-fix applied |
|---|---|
| `claude` not recognized | Adds `~/.local/bin` to User PATH, refreshes session |
| `requires git-bash` | Locates `bash.exe`, sets `CLAUDE_CODE_GIT_BASH_PATH` env var |
| SmartScreen blocked binary | Runs `Unblock-File` on all Claude Code binaries + share folder |
| Binary missing entirely | Re-runs the Anthropic bootstrap installer |
| Unknown / opaque error | Logs full diagnostics + shows manual fix instructions |

If every repair fails, you get a clear error message with the log file path for debugging.

---

## Every pain point, solved

Built from real classroom feedback. Every one of these is handled:

| # | Pain point | Fix |
|---|---|---|
| 1 | Never opened a terminal before | Banner, colored output, step counters, friendly messages |
| 2 | Git not installed | Auto-install from GitHub Releases |
| 3 | Git installed but not in PATH | Manual PATH fallback to both per-user and global Git dirs |
| 4 | Node.js not installed | Auto-install latest LTS from nodejs.org |
| 5 | Node.js outdated (< 18) | Version check + upgrade prompt |
| 6 | npm global path misconfigured | Auto-configure `~/.npm-global` prefix and add to PATH |
| 7 | PATH not refreshed after install | `Refresh-PathFromRegistry` after every component |
| 8 | PowerShell Execution Policy restricted | Pre-flight check with clear fix instructions |
| 9 | Admin privileges needed but unknown how | Per-user install — no admin required |
| 10 | Slow or interrupted internet | Retry with exponential backoff (1s, 3s, 9s) |
| 11 | `claude` not found after install | Auto-add `~/.local/bin` to PATH |
| 12 | PowerShell vs CMD confusion | Shell detection in pre-flight check |
| 13 | User doesn't know how to open Claude Code | **Auto-launches in a new terminal window** |

---

## Uninstall

Save the script and run:

```powershell
.\install-claude-code.ps1 -Uninstall
```

You'll get a menu:

```
  ============================================================
         Claude Code Uninstaller
  ============================================================

  What would you like to uninstall?

  [1] Claude Code only                  (recommended)
  [2] Claude Code + Node.js
  [3] Claude Code + Node.js + Git
  [4] Everything this installer installed
  [0] Cancel
```

The uninstaller is **careful by design**:

- Reads `~/.claude-installer/manifest.json` to distinguish what it installed vs. what was already there
- **Never removes pre-existing tools** without explicit confirmation
- Asks before removing config files (`~/.claude/`, `~/.claude.json`) — defaults to **keep**
- Warns specifically before removing Git, which may be used by VS Code / GitHub Desktop / SourceTree / etc.

---

## Requirements

- **Windows 10** (build 17763 / version 1809) or **Windows 11**
- **PowerShell 5.1+** (built-in) or **PowerShell 7+**
- **4 GB RAM** minimum
- **2 GB** free disk space
- Internet connection
- **No administrator privileges required**

Supports **x64** and **ARM64** architectures.

---

## Diagnostics

Every run writes a detailed log to:

```
%TEMP%\claude-code-install.log
```

Instructors can ask students to share this log to diagnose issues remotely. The log contains every step, every download, every PATH change, and every error with full stack traces.

---

## FAQ

**Q: Is this safe? It downloads and runs a script from the internet.**
The script is open source — read it before running. It follows the same pattern as Anthropic's official installer (`irm https://claude.ai/install.ps1 | iex`). Every download uses HTTPS with signature verification where available.

**Q: Do I need to run PowerShell as Administrator?**
No. The entire flow uses per-user installation. No UAC prompts. Node.js is the only component that may request elevation, and the script provides a clear fallback with manual instructions if that happens.

**Q: Will this break my existing Git or Node.js?**
No. If you already have Git or Node.js (>= 18), the installer detects them and skips installation. They're marked as `preExisting` in the manifest, so the uninstaller will warn before touching them.

**Q: What if something fails mid-install?**
Each component is independent. If Git installs but Node.js fails, Git stays installed and you get a clear error. Re-run the script to retry — successfully installed components are detected and skipped.

**Q: How do I update Claude Code later?**
Just run `claude`. Claude Code updates itself automatically.

**Q: Can I use this behind a corporate proxy?**
If your proxy is configured in Windows Settings, `Invoke-WebRequest` uses it automatically. If not, set `$env:HTTPS_PROXY` before running.

**Q: Can I run this via WinGet / Chocolatey / Scoop?**
Not yet. PRs welcome.

---

## Project structure

```
claudecodeinstaller/
├── claude-installer-ps/          # PowerShell installer (shipped, stable)
│   └── install-claude-code.ps1   # Single-file, ~1,800 lines, dual-mode
│
├── claude-installer-gui/         # Tauri v2 desktop installer (Rust + React)
│   ├── src/                      # React frontend (5 screens, TH/EN i18n)
│   ├── src-tauri/                # Rust backend (system check, installers)
│   └── README.md                 # Build & distribution guide
│
├── README.md                     # You are here
└── LICENSE
```

Both installers share the same philosophy: **detect existing tools, install only what's missing, auto-repair PATH issues, and verify the final result.** The GUI version wraps the same logic in a click-through interface for users who can't open PowerShell at all.

---

## Contributing

Found a bug? Have a pain point we missed? PRs and issues welcome.

**Especially valuable contributions:**

- Edge cases from real Windows installations (antivirus, corporate policies, non-English locales)
- Proxy / offline / air-gapped environments
- Additional auto-repair rules for error modes we haven't seen
- Tests (Pester)
- Translations

---

## License

MIT — see [LICENSE](LICENSE).

---

## Credits

Created by **Wasin Treesinthuros** (จารโต) to eliminate the Windows installation friction that made students drop out on Day 1 of Claude Code bootcamps.

I originally built this for my own classroom — but anyone teaching or learning Claude Code on Windows is welcome to use, fork, or adapt it. If it saved your class a day of debugging, please **star the repo** so other instructors can find it.

<div align="center">

**Made by จารโต — for everyone who ships with Claude Code.**

</div>
