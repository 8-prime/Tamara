-- Additive: existing attendance, homework, notes and assessments retain their records.
CREATE TABLE timetable_entries (
 id TEXT PRIMARY KEY,
 teacher_id TEXT NOT NULL REFERENCES teachers(id),
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 subject_id TEXT NOT NULL,
 weekday INTEGER NOT NULL CHECK(weekday BETWEEN 0 AND 6),
 starts_at TEXT NOT NULL, ends_at TEXT NOT NULL, room TEXT NOT NULL DEFAULT '',
 valid_from TEXT NOT NULL, valid_until TEXT NOT NULL,
 UNIQUE(id,classroom_id),
 FOREIGN KEY(subject_id,classroom_id) REFERENCES subjects(id,classroom_id)
);
CREATE TABLE lessons (
 id TEXT PRIMARY KEY,
 timetable_entry_id TEXT REFERENCES timetable_entries(id),
 teacher_id TEXT NOT NULL REFERENCES teachers(id),
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 subject_id TEXT NOT NULL,
 scheduled_date TEXT NOT NULL, lesson_date TEXT NOT NULL,
 starts_at TEXT NOT NULL, ends_at TEXT NOT NULL, room TEXT NOT NULL DEFAULT '',
 topic TEXT NOT NULL DEFAULT '', learning_goals TEXT NOT NULL DEFAULT '',
 status TEXT NOT NULL DEFAULT 'scheduled' CHECK(status IN ('scheduled','cancelled')),
 created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(id,classroom_id), UNIQUE(timetable_entry_id,scheduled_date),
 FOREIGN KEY(subject_id,classroom_id) REFERENCES subjects(id,classroom_id)
);
CREATE INDEX lessons_teacher_date ON lessons(teacher_id,lesson_date,starts_at);
CREATE TABLE lesson_performances (
 lesson_id TEXT NOT NULL, classroom_id TEXT NOT NULL, student_id TEXT NOT NULL,
 rating INTEGER NOT NULL CHECK(rating BETWEEN 1 AND 4), observation TEXT NOT NULL DEFAULT '',
 updated_by TEXT NOT NULL REFERENCES teachers(id), updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
 PRIMARY KEY(lesson_id,student_id),
 FOREIGN KEY(lesson_id,classroom_id) REFERENCES lessons(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);
CREATE TABLE class_tasks (
 id TEXT PRIMARY KEY, classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 lesson_id TEXT,
 title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
 completion_mode TEXT NOT NULL CHECK(completion_mode IN ('once','per_pupil')),
 due_date TEXT, completed_at INTEGER,
 created_by TEXT NOT NULL REFERENCES teachers(id), created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(id,classroom_id),
 FOREIGN KEY(lesson_id,classroom_id) REFERENCES lessons(id,classroom_id)
);
CREATE INDEX class_tasks_class_due ON class_tasks(classroom_id,due_date);
CREATE TABLE class_task_pupils (
 task_id TEXT NOT NULL, classroom_id TEXT NOT NULL, student_id TEXT NOT NULL,
 completed_at INTEGER, updated_by TEXT REFERENCES teachers(id),
 PRIMARY KEY(task_id,student_id),
 FOREIGN KEY(task_id,classroom_id) REFERENCES class_tasks(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);
ALTER TABLE student_notes ADD COLUMN lesson_id TEXT REFERENCES lessons(id) ON DELETE SET NULL;
