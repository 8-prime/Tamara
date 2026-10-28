-- Keep the observation date separate from last edit/copy timestamps.
ALTER TABLE math_assessments ADD COLUMN achieved_on TEXT
 CHECK(achieved_on IS NULL OR (value='yes' AND achieved_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'));
-- Existing assessments have only their last saved timestamp available.
UPDATE math_assessments SET achieved_on=date(updated_at,'unixepoch') WHERE value='yes';
