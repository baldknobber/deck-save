# DeckSave

A lightweight desktop app for **backing up and syncing game save files** across Steam Deck, Windows PC, and Linux. Built with [Tauri 2](https://tauri.app/) for a small, fast, native experience.

![Rust](https://img.shields.io/badge/Rust-backend-orange)
![React](https://img.shields.io/badge/React-frontend-blue)
![Tauri](https://img.shields.io/badge/Tauri%202-framework-yellow)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- **Automatic Steam detection** — Scans your Steam library and locates save files for 52,000+ games using the [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest)
- **One-click backup & restore** — Versioned zip backups with SHA-256 integrity verification
- **File watcher** — Detects save file changes in real time and triggers auto-backup (on-change, hourly, or daily)
- **Syncthing sync** — Peer-to-peer save sync between devices via [Syncthing](https://syncthing.net/) (no accounts, no cloud)
- **Syncthing auto-install** — DeckSave downloads and manages Syncthing for you — no separate setup needed
- **Gamepad controls** — Native Gamepad API support for Steam Deck (D-pad, A/B buttons, analog stick, shoulder buttons)
- **Auto-registers in Steam** — Adds itself as a non-Steam game so it appears in Gaming Mode
- **First-run wizard** — Guided setup: gamepad check, Steam registration, Syncthing install
- **Steam Deck optimized** — Gamepad-first UI with 48px+ touch targets and 1280×800 layout
- **System tray** — Minimizes to tray on Windows; runs quietly in the background
- **Desktop notifications** — OS-native notifications when auto-backups complete
- **Lightweight** — No Electron, no bundled runtime. Uses the system webview via Tauri

## Gamepad Controls

| Button | Action |
|--------|--------|
| D-pad / Left Stick | Navigate between items |
| A | Confirm / activate |
| B | Back / close modal |
| L1 | Previous tab |
| R1 | Next tab |
| L2 | Previous sub-tab |
| R2 | Next sub-tab |

Button hints appear automatically at the bottom of the screen when a gamepad is detected, and hide when you switch to mouse or keyboard.

## How It Works

```
┌─────────────────────────────────────────┐
│  React + TypeScript + Tailwind (UI)     │
├─────────────────────────────────────────┤
│  Tauri 2.0 (IPC bridge)                 │
├─────────────────────────────────────────┤
│  Rust backend                           │
│  ├── steamlocate (Steam library scan)   │
│  ├── Ludusavi manifest (save path DB)   │
│  ├── Backup engine (zip + SHA-256)      │
│  ├── File watcher (notify crate)        │
│  ├── Syncthing REST API + auto-install  │
│  ├── Steam shortcut registration        │
│  └── SQLite (history, settings)         │
└─────────────────────────────────────────┘
```

1. **Scan** — Detects installed Steam games and resolves their save file locations using Ludusavi's community-maintained manifest
2. **Backup** — Creates timestamped, compressed, checksummed zip archives with configurable retention
3. **Watch** — Monitors save directories for changes and auto-backs up based on your schedule
4. **Sync** — Shares backups between devices via Syncthing's peer-to-peer protocol

## Installation

### Windows
Download the `.msi` or `.exe` installer from [Releases](https://github.com/baldknobber/deck-save/releases).

### Steam Deck / Linux (Flatpak — recommended)
One-liner install (bundles all dependencies, works on SteamOS):
```bash
curl -sL https://raw.githubusercontent.com/baldknobber/deck-save/master/scripts/deck-install-flatpak.sh | bash
```
Then run: `flatpak run com.baldknobber.decksave`

On first launch, the setup wizard will offer to register DeckSave in Steam (so it appears in Gaming Mode) and install Syncthing.

### Linux (AppImage — requires webkit2gtk-4.1)
```bash
curl -sL https://raw.githubusercontent.com/baldknobber/deck-save/master/scripts/deck-install.sh | bash
```
> **Note:** The AppImage requires `webkit2gtk-4.1` installed on your system. SteamOS does not ship this — use the Flatpak instead.

## Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (18+)
- [Tauri CLI](https://tauri.app/start/): `cargo install tauri-cli`
- **Linux only:** `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`

### Build
```bash
npm install
cargo tauri build
```

### Development
```bash
npm install
cargo tauri dev
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | [Tauri 2](https://tauri.app/) |
| Frontend | [React 18](https://react.dev/) + [TypeScript](https://typescriptlang.org/) + [Tailwind CSS](https://tailwindcss.com/) |
| Build | [Vite](https://vitejs.dev/) |
| Backend | Rust |
| Database | [SQLite](https://sqlite.org/) via [rusqlite](https://github.com/rusqlite/rusqlite) |
| Steam detection | [steamlocate](https://github.com/WilliamVenner/steamlocate) |
| Save path database | [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest) |
| File watching | [notify](https://github.com/notify-rs/notify) |
| Sync | [Syncthing](https://syncthing.net/) REST API |
| Steam shortcuts | [steam_shortcuts_util](https://github.com/PhilipK/steam_shortcuts_util) |
| Compression | [zip](https://github.com/zip-rs/zip2) (deflate) |
| Integrity | [sha2](https://github.com/RustCrypto/hashes) (SHA-256) |

## Roadmap

- [x] **Flatpak packaging** — Bundled for Steam Deck / SteamOS
- [x] **Gamepad controls** — Native Gamepad API (D-pad, A/B, L1/R1 tab switching)
- [x] **Steam shortcut registration** — Auto-add to Steam for Gaming Mode
- [x] **Syncthing auto-install** — Download and manage Syncthing automatically
- [x] **First-run wizard** — Guided setup on first launch
- [ ] **Auto-updater** — In-app update checks via Tauri's updater plugin
- [ ] **Non-Steam game support** — Manual path entry for GOG, Epic, emulators, etc.
- [ ] **Cloud backup option** — Optional upload to a cloud provider as a secondary backup target
- [ ] **Per-game sync rules** — Choose which games sync and which stay local
- [ ] **UI themes** — Light mode, OLED-optimized dark theme
- [ ] **Backup browser** — View and diff individual files inside backup zips
- [ ] **Import/export settings** — Transfer DeckSave config between machines

## Credits & Acknowledgments

- **[Ludusavi](https://github.com/mtkennerly/ludusavi)** by Matthew T. Kennerly — DeckSave uses the [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest), a community-maintained database of game save locations covering 52,000+ games.
- **[Syncthing](https://syncthing.net/)** — Open-source continuous file synchronization. DeckSave uses Syncthing's REST API for peer-to-peer save sync.
- **[Tauri](https://tauri.app/)** — The framework that makes it possible to build a small, fast, native desktop app with a web frontend.
- **[steamlocate](https://github.com/WilliamVenner/steamlocate)** by William Venner — Rust crate for locating Steam installations and libraries.
- **[steam_shortcuts_util](https://github.com/PhilipK/steam_shortcuts_util)** by PhilipK — Rust crate for reading and writing Steam's shortcuts.vdf format (also used by [BoilR](https://github.com/PhilipK/BoilR)).
- **[PCGamingWiki](https://www.pcgamingwiki.com/)** — The Ludusavi manifest is largely sourced from PCGamingWiki's save game location data.

## License

MIT
