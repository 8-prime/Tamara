-- Preserve shared classroom records while removing deleted teachers from attribution.
-- initialize() disables foreign-key enforcement only during transactional migrations.

CREATE TABLE attendance_new (
 classroom_id TEXT NOT NULL,
 student_id TEXT NOT NULL,
 attendance_date TEXT NOT NULL CHECK(attendance_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
 status TEXT NOT NULL CHECK(status IN ('present','absent')),
 updated_by TEXT REFERENCES teachers(id) ON DELETE SET NULL,
 updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
 PRIMARY KEY(classroom_id,attendance_date,student_id),
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

INSERT INTO attendance_new (classroom_id,student_id,attendance_date,status,updated_by,updated_at) SELECT classroom_id,student_id,attendance_date,status,updated_by,updated_at FROM attendance;

DROP TABLE attendance;

ALTER TABLE attendance_new RENAME TO attendance;

CREATE TABLE class_task_pupils_new (
 task_id TEXT NOT NULL, classroom_id TEXT NOT NULL, student_id TEXT NOT NULL,
 completed_at INTEGER, updated_by TEXT REFERENCES teachers(id) ON DELETE SET NULL,
 PRIMARY KEY(task_id,student_id),
 FOREIGN KEY(task_id,classroom_id) REFERENCES class_tasks(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

INSERT INTO class_task_pupils_new (task_id,classroom_id,student_id,completed_at,updated_by) SELECT task_id,classroom_id,student_id,completed_at,updated_by FROM class_task_pupils;

DROP TABLE class_task_pupils;

ALTER TABLE class_task_pupils_new RENAME TO class_task_pupils;

CREATE TABLE class_tasks_new (
 id TEXT PRIMARY KEY, classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 lesson_id TEXT,
 title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
 completion_mode TEXT NOT NULL CHECK(completion_mode IN ('once','per_pupil')),
 due_date TEXT, completed_at INTEGER,
 created_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(id,classroom_id),
 FOREIGN KEY(lesson_id,classroom_id) REFERENCES lessons(id,classroom_id)
);

INSERT INTO class_tasks_new (id,classroom_id,lesson_id,title,description,completion_mode,due_date,completed_at,created_by,created_at) SELECT id,classroom_id,lesson_id,title,description,completion_mode,due_date,completed_at,created_by,created_at FROM class_tasks;

DROP TABLE class_tasks;

ALTER TABLE class_tasks_new RENAME TO class_tasks;

CREATE INDEX class_tasks_class_due ON class_tasks(classroom_id,due_date);

CREATE TABLE homework_new (
 id TEXT PRIMARY KEY,
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 subject_id TEXT NOT NULL,
 title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '', due_date TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','auto_closed','manually_closed')),
 closed_at INTEGER,
 created_by TEXT REFERENCES teachers(id) ON DELETE SET NULL,
 created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(id,classroom_id),
 FOREIGN KEY(subject_id,classroom_id) REFERENCES subjects(id,classroom_id)
);

INSERT INTO homework_new (id,classroom_id,subject_id,title,description,due_date,status,closed_at,created_by,created_at) SELECT id,classroom_id,subject_id,title,description,due_date,status,closed_at,created_by,created_at FROM homework;

DROP TABLE homework;

ALTER TABLE homework_new RENAME TO homework;

CREATE INDEX homework_classroom ON homework(classroom_id,status,due_date);

CREATE TABLE homework_submissions_new (
 homework_id TEXT NOT NULL, classroom_id TEXT NOT NULL, student_id TEXT NOT NULL,
 handed_in_at INTEGER, updated_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, quality TEXT
 CHECK(quality IN ('all_correct','partially_correct','missing','all_incorrect')), note TEXT NOT NULL DEFAULT ''
 CHECK(length(note) <= 10000),
 PRIMARY KEY(homework_id,student_id),
 FOREIGN KEY(homework_id,classroom_id) REFERENCES homework(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

INSERT INTO homework_submissions_new (homework_id,classroom_id,student_id,handed_in_at,updated_by,quality,note) SELECT homework_id,classroom_id,student_id,handed_in_at,updated_by,quality,note FROM homework_submissions;

DROP TABLE homework_submissions;

ALTER TABLE homework_submissions_new RENAME TO homework_submissions;

CREATE INDEX homework_submissions_student ON homework_submissions(student_id,classroom_id);

CREATE TABLE lesson_performances_new (
 lesson_id TEXT NOT NULL, classroom_id TEXT NOT NULL, student_id TEXT NOT NULL,
 rating INTEGER NOT NULL CHECK(rating BETWEEN 1 AND 4), observation TEXT NOT NULL DEFAULT '',
 updated_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
 PRIMARY KEY(lesson_id,student_id),
 FOREIGN KEY(lesson_id,classroom_id) REFERENCES lessons(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

INSERT INTO lesson_performances_new (lesson_id,classroom_id,student_id,rating,observation,updated_by,updated_at) SELECT lesson_id,classroom_id,student_id,rating,observation,updated_by,updated_at FROM lesson_performances;

DROP TABLE lesson_performances;

ALTER TABLE lesson_performances_new RENAME TO lesson_performances;

CREATE TABLE lessons_new (
 id TEXT PRIMARY KEY,
 timetable_entry_id TEXT REFERENCES timetable_entries(id),
 teacher_id TEXT REFERENCES teachers(id) ON DELETE SET NULL,
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

INSERT INTO lessons_new (id,timetable_entry_id,teacher_id,classroom_id,subject_id,scheduled_date,lesson_date,starts_at,ends_at,room,topic,learning_goals,status,created_at) SELECT id,timetable_entry_id,teacher_id,classroom_id,subject_id,scheduled_date,lesson_date,starts_at,ends_at,room,topic,learning_goals,status,created_at FROM lessons;

DROP TABLE lessons;

ALTER TABLE lessons_new RENAME TO lessons;

CREATE INDEX lessons_teacher_date ON lessons(teacher_id,lesson_date,starts_at);

CREATE TABLE math_assessments_new (
 classroom_id TEXT NOT NULL, period_id TEXT NOT NULL, student_id TEXT NOT NULL,
 criterion_id TEXT NOT NULL REFERENCES math_criteria(id),
 value TEXT NOT NULL CHECK(value IN ('yes','no')),
 updated_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, updated_at INTEGER NOT NULL DEFAULT (unixepoch()), achieved_on TEXT
 CHECK(achieved_on IS NULL OR (value='yes' AND achieved_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')),
 PRIMARY KEY(period_id,student_id,criterion_id),
 FOREIGN KEY(period_id,classroom_id) REFERENCES periods(id,classroom_id) ON DELETE CASCADE,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

INSERT INTO math_assessments_new (classroom_id,period_id,student_id,criterion_id,value,updated_by,updated_at,achieved_on) SELECT classroom_id,period_id,student_id,criterion_id,value,updated_by,updated_at,achieved_on FROM math_assessments;

DROP TABLE math_assessments;

ALTER TABLE math_assessments_new RENAME TO math_assessments;

CREATE TABLE periods_new (
 id TEXT PRIMARY KEY, classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 name TEXT NOT NULL, source_period_id TEXT REFERENCES periods(id) ON DELETE SET NULL,
 created_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 UNIQUE(classroom_id,name), UNIQUE(id,classroom_id)
);

INSERT INTO periods_new (id,classroom_id,name,source_period_id,created_by,created_at) SELECT id,classroom_id,name,source_period_id,created_by,created_at FROM periods;

DROP TABLE periods;

ALTER TABLE periods_new RENAME TO periods;

CREATE TABLE registration_tokens_new (
 id TEXT PRIMARY KEY, label TEXT NOT NULL, token_hash TEXT NOT NULL UNIQUE,
 created_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 expires_at INTEGER NOT NULL, used_by TEXT REFERENCES teachers(id) ON DELETE SET NULL, used_at INTEGER,
 revoked INTEGER NOT NULL DEFAULT 0
);

INSERT INTO registration_tokens_new (id,label,token_hash,created_by,created_at,expires_at,used_by,used_at,revoked) SELECT id,label,token_hash,created_by,created_at,expires_at,used_by,used_at,revoked FROM registration_tokens;

DROP TABLE registration_tokens;

ALTER TABLE registration_tokens_new RENAME TO registration_tokens;

CREATE TABLE student_notes_new (
 id TEXT PRIMARY KEY,
 classroom_id TEXT NOT NULL,
 student_id TEXT NOT NULL,
 content TEXT NOT NULL CHECK(length(trim(content)) BETWEEN 1 AND 10000),
 created_by TEXT REFERENCES teachers(id) ON DELETE SET NULL,
 created_at INTEGER NOT NULL DEFAULT (unixepoch()),
 updated_by TEXT REFERENCES teachers(id) ON DELETE SET NULL,
 updated_at INTEGER NOT NULL DEFAULT (unixepoch()), lesson_id TEXT REFERENCES lessons(id) ON DELETE SET NULL,
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);

INSERT INTO student_notes_new (id,classroom_id,student_id,content,created_by,created_at,updated_by,updated_at,lesson_id) SELECT id,classroom_id,student_id,content,created_by,created_at,updated_by,updated_at,lesson_id FROM student_notes;

DROP TABLE student_notes;

ALTER TABLE student_notes_new RENAME TO student_notes;

CREATE INDEX student_notes_class_date ON student_notes(classroom_id,created_at DESC);

CREATE TABLE timetable_entries_new (
 id TEXT PRIMARY KEY,
 teacher_id TEXT REFERENCES teachers(id) ON DELETE SET NULL,
 classroom_id TEXT NOT NULL REFERENCES classrooms(id) ON DELETE CASCADE,
 subject_id TEXT NOT NULL,
 weekday INTEGER NOT NULL CHECK(weekday BETWEEN 0 AND 6),
 starts_at TEXT NOT NULL, ends_at TEXT NOT NULL, room TEXT NOT NULL DEFAULT '',
 valid_from TEXT NOT NULL, valid_until TEXT NOT NULL,
 UNIQUE(id,classroom_id),
 FOREIGN KEY(subject_id,classroom_id) REFERENCES subjects(id,classroom_id)
);

INSERT INTO timetable_entries_new (id,teacher_id,classroom_id,subject_id,weekday,starts_at,ends_at,room,valid_from,valid_until) SELECT id,teacher_id,classroom_id,subject_id,weekday,starts_at,ends_at,room,valid_from,valid_until FROM timetable_entries;

DROP TABLE timetable_entries;

ALTER TABLE timetable_entries_new RENAME TO timetable_entries;

CREATE TABLE password_resets (
 token_hash TEXT PRIMARY KEY,
 teacher_id TEXT NOT NULL UNIQUE REFERENCES teachers(id) ON DELETE CASCADE,
 expires_at INTEGER NOT NULL
);
-- Fail the migration transaction if rebuilding left any broken references.
CREATE TEMP TABLE account_migration_integrity (violations INTEGER CHECK(violations=0));
INSERT INTO account_migration_integrity SELECT COUNT(*) FROM pragma_foreign_key_check;
DROP TABLE account_migration_integrity;
