-- database to store more expensive stats
CREATE TABLE metrics (
    id int PRIMARY KEY,
    num_schnicks integer NOT NULL DEFAULT 0,
    num_won integer NOT NULL DEFAULT 0,
    longest_winning_streak integer NOT NULL DEFAULT 0,
    current_winning_streak integer NOT NULL DEFAULT 0,
    longest_losing_streak integer NOT NULL DEFAULT 0,
    current_losing_streak integer NOT NULL DEFAULT 0,
    FOREIGN KEY(id) REFERENCES users(id)
);

-- function to insert new user into metrics
CREATE FUNCTION insert_user_into_metrics()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO metrics(id)
    VALUES(NEW.id);
    RETURN NEW;
END;
$$;

-- trigger to add new users to the metrics table
CREATE TRIGGER insertUserIntoMetrics
AFTER INSERT
ON users
FOR EACH ROW
EXECUTE FUNCTION insert_user_into_metrics();

-- function to update the streak in the metrics table
CREATE FUNCTION update_streak()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE metrics
    SET
        current_winning_streak = current_winning_streak + 1,
        longest_winning_streak = GREATEST(longest_winning_streak, current_winning_streak + 1),
        current_losing_streak = 0
    WHERE id = NEW.winner;
    UPDATE metrics
    SET
        current_losing_streak = current_losing_streak + 1,
        longest_losing_streak = GREATEST(longest_losing_streak, current_losing_streak + 1),
        current_winning_streak = 0
    WHERE id = NEW.loser;
    RETURN NEW;
END;
$$;

-- trigger that gets called on a new match to update the metrics of loser and winner
CREATE TRIGGER updateStreak
AFTER INSERT
ON schnicks
FOR EACH ROW
EXECUTE FUNCTION update_streak();

-- function to update number of schnicks on new schnick
CREATE FUNCTION update_num_schnicks()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE metrics
    SET num_schnicks = num_schnicks + 1
    WHERE id = NEW.winner OR id = NEW.loser;
    UPDATE metrics
    SET num_won = num_won + 1
    WHERE id = NEW.winner;
    RETURN NEW;
END;
$$;

CREATE TRIGGER updateNumSchnicks
AFTER INSERT
ON schnicks
FOR EACH ROW
EXECUTE FUNCTION update_num_schnicks();