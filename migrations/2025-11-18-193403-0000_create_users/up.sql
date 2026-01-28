CREATE TABLE colleges (
    id integer PRIMARY KEY not null unique,
    college varchar(32) not null unique
);

INSERT INTO colleges (id, college) VALUES
    (0, 'Other'),
    (1, 'Balliol'),
    (2, 'Blackfriars'),
    (3, 'Brasenose'),
    (4, 'Campion Hall'),
    (5, 'Christ Church'),
    (6, 'Corpus Christi'),
    (7, 'Exeter'),
    (8, 'Green Templeton'),
    (9, 'Harris Manchester'),
    (10, 'Hertford'),
    (11, 'Jesus'),
    (12, 'Keble'),
    (13, 'Kellogg'),
    (14, 'Lady Margaret Hall'),
    (15, 'Linacre'),
    (16, 'Lincoln'),
    (17, 'Magdalen'),
    (18, 'Mansfield'),
    (19, 'Merton'),
    (20, 'New'),
    (21, 'Nuffield'),
    (22, 'Oriel'),
    (23, 'Pembroke'),
    (24, 'Queen''s'),
    (25, 'Regent''s Park'),
    (26, 'Reuben'),
    (27, 'Somerville'),
    (28, 'St Antony''s'),
    (29, 'St Anne''s'),
    (30, 'St Catherine''s'),
    (31, 'St Cross'),
    (32, 'St Edmund Hall'),
    (33, 'St Hilda''s'),
    (34, 'St Hugh''s'),
    (35, 'St John''s'),
    (36, 'St Peter''s'),
    (37, 'Trinity'),
    (38, 'University'),
    (39, 'Wadham'),
    (40, 'Worcester'),
    (41, 'Wolfson'),
    (42, 'Wycliffe Hall'),
    (43, 'Brookes');

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username varchar(32) NOT NULL UNIQUE,
    college integer references colleges(id),
    parent integer references users(id) NOT NULL,
    token uuid NOT NULL DEFAULT uuidv4(),
    created timestamptz NOT NULL DEFAULT now(),
    active boolean NOT NULL DEFAULT false
);

INSERT INTO users (username, college, parent, token, created, active) VALUES ('root', 0, lastval(), uuidv4(), now(), true);
