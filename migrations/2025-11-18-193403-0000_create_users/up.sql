CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username varchar(32),
    dect char(4),
    parent integer references users(id) NOT NULL,
    token char(36) NOT NULL,
    created timestamptz NOT NULL,
    active boolean NOT NULL
);

INSERT INTO users (username, dect, parent, token, created, active) VALUES ('root', '5000', lastval(), uuidv4(), now(), true);