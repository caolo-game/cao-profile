DELETE FROM record WHERE 1=1;

ALTER TABLE record ADD COLUMN duration_us_total REAL NOT NULL DEFAULT 0.0;
ALTER TABLE record RENAME duration_mus_avg TO duration_us_avg;
ALTER TABLE record RENAME duration_mus_std_sq TO duration_us_std_sq;

CREATE OR REPLACE PROCEDURE add_to_record (REAL, VARCHAR, VARCHAR, INTEGER) 
LANGUAGE plpgsql
AS $$
DECLARE
    tmp REAL := 0.0;
BEGIN
    -- Add the record is not exists
    INSERT INTO record (name, file, line)
    VALUES ($2, $3, $4)
    ON CONFLICT DO NOTHING;

    tmp := (
        SELECT $1 - duration_us_avg
        FROM record
        WHERE name=$2 AND file=$3 AND line=$4
    );

    UPDATE record
    SET
        duration_us_avg = duration_us_avg + (tmp / (num_items + 1)),
        duration_us_total = duration_us_total + $1
    WHERE name=$2 AND file=$3 AND line=$4;

    UPDATE record
    SET
        duration_us_std_sq = duration_us_std_sq + (tmp * ($1 - duration_us_avg)),
        num_items = num_items + 1
    WHERE name=$2 AND file=$3 AND line=$4;
END;
$$;
