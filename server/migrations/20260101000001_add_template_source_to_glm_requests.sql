ALTER TABLE glm_requests
    ADD COLUMN IF NOT EXISTS template_source TEXT NOT NULL DEFAULT 'llm';
