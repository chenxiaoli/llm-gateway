-- Make provider_id nullable in models table
-- Supports N:N model-provider relationship
ALTER TABLE models ALTER COLUMN provider_id DROP NOT NULL;