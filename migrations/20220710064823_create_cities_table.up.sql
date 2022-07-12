CREATE TABLE cities (
    id UUID PRIMARY KEY,
    code CHAR(5) NOT NULL,
    area VARCHAR(40),
    name VARCHAR(40) NOT NULL,
    geom geometry(POLYGON, 3857)
);
CREATE INDEX idx_cities_code ON cities USING btree (code);
CREATE INDEX idx_cities_geom ON cities USING gist (geom);
