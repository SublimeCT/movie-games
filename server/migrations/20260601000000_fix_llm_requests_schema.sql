DO $$
BEGIN
  IF to_regclass('public.llm_requests') IS NULL AND to_regclass('public.glm_requests') IS NOT NULL THEN
    EXECUTE 'ALTER TABLE public.glm_requests RENAME TO llm_requests';
  END IF;
END $$;

DO $$
BEGIN
  IF to_regclass('public.llm_requests') IS NOT NULL THEN
    IF EXISTS (
      SELECT 1
      FROM information_schema.columns
      WHERE table_schema = 'public'
        AND table_name = 'llm_requests'
        AND column_name = 'glm_prompt'
    ) AND NOT EXISTS (
      SELECT 1
      FROM information_schema.columns
      WHERE table_schema = 'public'
        AND table_name = 'llm_requests'
        AND column_name = 'llm_prompt'
    ) THEN
      EXECUTE 'ALTER TABLE public.llm_requests RENAME COLUMN glm_prompt TO llm_prompt';
    END IF;

    IF EXISTS (
      SELECT 1
      FROM information_schema.columns
      WHERE table_schema = 'public'
        AND table_name = 'llm_requests'
        AND column_name = 'glm_response'
    ) AND NOT EXISTS (
      SELECT 1
      FROM information_schema.columns
      WHERE table_schema = 'public'
        AND table_name = 'llm_requests'
        AND column_name = 'llm_response'
    ) THEN
      EXECUTE 'ALTER TABLE public.llm_requests RENAME COLUMN glm_response TO llm_response';
    END IF;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS public.llm_requests (
  id uuid PRIMARY KEY,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  client_ip text NOT NULL,
  user_agent text,
  route text NOT NULL,
  status text NOT NULL,
  request_payload jsonb NOT NULL,
  llm_prompt text NOT NULL,
  llm_response text,
  error_text text,
  response_time_ms bigint,
  shared boolean NOT NULL DEFAULT false,
  processed_response jsonb,
  template_source text NOT NULL DEFAULT 'llm'
);

ALTER TABLE public.llm_requests ADD COLUMN IF NOT EXISTS user_agent text;
ALTER TABLE public.llm_requests ADD COLUMN IF NOT EXISTS shared boolean NOT NULL DEFAULT false;
ALTER TABLE public.llm_requests ADD COLUMN IF NOT EXISTS processed_response jsonb;
ALTER TABLE public.llm_requests ADD COLUMN IF NOT EXISTS template_source text NOT NULL DEFAULT 'llm';

DO $$
BEGIN
  IF to_regclass('public.idx_llm_requests_status_created_at') IS NULL
     AND to_regclass('public.idx_glm_requests_status_created_at') IS NULL THEN
    EXECUTE 'CREATE INDEX idx_llm_requests_status_created_at ON public.llm_requests(status, created_at DESC)';
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS public.records (
  id uuid PRIMARY KEY,
  request_id uuid NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  client_ip text NOT NULL,
  user_agent text,
  referer text
);

CREATE INDEX IF NOT EXISTS idx_records_request_id ON public.records(request_id);

DO $$
DECLARE
  rec record;
BEGIN
  IF to_regclass('public.records') IS NOT NULL THEN
    IF to_regclass('public.glm_requests') IS NOT NULL THEN
      FOR rec IN
        SELECT conname
        FROM pg_constraint
        WHERE contype = 'f'
          AND conrelid = 'public.records'::regclass
          AND confrelid = 'public.glm_requests'::regclass
      LOOP
        EXECUTE format('ALTER TABLE public.records DROP CONSTRAINT %I', rec.conname);
      END LOOP;
    END IF;

    IF NOT EXISTS (
      SELECT 1
      FROM pg_constraint
      WHERE contype = 'f'
        AND conrelid = 'public.records'::regclass
        AND confrelid = 'public.llm_requests'::regclass
    ) THEN
      EXECUTE 'ALTER TABLE public.records ADD CONSTRAINT records_request_id_fkey FOREIGN KEY (request_id) REFERENCES public.llm_requests(id)';
    END IF;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS public.shared_records (
  id uuid PRIMARY KEY,
  request_id uuid NOT NULL UNIQUE,
  shared_at timestamptz NOT NULL DEFAULT now(),
  shared_ip text NOT NULL,
  shared_user_agent text
);

CREATE INDEX IF NOT EXISTS idx_shared_records_request_id ON public.shared_records(request_id);

DO $$
DECLARE
  rec record;
BEGIN
  IF to_regclass('public.shared_records') IS NOT NULL THEN
    IF to_regclass('public.glm_requests') IS NOT NULL THEN
      FOR rec IN
        SELECT conname
        FROM pg_constraint
        WHERE contype = 'f'
          AND conrelid = 'public.shared_records'::regclass
          AND confrelid = 'public.glm_requests'::regclass
      LOOP
        EXECUTE format('ALTER TABLE public.shared_records DROP CONSTRAINT %I', rec.conname);
      END LOOP;
    END IF;

    IF NOT EXISTS (
      SELECT 1
      FROM pg_constraint
      WHERE contype = 'f'
        AND conrelid = 'public.shared_records'::regclass
        AND confrelid = 'public.llm_requests'::regclass
    ) THEN
      EXECUTE 'ALTER TABLE public.shared_records ADD CONSTRAINT shared_records_request_id_fkey FOREIGN KEY (request_id) REFERENCES public.llm_requests(id)';
    END IF;
  END IF;
END $$;
