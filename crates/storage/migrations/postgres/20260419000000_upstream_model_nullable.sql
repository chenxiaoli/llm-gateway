-- Make upstream_model_name nullable in channel_models table
ALTER TABLE channel_models ALTER COLUMN upstream_model_name DROP NOT NULL;