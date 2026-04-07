-- DeckSave initial schema

CREATE TABLE IF NOT EXISTS games (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    title       TEXT    NOT NULL,
    steam_id    TEXT,
    install_dir TEXT,
    save_paths  TEXT    NOT NULL DEFAULT '[]',
    status      TEXT    NOT NULL DEFAULT 'never_backed_up',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS backups (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id     INTEGER NOT NULL REFERENCES games(id),
    timestamp   TEXT    NOT NULL DEFAULT (datetime('now')),
    file_path   TEXT    NOT NULL,
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    checksum    TEXT,
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_devices (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    name                TEXT    NOT NULL,
    syncthing_device_id TEXT    UNIQUE,
    last_seen           TEXT,
    created_at          TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Default settings
INSERT OR IGNORE INTO settings (key, value) VALUES ('backup_dir', '');
INSERT OR IGNORE INTO settings (key, value) VALUES ('auto_backup', 'true');

-- Ensure no duplicate Steam games on re-scan
CREATE UNIQUE INDEX IF NOT EXISTS idx_games_steam_id ON games(steam_id) WHERE steam_id IS NOT NULL;
INSERT OR IGNORE INTO settings (key, value) VALUES ('backup_interval', 'hourly');
INSERT OR IGNORE INTO settings (key, value) VALUES ('max_versions', '5');
