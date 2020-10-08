DROP TABLE record;
CREATE TABLE record (
    id SERIAL PRIMARY KEY,
    created TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    duration_ns BIGINT NOT NULL,
    name VARCHAR NOT NULL,
    file VARCHAR NOT NULL,
    line INTEGER NOT NULL
);

CREATE INDEX record_name ON record (name);
