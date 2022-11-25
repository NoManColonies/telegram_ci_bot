-- Add down migration script here
DROP INDEX IF EXISTS job_started_date;
DROP INDEX IF EXISTS job_created_by;

DROP TABLE IF EXISTS main.jobs;
