-- School-wide usage statistics for administrators. Only daily totals per action
-- are stored: no teacher, class or pupil identifiers and no content. Counting
-- starts with this migration; existing records are deliberately not counted.
-- Days are UTC dates. Any timeframe is the sum of its days.
CREATE TABLE usage_counters (
 day TEXT NOT NULL CHECK(day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
 metric TEXT NOT NULL,
 count INTEGER NOT NULL CHECK(count >= 0),
 PRIMARY KEY(day, metric)
) WITHOUT ROWID;

-- Marks the first counted day, so the statistics can say since when they count.
INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'counting_started', 1);

-- Lets the first request of each UTC day count a teacher as active exactly once.
ALTER TABLE teachers ADD COLUMN last_active_day TEXT;

-- Triggers count in the same transaction as the write they describe, so counts
-- cannot drift from what actually happened. Assessments and exports are counted
-- by the application because they are copied in bulk or do not write records.
-- A later migration that rebuilds one of these tables (CREATE _new, DROP, RENAME)
-- drops its trigger as well and must recreate it.
CREATE TRIGGER usage_teacher_active AFTER UPDATE OF last_active_day ON teachers
WHEN NEW.last_active_day IS NOT OLD.last_active_day BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'teacher_active', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_teacher_registered AFTER INSERT ON teachers BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'teacher_registered', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_session_started AFTER INSERT ON sessions BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'session_started', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_classroom_created AFTER INSERT ON classrooms BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'classroom_created', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_pupil_added AFTER INSERT ON students BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'pupil_added', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_lesson_created AFTER INSERT ON lessons BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'lesson_created', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

-- One attendance list per class and date: count only its first entry.
CREATE TRIGGER usage_attendance_taken AFTER INSERT ON attendance
WHEN (SELECT COUNT(*) FROM attendance
      WHERE classroom_id=NEW.classroom_id AND attendance_date=NEW.attendance_date)=1 BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'attendance_taken', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_performance_recorded AFTER INSERT ON lesson_performances BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'performance_recorded', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_performance_corrected AFTER UPDATE ON lesson_performances
WHEN NEW.rating IS NOT OLD.rating OR NEW.observation IS NOT OLD.observation BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'performance_corrected', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_note_created AFTER INSERT ON student_notes BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'note_created', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_note_edited AFTER UPDATE OF content ON student_notes
WHEN NEW.content IS NOT OLD.content BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'note_edited', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_homework_created AFTER INSERT ON homework BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'homework_created', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_homework_handed_in AFTER UPDATE OF handed_in_at ON homework_submissions
WHEN OLD.handed_in_at IS NULL AND NEW.handed_in_at IS NOT NULL BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'homework_handed_in', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_homework_rated AFTER UPDATE OF quality ON homework_submissions
WHEN NEW.quality IS NOT NULL AND NEW.quality IS NOT OLD.quality BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'homework_rated', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_worksheet_uploaded AFTER INSERT ON homework_worksheets BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'worksheet_uploaded', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

-- Replacing an existing worksheet is an upload as well.
CREATE TRIGGER usage_worksheet_replaced AFTER UPDATE OF content ON homework_worksheets BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'worksheet_uploaded', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_task_created AFTER INSERT ON class_tasks BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'task_created', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_task_completed AFTER UPDATE OF completed_at ON class_tasks
WHEN OLD.completed_at IS NULL AND NEW.completed_at IS NOT NULL BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'task_completed', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_task_pupil_completed AFTER UPDATE OF completed_at ON class_task_pupils
WHEN OLD.completed_at IS NULL AND NEW.completed_at IS NOT NULL BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'task_completed', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;

CREATE TRIGGER usage_period_created AFTER INSERT ON periods BEGIN
 INSERT INTO usage_counters(day, metric, count) VALUES(date('now'), 'period_created', 1)
 ON CONFLICT(day, metric) DO UPDATE SET count=count+1;
END;
