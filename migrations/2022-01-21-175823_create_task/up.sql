-- Your SQL goes here
CREATE TABLE tasks (
  id SERIAL PRIMARY KEY,
  done boolean NOT NULL DEFAULT false,
  img VARCHAR,
  title VARCHAR NOT NULL,
  description VARCHAR NOT NULL DEFAULT '',
  points integer NOT NULL DEFAULT 0,
  parent VARCHAR,
  children integer[] NOT NULL DEFAULT array[]::integer[]
);

-- CREATE TABLE children (
--  parent_id integer,
--  child_id integer,
--FOREIN KEY (parent_id)
--  REFERENCES tasks (id),
--FOREIN KEY (child_id)
--  REFERENCES tasks (id),
--PRIMARY KEY (parent_id, child_id)
--);
