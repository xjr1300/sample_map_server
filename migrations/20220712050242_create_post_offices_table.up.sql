CREATE TABLE post_offices (
    id UUID PRIMARY KEY,
    city_code CHAR(5),
    category_code CHAR(2),
    subcategory_code CHAR(5),
    post_office_code CHAR(5),
    name VARCHAR(40),
    address VARCHAR(80),
    geom geometry(POINT, 3857)
);

CREATE INDEX idx_post_offices_city_code ON post_offices USING btree (city_code);
CREATE INDEX idx_post_offices_geom ON post_offices USING gist (geom);
