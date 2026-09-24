-- A style brick can now come back from the trash.
--
-- Until v0.76.1 deleting a brick was a plain DELETE: gone, with no entry to
-- restore and nothing for undo to take back, and the pictures it was described
-- from went with it. The brick is a trash entity now (`trash::Entity::Style`),
-- so a row of this table can be deleted and then inserted again under the same
-- id - which is exactly the case 0026 did not cover. Every other table a
-- person's work lives in has the pair of tombstone triggers from 0011; this one
-- had only the half that buries. Without the other half a restored brick would
-- still read as deleted to anything that trusts the tombstones, which is what
-- synchronisation will do.

CREATE TRIGGER style_brick_tombstone_on_insert AFTER INSERT ON style_brick
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'style_brick' AND entity_id = NEW.id AND restored_at IS NULL;
END;
