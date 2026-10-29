CREATE TABLE subjects (
 id TEXT PRIMARY KEY,
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 name TEXT NOT NULL COLLATE NOCASE,
 UNIQUE(classroom_id,name), UNIQUE(id,classroom_id)
);

CREATE TABLE homework (
 id TEXT PRIMARY KEY,
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 subject_id TEXT NOT NULL,
 title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', due_date TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','auto_closed','manually_closed')),
 closed_at INTEGER,
 created_by TEXT NOT NULL REFERENCES teachers(id),
 created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(id,classroom_id),
 FOREIGN KEY(subject_id,classroom_id) REFERENCES subjects(id,classroom_id)
);
CREATE INDEX homework_classroom ON homework(classroom_id,status,due_date);

-- The assigned pupils are a snapshot, independent of future roster additions.
CREATE TABLE homework_submissions (
 homework_id TEXT NOT NULL, classroom_id TEXT NOT NULL, student_id TEXT NOT NULL,
 handed_in_at INTEGER, updated_by TEXT REFERENCES teachers(id),
 PRIMARY KEY(homework_id,student_id),
 FOREIGN KEY(homework_id,classroom_id) REFERENCES homework(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);
CREATE INDEX homework_submissions_student ON homework_submissions(student_id,classroom_id);

CREATE TABLE homework_worksheets (
 homework_id TEXT PRIMARY KEY REFERENCES homework(id) ON DELETE CASCADE,
 name TEXT NOT NULL, content BLOB NOT NULL
);
