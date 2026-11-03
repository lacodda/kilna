-- What the assistant's runs cost, and how much a week may (v0.107).
--
-- A run's cost was kept on the answer it wrote, as `cost_usd` in the
-- message's meta. A run that fails costs too - the CLI's error result carries
-- `total_cost_usd` as a finished one does - and writes no answer, so its price
-- had nowhere to go and was dropped. The price is the run's: it moves onto the
-- row that already settles the run's state and detail from its last event,
-- and leaves the meta, so the chat list, the chat's Markdown and the day's and
-- the week's spend all read one number.

ALTER TABLE chat_run ADD COLUMN cost_usd REAL CHECK (cost_usd IS NULL OR cost_usd >= 0);

-- Every priced answer since v0.27 names its run, so the price moves whole.
UPDATE chat_run
   SET cost_usd = (
       SELECT json_extract(m.meta, '$.cost_usd')
         FROM chat_message m
        WHERE json_extract(m.meta, '$.run_id') = chat_run.id
          AND json_type(m.meta, '$.cost_usd') IN ('integer', 'real')
          AND json_extract(m.meta, '$.cost_usd') >= 0
        LIMIT 1
   )
 WHERE cost_usd IS NULL;

-- Out of the meta only where the run took it. An answer whose run is unknown
-- keeps its number rather than lose it, though nothing reads it there any
-- more; the one workspace this was run on before release had none.
UPDATE chat_message
   SET meta = json_remove(meta, '$.cost_usd')
 WHERE json_type(meta, '$.cost_usd') IS NOT NULL
   AND json_extract(meta, '$.run_id') IN (SELECT id FROM chat_run WHERE cost_usd IS NOT NULL);

-- The spend of a day or a week is a range of the moments runs ended.
CREATE INDEX chat_run_spent ON chat_run (ended_at) WHERE cost_usd IS NOT NULL;

-- The assistant's settings for this workspace: one row, a column a setting.
-- A row of columns rather than a table of keys, so that each setting has its
-- type and its bounds in the schema - a budget of nothing or of minus ten is
-- not a value this table can hold. The configuration stage (v0.108) adds its
-- columns here.
CREATE TABLE assistant_settings (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    -- What a week of runs may cost before the window says so, in US dollars
    -- as the CLI prices them. Soft: nothing is refused for crossing it.
    weekly_budget_usd REAL CHECK (weekly_budget_usd IS NULL OR weekly_budget_usd > 0),
    updated_at        TEXT NOT NULL
) STRICT;
