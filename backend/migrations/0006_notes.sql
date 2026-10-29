CREATE TABLE student_notes (
 id TEXT PRIMARY KEY,
 classroom_id TEXT NOT NULL,
 student_id TEXT NOT NULL,
 content TEXT NOT NULL CHECK(length(trim(content)) BETWEEN 1 AND 10000),
 created_by TEXT NOT NULL REFERENCES teachers(id),
 created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 updated_by TEXT NOT NULL REFERENCES teachers(id),
 updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

CREATE INDEX student_notes_class_date ON student_notes(classroom_id,created_at DESC);
