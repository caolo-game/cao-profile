DROP TABLE record;

CREATE TABLE record (
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    duration_mus_avg REAL NOT NULL DEFAULT 0.0,
    -- to get STD: take the square root of `duration_mus_std_sq / (num_items-1)`
    duration_mus_std_sq REAL NOT NULL DEFAULT 0.0,
    num_items INTEGER NOT NULL DEFAULT 0,

    name VARCHAR NOT NULL,
    file VARCHAR NOT NULL,
    line INTEGER NOT NULL,

    PRIMARY KEY (name,file,line)
);

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
