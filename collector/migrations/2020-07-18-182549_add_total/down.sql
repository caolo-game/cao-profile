ALTER TABLE record DROP COLUMN duration_us_total;
ALTER TABLE record RENAME duration_us_avg TO duration_mus_avg;
ALTER TABLE record RENAME duration_us_std_sq TO duration_mus_std_sq;

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
        SELECT $1 - duration_mus_avg
        FROM record
        WHERE name=$2 AND file=$3 AND line=$4
    );

    UPDATE record
    SET
        duration_mus_avg = duration_mus_avg + (tmp / (num_items + 1))
    WHERE name=$2 AND file=$3 AND line=$4;

    UPDATE record
    SET
        duration_mus_std_sq = duration_mus_std_sq + (tmp * ($1 - duration_mus_avg)),
        num_items = num_items + 1
    WHERE name=$2 AND file=$3 AND line=$4;
END;
$$;
