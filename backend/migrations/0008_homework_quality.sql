-- Existing submissions stay ungraded until a teacher chooses a quality.
ALTER TABLE homework_submissions ADD COLUMN quality TEXT
 CHECK(quality IN ('all_correct','partially_correct','missing','all_incorrect'));
