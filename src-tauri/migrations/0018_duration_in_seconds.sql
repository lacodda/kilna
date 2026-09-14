-- A duration is a number of seconds, not a string that looks like a time.
--
-- The board is timed from the work's duration: the scenes divide it
-- proportionally and are adjusted by hand (decision of 2026-09-11). Dividing
-- needs a number, and `duration` shipped as a `text` field holding "3:45" --
-- a value nothing in the code could read. Two truths about one length (a
-- text field beside a numeric one) is what ADR 0001 forbids, so the field
-- itself is retyped rather than joined by a second one (decision of
-- 2026-09-14).
--
-- Both halves move together, because either alone leaves the workspace
-- inconsistent: the field's type in every stored profile, and every value
-- already written under that key. `carry_forward` cannot do this -- it
-- matches vocabulary by key and leaves a stored entry as the owner has it,
-- which is what keeps a renamed or retyped field theirs.
--
-- Values are read as m:ss or h:mm:ss and written as whole seconds. Anything
-- else -- a plain number already, or prose -- is left exactly as it stands:
-- a length nobody can parse is not worth guessing at, and the field shows it
-- back to the person unchanged.

-- The field's type, in every stored profile that names it.
UPDATE profile
SET config = (
        SELECT json_set(
            profile.config,
            '$.work_meta_fields[' || field.key || '].type',
            'number'
        )
        FROM json_each(profile.config, '$.work_meta_fields') AS field
        WHERE json_extract(field.value, '$.key') = 'duration'
    ),
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
WHERE EXISTS (
    SELECT 1 FROM json_each(profile.config, '$.work_meta_fields') AS field
    WHERE json_extract(field.value, '$.key') = 'duration'
      AND json_extract(field.value, '$.type') = 'text'
);

-- m:ss -- the shape every stored duration actually has.
UPDATE work
SET meta = json_set(
        meta,
        '$.duration',
        CAST(substr(json_extract(meta, '$.duration'), 1, instr(json_extract(meta, '$.duration'), ':') - 1) AS INTEGER) * 60
        + CAST(substr(json_extract(meta, '$.duration'), instr(json_extract(meta, '$.duration'), ':') + 1) AS INTEGER)
    )
WHERE json_valid(meta)
  AND json_type(meta, '$.duration') = 'text'
  AND json_extract(meta, '$.duration') GLOB '[0-9]*:[0-9][0-9]'
  AND json_extract(meta, '$.duration') NOT GLOB '*:*:*';

-- h:mm:ss, for a length that ran past the hour.
UPDATE work
SET meta = json_set(
        meta,
        '$.duration',
        CAST(substr(json_extract(meta, '$.duration'), 1, instr(json_extract(meta, '$.duration'), ':') - 1) AS INTEGER) * 3600
        + CAST(substr(json_extract(meta, '$.duration'), instr(json_extract(meta, '$.duration'), ':') + 1, 2) AS INTEGER) * 60
        + CAST(substr(json_extract(meta, '$.duration'), -2) AS INTEGER)
    )
WHERE json_valid(meta)
  AND json_type(meta, '$.duration') = 'text'
  AND json_extract(meta, '$.duration') GLOB '[0-9]*:[0-9][0-9]:[0-9][0-9]';
