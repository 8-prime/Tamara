-- Notes belong to an individual pupil's assignment, independently of grading.
ALTER TABLE homework_submissions ADD COLUMN note TEXT NOT NULL DEFAULT ''
 CHECK(length(note) <= 10000);
