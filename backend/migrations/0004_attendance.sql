CREATE TABLE attendance (
 classroom_id TEXT NOT NULL,
 student_id TEXT NOT NULL,
 attendance_date TEXT NOT NULL CHECK(attendance_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
 status TEXT NOT NULL CHECK(status IN ('present','absent')),
 updated_by TEXT NOT NULL REFERENCES teachers(id),
 updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
 PRIMARY KEY(classroom_id,attendance_date,student_id),
 FOREIGN KEY(student_id,classroom_id) REFERENCES students(id,classroom_id) ON DELETE CASCADE
);
