-- Add up migration script here
CREATE TABLE IF NOT EXISTS main.repos (
  id TEXT PRIMARY KEY,
  name TEXT UNIQUE NOT NULL,
  message_id INTEGER NOT NULL
) WITHOUT ROWID;
