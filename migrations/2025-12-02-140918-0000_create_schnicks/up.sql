CREATE TABLE schnicks (
  id SERIAL primary key,
  winner integer references users(id) NOT NULL,
  loser integer references users(id) NOT NULL,
  weapon integer NOT NULL,
  played_at timestamptz NOT NULL
)