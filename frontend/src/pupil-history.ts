export function recentMissing<T extends { handed_in_at?: string | number | null; due_date: string }>(homework: T[], today: string) {
  const start = new Date(`${today}T12:00:00Z`);
  start.setUTCDate(start.getUTCDate() - 14);
  const from = start.toISOString().slice(0, 10);
  // Today's assignments are not overdue until the following day.
  return homework.filter(
    (h) => h.handed_in_at == null && h.due_date >= from && h.due_date < today,
  );
}

export function performancePoints<T extends { date: string; time: string; lesson_id: string; rating: number; subject_id?: string; subject_name?: string; count?: number; observation?: string }>(records: T[]) {
  const ordered = records.toSorted(
    (a, b) =>
      (a.date + a.time).localeCompare(b.date + b.time) ||
      a.lesson_id.localeCompare(b.lesson_id),
  );
  const firstRecord = ordered[0], lastRecord = ordered.at(-1);
  if (!firstRecord || !lastRecord) return [];
  const timestamp = (record: T) => Date.parse(`${record.date}T${record.time}:00Z`);
  const first = timestamp(firstRecord);
  const span = timestamp(lastRecord) - first;
  return ordered.map((record) => ({
    ...record,
    x: span ? 60 + ((timestamp(record) - first) / span) * 600 : 360,
    y: 210 - ((record.rating - 1) / 3) * 180,
  }));
}

export function filterPupilHistory<T extends { date?: string; due_date?: string; subject_id: string }>(
  records: T[],
  { subject = "", from = "", until = "" } = {},
) {
  return records.filter((record) => {
    const date = record.date || record.due_date;
    return (
      (!subject || record.subject_id === subject) &&
      (!from || (date !== undefined && date >= from)) &&
      (!until || (date !== undefined && date <= until))
    );
  });
}

// Each point describes one lesson; unrated children never count as zero.
export function classLessonAverages<T extends {lesson_id: string; rating: number}>(records: T[]) {
  const lessons = new Map<string, T & {count: number; total: number; observation: string}>();
  for (const record of records) {
    let lesson = lessons.get(record.lesson_id);
    if (!lesson) {
      lesson = { ...record, count: 0, total: 0, observation: "" };
      lessons.set(record.lesson_id, lesson);
    }
    lesson.total += record.rating;
    lesson.count += 1;
  }
  return [...lessons.values()].map(({ total, ...lesson }) => ({
    ...lesson,
    rating: total / lesson.count,
  }));
}

export function classPupilSummaries<S extends {id: string}, P extends {student_id: string; date: string; rating: number}, H extends {student_id: string; due_date: string; handed_in_at?: number | string | null}>(students: S[], performances: P[], homework: H[], today: string) {
  return students.map((student) => {
    const records = performances.filter((r) => r.student_id === student.id);
    const assignments = homework.filter((r) => r.student_id === student.id);
    // Only overdue assignments enter the submission rate; today's work is still open.
    const due = assignments.filter((r) => r.due_date < today);
    return {
      ...student,
      count: records.length,
      latest:
        records
          .map((r) => r.date)
          .toSorted()
          .at(-1) || "",
      average: records.length
        ? records.reduce((sum, r) => sum + r.rating, 0) / records.length
        : null,
      due: due.length,
      submitted: due.filter((r) => r.handed_in_at != null).length,
      missing: recentMissing(assignments, today).length,
    };
  });
}
