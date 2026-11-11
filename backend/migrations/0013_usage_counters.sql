-- School-wide usage statistics for administrators. Only daily totals per action
-- are stored: no teacher, class or pupil identifiers and no content. Counting
-- starts with this migration; existing records are deliberately not counted.
-- Days are UTC dates. Any timeframe is the sum of its days. The application
-- decides what to count (backend/src/db/usage.rs); this is storage only.
CREATE TABLE usage_counters (
 day TEXT NOT NULL CHECK(day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
 metric TEXT NOT NULL,
 count INTEGER NOT NULL CHECK(count >= 0),
 PRIMARY KEY(day, metric)
) WITHOUT ROWID;

-- Marks the first counted day, so the statistics can say since when they count.
INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'counting_started', 1);

-- The UTC day of a teacher's latest activity, so each teacher counts as active
-- at most once per day. Never exposed.
ALTER TABLE teachers ADD COLUMN last_active_day TEXT;
