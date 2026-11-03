CREATE TABLE conferences (
 id TEXT PRIMARY KEY,
 teacher_id TEXT NOT NULL REFERENCES teachers(id) ON DELETE CASCADE,
 classroom_id TEXT REFERENCES classrooms(id) ON DELETE SET NULL,
 date TEXT NOT NULL,
 title TEXT,
 content TEXT NOT NULL DEFAULT '',
 updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX conferences_teacher_date ON conferences(teacher_id,date DESC);
