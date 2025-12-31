ALTER TABLE glm_requests ADD COLUMN IF NOT EXISTS shared BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE glm_requests ADD COLUMN IF NOT EXISTS processed_response JSONB;

CREATE TABLE IF NOT EXISTS records (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL REFERENCES glm_requests(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    client_ip TEXT NOT NULL,
    user_agent TEXT,
    referer TEXT
);

CREATE INDEX IF NOT EXISTS idx_records_request_id ON records(request_id);
