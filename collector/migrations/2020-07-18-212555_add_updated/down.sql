ALTER TABLE record DROP COLUMN updated;
DROP TRIGGER record_updated ON record;
