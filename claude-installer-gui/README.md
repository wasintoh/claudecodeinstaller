# Claude Code Installer — GUI (Tauri v2)

A desktop GUI installer for Claude Code built with Tauri v2 (Rust + React).
Feature-equivalent to the [PowerShell installer](../claude-installer-ps/install-claude-code.ps1), including the post-install test + auto-repair + auto-launch flow.

> **Important:** no pre-built `.exe` ships with this repo. You must compile it yourself on a Windows machine. See the [main README](../README.md#option-b--gui-installer-build-it-yourself) for the quick start.

## What it installs

- **Git for Windows** — required by Claude Code
- **Node.js LTS** — required for npx, MCP Servers, and dev tools
- **Claude Code** — AI Coding Assistant by Anthropic

After install, the GUI automatically runs `claude --version`, auto-repairs common issues (PATH, Git Bash missing, SmartScreen blocks), and opens a new PowerShell window with Claude Code ready to use.

## Prerequisites (Windows)

1. **Node.js 20+** — https://nodejs.org
2. **Rust** (stable) — https://rustup.rs
3. **Microsoft Visual Studio Build Tools** — https://visualstudio.microsoft.com/visual-cpp-build-tools/ (check "Desktop development with C++")
4. **WebView2 Runtime** — pre-installed on Windows 11, [download for Windows 10](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

No separate Tauri CLI install is needed — it's pulled in via `@tauri-apps/cli` in `package.json`.

## Development

```powershell
# Install frontend dependencies
npm install

# Run in development mode (opens the window with hot reload)
npm run tauri dev
```

## Building for Distribution

```powershell
# Build the production installer
npm run tauri build
```

First build takes ~5–8 minutes (Rust compiles a lot). Subsequent builds are cached and take ~1 minute.

**Output files** in `src-tauri/target/release/bundle/`:

- `nsis/Claude Code Installer_1.0.0_x64-setup.exe` — **recommended** NSIS installer (smaller, modern)
- `msi/Claude Code Installer_1.0.0_x64_en-US.msi` — MSI installer (enterprise / GPO-friendly)

## Code Signing

> **Important**: Before distributing the `.exe` or `.msi`, the binary should be
> code-signed with a valid certificate to avoid Windows SmartScreen warnings.
> Update `src-tauri/tauri.conf.json` with your certificate thumbprint:
>
> ```json
> "windows": {
>   "certificateThumbprint": "YOUR_CERT_THUMBPRINT",
>   "digestAlgorithm": "sha256",
>   "timestampUrl": "http://timestamp.digicert.com"
> }
> ```

## Project Structure

```
claude-installer-gui/
├── src-tauri/               # Rust backend
│   ├── src/
│   │   ├── main.rs          # Entry point, command registration
│   │   ├── lib.rs           # Library entry for Tauri v2
│   │   ├── commands/        # Tauri commands (system check, installers, uninstaller)
│   │   └── utils/           # Download, process, logging utilities
│   ├── Cargo.toml
│   └── tauri.conf.json      # Tauri window & build configuration
├── src/                     # React frontend
│   ├── screens/             # 5 main screens (Welcome, SystemCheck, Installation, Completion, Uninstall)
│   ├── components/          # Reusable UI components
│   ├── hooks/               # useInstaller (state machine), useI18n (translations)
│   ├── i18n/                # Thai and English translation files
│   └── styles/              # Tailwind CSS globals
├── package.json
└── README.md
```

## Uninstall Mode

Run the installer with `--uninstall` flag to go directly to the uninstall screen:

```bash
"Claude Code Installer.exe" --uninstall
```

## Language Support

The UI supports English and Thai. Toggle the language using the flag button in the top-right corner of the Welcome screen.
