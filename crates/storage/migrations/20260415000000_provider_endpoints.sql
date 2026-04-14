-- Migration: Add base_url and endpoints to providers table
-- Run this after existing migrations

-- Add new columns
ALTER TABLE providers ADD COLUMN base_url TEXT;
ALTER TABLE providers ADD COLUMN endpoints TEXT;

-- Migrate existing URLs to endpoints JSON
-- Note: This is a simplified migration. Adjust based on actual data.
UPDATE providers SET endpoints =
    CASE
        WHEN openai_base_url IS NOT NULL OR anthropic_base_url IS NOT NULL
        THEN '{"openai":"' || COALESCE(openai_base_url, '') || '","anthropic":"' || COALESCE(anthropic_base_url, '') || '"}'
        ELSE NULL
    END
WHERE openai_base_url IS NOT NULL OR anthropic_base_url IS NOT NULL;

-- Set base_url as fallback (use openai_base_url as default)
UPDATE providers SET base_url = openai_base_url WHERE openai_base_url IS NOT NULL;