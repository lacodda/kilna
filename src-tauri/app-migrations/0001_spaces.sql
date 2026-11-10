-- The application's own database (v0.109, ADR 0077): what this machine knows
-- about the spaces it has opened, and the settings that are the person's
-- whatever space is open. It lives in the application's data directory and
-- never travels with a space.

-- A space this machine has opened. What a space is - its name, its kind, its
-- colour and its mark - is the space's own and lives in its folder
-- (`space.kilna`); this row says only where the folder is and what this
-- machine has done with it, so a folder moved to another machine is the same
-- space there.
CREATE TABLE space (
    -- The id the space's own file gives it.
    id         TEXT PRIMARY KEY,
    -- The folder. Relative to the application's data directory when it lies
    -- under it, so a portable copy keeps its spaces when its drive letter
    -- changes; absolute otherwise.
    path       TEXT NOT NULL UNIQUE CHECK (length(path) > 0),
    added_at   TEXT NOT NULL,
    -- When the window last opened it: the newest is the one a start, and
    -- `kilna --mcp`, open.
    opened_at  TEXT,
    -- Put aside: not listed in the switcher, nothing on disk touched.
    trashed_at TEXT,
    -- The name was given by kilna - the move from one database, or a first
    -- start - not by the person: the window asks for one, once.
    ask_name   INTEGER NOT NULL DEFAULT 0 CHECK (ask_name IN (0, 1))
) STRICT;

CREATE INDEX space_recent ON space (opened_at DESC) WHERE trashed_at IS NULL;

-- The assistant's settings, moved here from each workspace (0048 and 0049 of a
-- space's schema), with the columns and the bounds they had there: one row, a
-- column a setting, so a budget of nothing or eleven runs at once is not a
-- value the table can hold.
CREATE TABLE assistant_settings (
    id                INTEGER PRIMARY KEY CHECK (id = 1),
    -- What a week of runs may cost before the window says so, in US dollars
    -- as the CLI prices them. Soft: nothing is refused for crossing it.
    weekly_budget_usd REAL CHECK (weekly_budget_usd IS NULL OR weekly_budget_usd > 0),
    updated_at        TEXT NOT NULL,
    -- How many CLI processes may be alive at once: a memory budget.
    parallel_runs     INTEGER NOT NULL DEFAULT 3 CHECK (parallel_runs BETWEEN 1 AND 8),
    -- The model a run is started with, as the CLI's `--model` takes it; none is
    -- whatever Claude Code itself is set to.
    model             TEXT CHECK (model IS NULL OR length(model) BETWEEN 1 AND 100),
    -- What a run may read beyond its prompt (ADR 0074).
    run_sees          TEXT NOT NULL DEFAULT 'nothing' CHECK (run_sees IN ('nothing', 'work'))
) STRICT;
