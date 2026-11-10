-- Spaces (v0.109, ADR 0077): a space is a folder with a database of its own,
-- and what was the person's on this machine whatever profile was open moves
-- out of the workspace into the application's database (`app.db`).
--
-- The assistant's settings - the week's budget, how many runs may go at once,
-- the model and what a run may read - are the person's and this machine's,
-- not one space's. kilna copies the row into `app.db` before this step runs
-- on a workspace from before spaces (`space::legacy`), so dropping the table
-- here loses nothing.

DROP TABLE assistant_settings;
