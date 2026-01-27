CREATE TABLE colleges (
    id integer PRIMARY KEY,
    college varchar(32) not null unique
);

INSERT INTO colleges (id, college) VALUES
    (1, 'Balliol College'),
    (2, 'Brasenose College'),
    (3, 'Christ Church College'),
    (4, 'Exeter College'),
    (5, 'Harris Manchester College'),
    (6, 'Jesus College'),
    (7, 'Kellogg College'),
    (8, 'Linacre College'),
    (9, 'Magdalen College'),
    (10, 'Merton College'),
    (11, 'Nuffield College'),
    (12, 'Pembroke College'),
    (13, 'Reuben College'),
    (14, 'St Anne''s College'),
    (15, 'St Catherine''s College'),
    (16, 'St Edmund Hall'),
    (17, 'St Hugh''s College'),
    (18, 'St Peter''s College'),
    (19, 'Trinity College'),
    (20, 'Wadham College'),
    (21, 'Worcester College'),
    (22, 'Blackfriars'),
    (23, 'Campion Hall'),
    (24, 'Corpus Christi College'),
    (25, 'Green Templeton College'),
    (26, 'Hertford College'),
    (27, 'Keble College'),
    (28, 'Lady Margaret Hall'),
    (29, 'Lincoln College'),
    (30, 'Mansfield College'),
    (31, 'New College'),
    (32, 'Oriel College'),
    (33, 'Regent''s Park College'),
    (34, 'Somerville College'),
    (35, 'St Antony''s College'),
    (36, 'St Cross College'),
    (37, 'St Hilda''s College'),
    (38, 'St John''s College'),
    (39, 'The Queen''s College'),
    (40, 'University College'),
    (41, 'Wolfson College'),
    (42, 'Wycliffe Hall'),
    (43, 'Brookes'),
    (0, 'Other');

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
