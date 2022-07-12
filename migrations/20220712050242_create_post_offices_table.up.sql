CREATE TABLE post_offices (
    id UUID PRIMARY KEY,
    city_code CHAR(5),
    facility_category_code CHAR(2),
    facility_subcategory_code CHAR(5),
    post_office_code CHAR(5),
    name VARCHAR(40),
    address VARCHAR(80),
    geom geometry(POINT, 3857)
);
