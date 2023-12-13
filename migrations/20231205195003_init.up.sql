-- Add up migration script here
CREATE TABLE IF NOT EXISTS txdata (
    txhash VARCHAR(64) PRIMARY KEY NOT NULL,
    height BIGINT NOT NULL,
    source VARCHAR(64) NOT NULL,
    destination VARCHAR(64) NOT NULL,
    token VARCHAR(64) NOT NULL,
    amount VARCHAR(32) NOT NULL,
    timestamp VARCHAR(48) NOT NULL
);

-- Create configuration table
CREATE TABLE IF NOT EXISTS configuration (
    key TEXT PRIMARY KEY,
    value BIGINT
);

-- Insert initial value for highest_completed_height if it doesn't exist
INSERT INTO configuration (key, value) VALUES ('highest_indexed', 0)
ON CONFLICT (key) DO NOTHING;