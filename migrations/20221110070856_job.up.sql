-- Add up migration script here
CREATE TABLE IF NOT EXISTS main.jobs (
  id INTEGER PRIMARY KEY,
  status TEXT CHECK (status IN ('CANCELLED', 'RUNNING', 'FAILURE', 'SUCCESS')) NOT NULL DEFAULT 'RUNNING',
  triggered_by TEXT,
  description TEXT,
  callback_url TEXT,
  repo_id TEXT NOT NULL,
  started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
  elapsed INTEGER,
  FOREIGN KEY (repo_id) 
   REFERENCES repos (id) 
      ON DELETE CASCADE 
      ON UPDATE NO ACTION
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS job_started_date ON jobs (repo_id, started_at);
CREATE INDEX IF NOT EXISTS job_created_by ON jobs (repo_id, triggered_by);
