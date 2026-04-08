-- Add custom_save_paths column to games table
-- JSON array of user-added save paths that survive rescans
ALTER TABLE games ADD COLUMN custom_save_paths TEXT NOT NULL DEFAULT '[]';
