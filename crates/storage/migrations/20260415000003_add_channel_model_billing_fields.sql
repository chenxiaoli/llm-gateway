-- Migration: Add billing and pricing fields to channel_models table
-- Date: 2026-04-15

ALTER TABLE channel_models ADD COLUMN billing_type TEXT;
ALTER TABLE channel_models ADD COLUMN input_price REAL;
ALTER TABLE channel_models ADD COLUMN output_price REAL;
ALTER TABLE channel_models ADD COLUMN request_price REAL;