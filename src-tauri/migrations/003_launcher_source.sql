-- Add launcher source column to games table
-- Identifies which launcher/store detected the game (steam, heroic, lutris, bottles, ea, ubisoft, rockstar, epic, gog, custom)
ALTER TABLE games ADD COLUMN launcher TEXT NOT NULL DEFAULT 'steam';
