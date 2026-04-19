-- Remove enabled column from models table
-- Model availability is now determined solely by whether it has active channel_models
ALTER TABLE models DROP COLUMN enabled;
