-- A work's stage is one of five stops (FR-03, v0.119).
--
-- Until now the scale had seven stops - 0, 17, 33, 50, 67, 83, 100 - and
-- before that six, and a work nobody had judged held nothing at all. Nobody
-- could say what told "Nearly there" from "Polishing", and "nobody has said"
-- drew like a bare idea and meant the same to anyone looking for what to
-- pick up next.
--
-- Every work lands on the nearest of 0, 25, 50, 75, 100, and one with no
-- stage on the first of them. The thresholds are the midpoints between the
-- new stops, so no work moves further than twelve points from where its
-- author put it: of the seven, 17 and 33 go to 25, 50 stays, 67 and 83 go
-- to 75, and the ends stay the ends.
--
-- The column stays nullable - SQLite cannot change that without rebuilding
-- the table, and nothing gains from it: every insert writes 0 and every
-- write passes through `normalize_stage`, which is also what a line of the
-- operation log from before this goes through when it is replayed.
UPDATE work
   SET stage = CASE
         WHEN stage IS NULL OR stage <= 12 THEN 0
         WHEN stage <= 37 THEN 25
         WHEN stage <= 62 THEN 50
         WHEN stage <= 87 THEN 75
         ELSE 100
       END
 WHERE stage IS NULL OR stage NOT IN (0, 25, 50, 75, 100);
