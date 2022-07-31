CREATE TABLE post_offices (
    id UUID PRIMARY KEY,
    city_code CHAR(5) NOT NULL,
    category_code CHAR(2) NOT NULL,
    subcategory_code CHAR(5) NOT NULL,
    post_office_code CHAR(5) NOT NULL,
    name VARCHAR(40) NOT NULL,
    address VARCHAR(80) NOT NULL,
    geom geometry(POINT, 3857 NOT NULL)
);

CREATE INDEX idx_post_offices_city_code ON post_offices USING btree (city_code);
CREATE INDEX idx_post_offices_geom ON post_offices USING gist (geom);
