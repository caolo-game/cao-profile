DROP TRIGGER record_updated ON record;

CREATE TRIGGER record_updated AFTER UPDATE ON record
FOR EACH ROW EXECUTE PROCEDURE on_record_update();
