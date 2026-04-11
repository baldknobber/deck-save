# Changelog

## [0.3.1](https://github.com/baldknobber/deck-save/compare/deck-save-v0.3.0...deck-save-v0.3.1) (2026-04-08)


### Features

* adaptive gamepad UX, hint bar, navigation fixes, Syncthing & shortcut fixes ([9684821](https://github.com/baldknobber/deck-save/commit/9684821abb72543015bb83905d8d68af7ab3de0e))
* Flatpak packaging for Steam Deck + white screen diagnostics ([85fda82](https://github.com/baldknobber/deck-save/commit/85fda82ea6fcc986e0756d544d3bfa6dea9b77d0))
* gamepad controls, Steam shortcut registration, Syncthing auto-install, first-run wizard ([d6046fb](https://github.com/baldknobber/deck-save/commit/d6046fb4533ecf2c7ced38a3eb15a7dab4c7a0b9))
* v0.3.0 — Steam Deck UX polish ([836f87f](https://github.com/baldknobber/deck-save/commit/836f87fc46342239d1a9cde951a7964e5dfa215a))
* v0.4.0 — non-Steam launchers, restore flow, dashboard polish, shortcut & wizard fixes ([47b2273](https://github.com/baldknobber/deck-save/commit/47b2273b273e54be8d34fdbea5227d32a00bc6ff))


### Bug Fixes

* add Flatpak icon + upgrade to Node.js 22 ([fd913a5](https://github.com/baldknobber/deck-save/commit/fd913a560a6dd73f7247e295b89f77f6852c3dd6))
* clippy warnings, winget continue-on-error ([90281cb](https://github.com/baldknobber/deck-save/commit/90281cba2d94bbf4c5739631e2d9519a1b15022f))
* CSP policy causing white screen on Linux/Steam Deck ([71c0fc1](https://github.com/baldknobber/deck-save/commit/71c0fc19745aa3e519fbd0b34b95c9241dcec1de))
* disable appstream-compose in Flatpak manifest (not in SDK 47) ([1f9bda5](https://github.com/baldknobber/deck-save/commit/1f9bda51019ed05212debfeeca642e5a3ceca2ad))
* make system tray non-fatal so app launches without libayatana ([26c711b](https://github.com/baldknobber/deck-save/commit/26c711bd56a83ffc05ebf881e89399798a072404))
* make tray-icon Windows-only to prevent libappindicator panic on Linux/Flatpak ([8a31537](https://github.com/baldknobber/deck-save/commit/8a31537f7364fb9ba11dbdc4e77c9f7768922bec))
* release-please extra-files config for Cargo.toml and tauri.conf.json ([bfb7d8d](https://github.com/baldknobber/deck-save/commit/bfb7d8d45f2182a8ffb96861a8b1cf26c2f96f7a))
* replace Web Gamepad API with native gilrs backend ([ac2828d](https://github.com/baldknobber/deck-save/commit/ac2828d72f0d95310de645e5688775b118756269))
* rewrite Flatpak CI with official flatpak-builder action ([c5754e0](https://github.com/baldknobber/deck-save/commit/c5754e040f45b1380d9ebe30bb1a7c1b2d9a027d))
* skip appstream-compose in Flatpak build (removed in SDK 47) ([472b6ee](https://github.com/baldknobber/deck-save/commit/472b6ee88e242980ae5232a47bce9f9e08191747))
* upgrade GNOME Platform runtime from EOL 46 to 47 ([d72fe0c](https://github.com/baldknobber/deck-save/commit/d72fe0cc35c401713334748be47466ab3eba37f3))
* upgrade GNOME Platform runtime from EOL 47 to 48 ([83dbbf0](https://github.com/baldknobber/deck-save/commit/83dbbf0f1047b6c576ab7f664b766fb1521bcc61))
* use gnome-46 container + install GNOME 48 runtime (no gnome-48 image exists) ([f88187c](https://github.com/baldknobber/deck-save/commit/f88187ccab020d3f5dfd41d4ede8682df5519723))
* white screen on Steam Deck (EGL display crash) ([49e70ca](https://github.com/baldknobber/deck-save/commit/49e70caa06dc799bd1791ef2958a073620e69d71))
