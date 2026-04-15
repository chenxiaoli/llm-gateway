-- Migration: Add billing_type column to models table
-- Date: 2026-04-15

ALTER TABLE models ADD COLUMN billing_type TEXT NOT NULL DEFAULT 'per_token'
    CHECK(billing_type IN ('per_token', 'per_request', 'per_character', 'tiered_token', 'hybrid'));