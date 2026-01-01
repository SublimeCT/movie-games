ALTER TABLE glm_requests
    ADD COLUMN IF NOT EXISTS template_source TEXT NOT NULL DEFAULT 'llm';

CREATE TABLE IF NOT EXISTS shared_records (
    id UUID PRIMARY KEY,
    request_id UUID NOT NULL UNIQUE REFERENCES glm_requests(id),
    shared_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    shared_ip TEXT NOT NULL,
    shared_user_agent TEXT
);

CREATE INDEX IF NOT EXISTS idx_shared_records_request_id ON shared_records(request_id);
