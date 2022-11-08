-- Add up migration script here
CREATE TABLE IF NOT EXISTS main.repos (
  id TEXT PRIMARY KEY,
  name TEXT UNIQUE NOT NULL,
  status TEXT CHECK (status IN ('IDLE', 'DEPLOY', 'FAILURE', 'SUCCESS')) NOT NULL DEFAULT 'IDLE',
  message_id INTEGER NOT NULL
) WITHOUT ROWID;
