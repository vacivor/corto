-- Schema for corto

CREATE TABLE IF NOT EXISTS short_urls (
    id BIGSERIAL PRIMARY KEY,
    short_code VARCHAR(32) UNIQUE,
    original_url TEXT NOT NULL,
    visit_count BIGINT NOT NULL DEFAULT 0,
    status SMALLINT NOT NULL DEFAULT 1,
    is_deleted SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_short_urls_code ON short_urls(short_code);

-- Ensure sequence starts at 10001 (for empty table). If table has data, continue from max(id).
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE c.relkind = 'S'
          AND c.relname = 'short_urls_id_seq'
    ) THEN
        PERFORM setval('short_urls_id_seq', GREATEST((SELECT COALESCE(MAX(id), 0) FROM short_urls), 10000));
    END IF;
END $$;
