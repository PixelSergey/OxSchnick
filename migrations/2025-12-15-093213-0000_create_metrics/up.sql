-- database to store more expensive stats
CREATE TABLE metrics (
    id int PRIMARY KEY,
    num_schnicks integer NOT NULL DEFAULT 0,
    num_won integer NOT NULL DEFAULT 0,
    longest_winning_streak integer NOT NULL DEFAULT 0,
    current_winning_streak integer NOT NULL DEFAULT 0,
    longest_losing_streak integer NOT NULL DEFAULT 0,
    current_losing_streak integer NOT NULL DEFAULT 0,
    num_children integer NOT NULL DEFAULT 0,
    num_rock integer NOT NULL DEFAULT 0,
    num_paper integer NOT NULL DEFAULT 0,
    num_scissors integer NOT NULL DEFAULT 0,
    FOREIGN KEY(id) REFERENCES users(id)
);

INSERT INTO metrics (id) VALUES (1);

-- function to update number of schnicks and wins on new schnick
CREATE FUNCTION update_num_schnicks()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE metrics
        SET num_schnicks = num_schnicks + 1
    WHERE (id = NEW.winner OR id = NEW.loser) AND (NEW.winner <> 1 AND NEW.loser <> 1);
    UPDATE metrics
        SET num_won = num_won + 1
    WHERE id = NEW.winner AND NEW.loser <> 1 AND NEW.winner <> 1;
    UPDATE users SET active = true WHERE (id = NEW.winner) OR (id = NEW.loser);
    RETURN NEW;
END;
$$;

-- trigger to update number of schnicks and wins on new schnick
CREATE TRIGGER updateNumSchnicks
AFTER INSERT
ON schnicks
FOR EACH ROW
EXECUTE FUNCTION update_num_schnicks();

-- function to insert new user into metrics
-- also updates count of children for parent
CREATE FUNCTION insert_user_into_metrics()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    -- insert user
    INSERT INTO metrics(id)
    VALUES(NEW.id);

    -- update children count for parent
    UPDATE METRICS
    SET
        num_children = num_children + 1
    WHERE id = NEW.parent;
    RETURN NEW;
END;
$$;

-- trigger to add new users to the metrics table
-- also updates count of children for parent
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
    WHERE id = NEW.winner AND NEW.loser <> 1 AND NEW.winner <> 1;
    UPDATE metrics
    SET
        current_losing_streak = current_losing_streak + 1,
        longest_losing_streak = GREATEST(longest_losing_streak, current_losing_streak + 1),
        current_winning_streak = 0
    WHERE id = NEW.loser AND NEW.winner <> 1 AND NEW.loser <> 1;
    RETURN NEW;
END;
$$;

-- trigger that gets called on a new match to update the metrics of loser and winner
CREATE TRIGGER updateStreak
AFTER INSERT
ON schnicks
FOR EACH ROW
EXECUTE FUNCTION update_streak();

-- function to increase number of used weapons on schnick
CREATE FUNCTION update_weapons()
    RETURNS trigger
    LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.weapon = 0 THEN
        UPDATE metrics
            SET num_rock = num_rock + 1
        WHERE id = NEW.winner AND NEW.loser <> 1 AND NEW.winner <> 1;

        UPDATE metrics
            SET num_scissors = num_scissors + 1
        WHERE id = NEW.loser AND NEW.winner <> 1 AND NEW.loser <> 1;

    ELSIF NEW.weapon = 1 THEN
        UPDATE metrics
            SET num_scissors = num_scissors + 1
        WHERE id = NEW.winner AND NEW.loser <> 1 AND NEW.winner <> 1;

        UPDATE metrics
            SET num_paper = num_paper + 1
        WHERE id = NEW.loser AND NEW.winner <> 1 AND NEW.loser <> 1;

    ELSIF NEW.weapon = 2 THEN
        UPDATE metrics
            SET num_paper = num_paper + 1
        WHERE id = NEW.winner AND NEW.loser <> 1 AND NEW.winner <> 1;

        UPDATE metrics
            SET num_rock = num_rock + 1
        WHERE id = NEW.loser AND NEW.winner <> 1 AND NEW.loser <> 1;
    END IF;

    RETURN NEW;
END;
$$;

-- trigger to increase number of used weapons on schnick
CREATE TRIGGER updateWeapons
AFTER INSERT
ON schnicks
FOR EACH ROW
EXECUTE FUNCTION update_weapons();