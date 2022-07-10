CREATE TABLE prefectures (
    id UUID PRIMARY KEY,
    code CHAR(2) NOT NULL,
    name VARCHAR(40) NOT NULL,
    geom geometry(POLYGON, 6668)
);
