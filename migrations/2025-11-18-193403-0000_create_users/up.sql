CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username varchar(32) NOT NULL UNIQUE,
    dect char(4),
    parent integer references users(id) NOT NULL,
    token uuid NOT NULL DEFAULT uuid_generate_v4(),
    created timestamptz NOT NULL DEFAULT now(),
    active boolean NOT NULL DEFAULT false
);

INSERT INTO users (username, dect, parent, token, created, active) VALUES ('root', '5000', lastval(), uuid_generate_v4(), now(), true);
