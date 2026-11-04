CREATE TABLE parent_conferences (
 id TEXT PRIMARY KEY,
 teacher_id TEXT NOT NULL REFERENCES teachers(id) ON DELETE CASCADE,
 student_id TEXT NOT NULL REFERENCES students(id) ON DELETE CASCADE,
 date TEXT NOT NULL,
 content TEXT NOT NULL DEFAULT '',
 attendees TEXT NOT NULL DEFAULT '[]',
 review_snapshot TEXT,
 updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX parent_conferences_teacher_date ON parent_conferences(teacher_id,date DESC);
CREATE TABLE parent_conference_documents (
 id TEXT PRIMARY KEY,
 conference_id TEXT NOT NULL REFERENCES parent_conferences(id) ON DELETE CASCADE,
 name TEXT NOT NULL,
 content BLOB NOT NULL,
 created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX parent_conference_documents_conference ON parent_conference_documents(conference_id);
