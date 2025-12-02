CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username varchar(32),
    parent integer references users(id) NOT NULL,
    token char(36) NOT NULL,
    invite char(36) NOT NULL,
    active boolean NOT NULL
)