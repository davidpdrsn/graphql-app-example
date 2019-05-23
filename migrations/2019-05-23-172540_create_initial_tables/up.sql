CREATE TABLE countries (
    id serial PRIMARY KEY,
    name text NOT NULL
);

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    country_id integer NOT NULL REFERENCES countries(id)
);
