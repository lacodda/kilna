-- The configuration stage (v0.108): what the assistant may do on this
-- workspace, and what the window keeps for each plugin.
--
-- Three more columns on the one row of the assistant's settings, each with
-- its bounds in the schema, as 0048 promised. A row that already holds a budget
-- keeps it and reads the defaults: three runs, the model Claude Code itself is
-- set to, and the empty directory of ADR 0008.

-- How many CLI processes may be alive at once. Each costs a few hundred
-- megabytes, so this is a memory budget, and eight is already more than a
-- working machine should be asked to carry.
ALTER TABLE assistant_settings
    ADD COLUMN parallel_runs INTEGER NOT NULL DEFAULT 3
    CHECK (parallel_runs BETWEEN 1 AND 8);

-- The model a run is started with, as the CLI's `--model` takes it. None is
-- whatever Claude Code itself is set to.
ALTER TABLE assistant_settings
    ADD COLUMN model TEXT
    CHECK (model IS NULL OR length(model) BETWEEN 1 AND 100);

-- What a run may read beyond what its prompt carries (ADR 0074): `nothing`, or
-- the files of the `work` its chat is about.
ALTER TABLE assistant_settings
    ADD COLUMN run_sees TEXT NOT NULL DEFAULT 'nothing'
    CHECK (run_sees IN ('nothing', 'work'));

-- What the window keeps for a plugin, by the name of its executable (ADR 0075).
-- A row only once something was changed: a plugin nobody touched is on and
-- reads its own defaults. `settings` holds only what was set here, so a default
-- the plugin changes in its next release reaches everyone who left it alone. A
-- row outlives the plugin, so a reinstall finds its settings where they were.
--
-- Past the operations log, like the budget and the media folder: which plugins
-- stand on this machine, and what they are told, is this machine's business.
CREATE TABLE plugin_settings (
    executable TEXT PRIMARY KEY,
    enabled    INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    settings   TEXT NOT NULL DEFAULT '{}'
               CHECK (json_valid(settings) AND json_type(settings) = 'object'),
    updated_at TEXT NOT NULL
) STRICT;
