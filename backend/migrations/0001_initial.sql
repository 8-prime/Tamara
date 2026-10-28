CREATE TABLE teachers (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL UNIQUE COLLATE NOCASE,
 password_hash TEXT NOT NULL, created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE sessions (
 token_hash TEXT PRIMARY KEY, teacher_id TEXT NOT NULL REFERENCES teachers(id) ON DELETE CASCADE,
 expires_at INTEGER NOT NULL
);
CREATE INDEX sessions_teacher ON sessions(teacher_id);
CREATE TABLE classrooms (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, school_year TEXT NOT NULL,
 created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE classroom_teachers (
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 teacher_id TEXT NOT NULL REFERENCES teachers(id) ON DELETE CASCADE,
 role TEXT NOT NULL CHECK(role IN ('owner','editor')), PRIMARY KEY(classroom_id,teacher_id)
);
CREATE TABLE students (
 id TEXT PRIMARY KEY, classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 name TEXT NOT NULL, UNIQUE(id,classroom_id)
);
CREATE INDEX students_classroom ON students(classroom_id);
CREATE TABLE periods (
 id TEXT PRIMARY KEY, classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 name TEXT NOT NULL, source_period_id TEXT REFERENCES periods(id) ON DELETE SET NULL,
 created_by TEXT NOT NULL REFERENCES teachers(id), created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(classroom_id,name), UNIQUE(id,classroom_id)
);
CREATE TABLE math_criteria (
 id TEXT PRIMARY KEY, label TEXT NOT NULL, domain TEXT NOT NULL, section TEXT NOT NULL,
 level TEXT NOT NULL, page INTEGER NOT NULL, x REAL NOT NULL, y REAL NOT NULL
);
CREATE TABLE math_assessments (
 classroom_id TEXT NOT NULL, period_id TEXT NOT NULL, student_id TEXT NOT NULL,
 criterion_id TEXT NOT NULL REFERENCES math_criteria(id),
 value TEXT NOT NULL CHECK(value IN ('yes','no')),
 updated_by TEXT NOT NULL REFERENCES teachers(id), updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
 PRIMARY KEY(period_id,student_id,criterion_id),
 FOREIGN KEY(period_id,classroom_id) REFERENCES periods(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);
