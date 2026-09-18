CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    taken_at INTEGER NOT NULL,
    last_sync_oid TEXT NOT NULL,
    plat INTEGER NOT NULL,
    credits INTEGER NOT NULL,
    endo INTEGER NOT NULL,
    ducats INTEGER NOT NULL,
    aya INTEGER,
    mr INTEGER NOT NULL
);

CREATE INDEX snapshots_taken_at ON snapshots (taken_at);

CREATE TABLE deltas (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_snapshot INTEGER NOT NULL REFERENCES snapshots (id),
    to_snapshot INTEGER NOT NULL REFERENCES snapshots (id),
    item_type TEXT NOT NULL,
    category TEXT NOT NULL,
    delta INTEGER NOT NULL
);

CREATE INDEX deltas_to_snapshot ON deltas (to_snapshot);

CREATE TABLE trades (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at INTEGER NOT NULL,
    partner TEXT,
    items_json TEXT NOT NULL,
    plat INTEGER NOT NULL
);

CREATE INDEX trades_at ON trades (at);

CREATE TABLE relic_openings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    at INTEGER NOT NULL,
    relic TEXT NOT NULL,
    reward_item TEXT NOT NULL,
    player_count INTEGER NOT NULL
);

CREATE INDEX relic_openings_at ON relic_openings (at);

CREATE TABLE favourites (
    unique_name TEXT PRIMARY KEY,
    since INTEGER NOT NULL
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
