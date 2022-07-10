CREATE TABLE prefectures (
    id UUID PRIMARY KEY,
    code VARCHAR(2) NOT NULL,
    name VARCHAR(40) NOT NULL,
    geom geometry(POLYGON, 6668)
);
