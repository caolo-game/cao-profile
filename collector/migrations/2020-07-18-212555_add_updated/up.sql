ALTER TABLE record ADD COLUMN updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now();

CREATE OR REPLACE FUNCTION on_record_update() RETURNS trigger
AS $$
BEGIN
    NEW.updated := now();
    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER record_updated AFTER UPDATE ON record
FOR EACH ROW EXECUTE PROCEDURE on_record_update();
