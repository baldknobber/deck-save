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
- **Steam Deck optimized** — Gamepad-first UI with 48px+ touch targets, D-pad navigation, and 1280×800 layout
- **System tray** — Minimizes to tray on Windows; runs quietly in the background
- **Desktop notifications** — OS-native notifications when auto-backups complete
- **Lightweight** — No Electron, no bundled runtime. Uses the system webview via Tauri

## How It Works

```
┌─────────────────────────────────────────┐
│  React + TypeScript + Tailwind (UI)     │
├─────────────────────────────────────────┤
│  Tauri 2.0 (IPC bridge)                │
├─────────────────────────────────────────┤
│  Rust backend                           │
│  ├── steamlocate (Steam library scan)   │
│  ├── Ludusavi manifest (save path DB)   │
│  ├── Backup engine (zip + SHA-256)      │
│  ├── File watcher (notify crate)        │
│  ├── Syncthing REST API client          │
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

### Steam Deck / Linux
Download the `.AppImage` from [Releases](https://github.com/baldknobber/deck-save/releases), then:
```bash
chmod +x DeckSave_*.AppImage
./DeckSave_*.AppImage
```

Or use the included setup script for Steam Deck:
```bash
bash scripts/deck-test-setup.sh
```

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
| Compression | [zip](https://github.com/zip-rs/zip2) (deflate) |
| Integrity | [sha2](https://github.com/RustCrypto/hashes) (SHA-256) |

## Testing on Steam Deck

1. **Build the AppImage** — Push a version tag to trigger the release workflow:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
   Or build locally on a Linux machine: `cargo tauri build`

2. **Transfer to Deck** — Copy the `.AppImage` to your Steam Deck (via USB, SSH, or Syncthing)

3. **Run the setup script** — In Desktop Mode, open a terminal:
   ```bash
   bash scripts/deck-test-setup.sh
   ```
   This makes the AppImage executable and installs a `.desktop` shortcut.

4. **Launch** — Open DeckSave from the application menu or run the AppImage directly

5. **Verify**
   - Dashboard should detect your installed Steam games
   - Tap a game and create a backup — check that the zip appears in `~/.local/share/com.decksave.app/backups/`
   - Enable the file watcher and confirm it triggers on save file changes
   - (Optional) Set up Syncthing on both devices and test sync via the Sync Wizard

> **Note:** Syncthing must be installed separately on the Deck (`flatpak install flathub me.kozec.syncthingtk` or via `pacman`).

## Roadmap

DeckSave is currently at **v0.1.0 (MVP)**. Planned improvements:

- [ ] **Auto-updater** — In-app update checks via Tauri's updater plugin
- [ ] **Non-Steam game support** — Manual path entry for GOG, Epic, emulators, etc.
- [ ] **Cloud backup option** — Optional upload to a cloud provider (S3, Backblaze B2) as a secondary backup target
- [ ] **Per-game sync rules** — Choose which games sync and which stay local
- [ ] **UI themes** — Light mode, OLED-optimized dark theme
- [ ] **Backup browser** — View and diff individual files inside backup zips
- [ ] **Import/export settings** — Transfer DeckSave config between machines
- [ ] **Flatpak packaging** — Publish to Flathub for easier Deck installation

## Credits & Acknowledgments

This project builds on the work of several open-source projects:

- **[Ludusavi](https://github.com/mtkennerly/ludusavi)** by Matthew T. Kennerly — DeckSave uses the [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest), a community-maintained database of game save locations covering 52,000+ games. This project would not be practical without it.
- **[Syncthing](https://syncthing.net/)** — Open-source continuous file synchronization. DeckSave uses Syncthing's REST API for peer-to-peer save sync between devices.
- **[Tauri](https://tauri.app/)** — The framework that makes it possible to build a small, fast, native desktop app with a web frontend.
- **[steamlocate](https://github.com/WilliamVenner/steamlocate)** by William Venner — Rust crate for locating Steam installations and libraries.
- **[PCGamingWiki](https://www.pcgamingwiki.com/)** — The Ludusavi manifest that DeckSave relies on is largely sourced from PCGamingWiki's save game location data.

## License

MIT
