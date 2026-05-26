CREATE TABLE IF NOT EXISTS snapshots (
  tick       INTEGER PRIMARY KEY,
  bin        BLOB NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
  tick       INTEGER NOT NULL,
  seq        INTEGER NOT NULL,
  event_json TEXT NOT NULL,
  PRIMARY KEY (tick, seq)
);

CREATE TABLE IF NOT EXISTS agents_meta (
  agent_id    TEXT PRIMARY KEY,
  name        TEXT UNIQUE NOT NULL,
  token_hash  TEXT NOT NULL,
  joined_at   INTEGER NOT NULL,
  total_lives INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_events_tick ON events(tick);
