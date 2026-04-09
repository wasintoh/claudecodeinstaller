# Claude Code Installer (GUI)

A desktop GUI installer for Claude Code built with Tauri v2 (Rust + React).
Designed for non-technical Windows users who need a one-click install experience.

## What it installs

- **Git for Windows** — required by Claude Code
- **Node.js LTS** — required for npx, MCP Servers, and dev tools
- **Claude Code** — AI Coding Assistant by Anthropic

## Prerequisites

- [Node.js](https://nodejs.org/) v18+ (for building the frontend)
- [Rust](https://rustup.rs/) (stable toolchain)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/) v2

```bash
# Install Tauri CLI
cargo install tauri-cli --version "^2.0"
```

## Development

```bash
# Install frontend dependencies
npm install

# Run in development mode (opens the window with hot reload)
npm run tauri dev
```

## Building for Distribution

```bash
# Build the production installer
npm run tauri build
```

Output files will be in `src-tauri/target/release/bundle/`:
- `nsis/Claude Code Installer_1.0.0_x64-setup.exe` — NSIS installer
- `msi/Claude Code Installer_1.0.0_x64_en-US.msi` — MSI installer

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
