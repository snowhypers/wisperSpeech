# WhistleFlow — Local Voice-to-Text Desktop App (Tauri v2 + Cactus Whistle 16.9 MB)

WhistleFlow is an offline, Wispr-Flow-style voice dictation desktop app built with **Tauri v2**, **Rust**, and **React**. It runs **Cactus Compute's Whistle** (`whistle.cact`, 16.9 MB) locally on your CPU—no cloud API keys, no GPU required.

## Features
- **Global Hotkey (`Option + Space` on macOS / `Alt + Space` on Windows & Linux):** Press anywhere to start/stop dictation (supports both Toggle and Hold-to-Talk modes).
- **Auto-Paste into Any Input Field:** Transcribes 16 kHz mono audio locally and pastes directly at your active cursor (`Cmd+V` / `Ctrl+V`), then restores your previous clipboard.
- **Floating Glassmorphic Pill Overlay:** Appears at the bottom-center of your screen with a live microphone waveform while listening.
- **1-Click Model Setup:** Built-in downloader fetches `whistle.cact` (16.9 MB) and the native `cactus` CPU runner from Hugging Face (`Cactus-Compute/whistle` and `Cactus-Compute/cactus`).
- **7 Languages:** Auto-detect or force English, German, French, Spanish, Italian, Dutch, or Polish.

## Quick Start

### Prerequisites
- **Node.js 20+**
- **Rust (stable)** (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Tauri v2 OS Prerequisites:** https://v2.tauri.app/start/prerequisites/
  - On **Linux**, install `libasound2-dev` and `libxdo-dev` in addition to WebKitGTK.

### Run in Development
```bash
npm install
npm run tauri dev
```

### Build Native Installer (`.dmg`, `.exe`/`.msi`, `.AppImage`/`.deb`)
```bash
npm run tauri build
```

## macOS Permissions
When running on macOS for the first time, grant:
1. **Microphone** access (to capture 16 kHz voice audio).
2. **Accessibility** access in *System Settings → Privacy & Security → Accessibility* (required to simulate `Cmd+V` into other apps' input fields).
