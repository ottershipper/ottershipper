-- Create applications table
CREATE TABLE IF NOT EXISTS applications (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL
);

-- Index for faster name lookups
CREATE INDEX IF NOT EXISTS idx_applications_name ON applications(name);
