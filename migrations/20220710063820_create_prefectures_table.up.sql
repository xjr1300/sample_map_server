CREATE TABLE prefectures (
    id UUID PRIMARY KEY,
    code CHAR(2) NOT NULL,
    name VARCHAR(40) NOT NULL,
    geom geometry(POLYGON, 3857)
);
CREATE INDEX idx_prefectures_code ON prefectures USING btree (code);
CREATE INDEX idx_prefectures_geom ON prefectures USING gist (geom);
