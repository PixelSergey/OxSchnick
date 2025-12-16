-- database to store more expensive stats
CREATE TABLE metrics (
    id int PRIMARY KEY,
    longest_winning_streak integer NOT NULL,
    current_winning_streak integer NOT NULL,
    longest_losing_streak integer NOT NULL,
    current_losing_streak integer NOT NULL,
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
CREATE TRIGGER insertUserIntoStreaks
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