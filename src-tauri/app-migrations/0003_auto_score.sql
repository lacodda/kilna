-- Scoring a text by itself (FR-01, v0.123).
--
-- The person's, whatever space is open, like the budget beside them: whether
-- the assistant scores a text when it has been left alone for a while, from
-- which stage on, how long "a while" is, and whether what it says is kept as
-- a score at once or waits as a proposal.
--
-- Off by default, both of them. A score costs money, and turning the first on
-- scores nothing that is already there - only what is written from then on,
-- or what the catch-up button is pressed for. The second off keeps the rule
-- the rest of the assistant lives by: it proposes, a person keeps.
ALTER TABLE assistant_settings
    ADD COLUMN auto_score INTEGER NOT NULL DEFAULT 0 CHECK (auto_score IN (0, 1));

-- Minutes a text has to be left alone before it is scored: long enough that
-- a person still writing is not scored line by line.
ALTER TABLE assistant_settings
    ADD COLUMN auto_score_debounce_min INTEGER NOT NULL DEFAULT 10
        CHECK (auto_score_debounce_min BETWEEN 1 AND 1440);

-- The stage a work has to stand at: one of the five stops. Half way by
-- default - a first draft is not worth a run.
ALTER TABLE assistant_settings
    ADD COLUMN auto_score_min_stage INTEGER NOT NULL DEFAULT 50
        CHECK (auto_score_min_stage IN (0, 25, 50, 75, 100));

ALTER TABLE assistant_settings
    ADD COLUMN auto_score_apply INTEGER NOT NULL DEFAULT 0 CHECK (auto_score_apply IN (0, 1));
