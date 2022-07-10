CREATE TABLE cities (
    id UUID PRIMARY KEY,
    code CHAR(5) NOT NULL,
    area VARCHAR(40),
    name VARCHAR(40) NOT NULL
);
