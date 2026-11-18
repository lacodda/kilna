-- A planned slot of a work (v0.138, FR-11): the day a work is meant to go
-- out before anything of it can. One per work; not a release.
CREATE TABLE work_plan (
    id           TEXT PRIMARY KEY,
    work_id      TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    scheduled_at TEXT NOT NULL,
    note         TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX work_plan_one_per_work ON work_plan (work_id);
CREATE INDEX work_plan_schedule ON work_plan (scheduled_at);
