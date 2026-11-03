export function recentMissing(homework, today) {
  const start = new Date(`${today}T12:00:00Z`);
  start.setUTCDate(start.getUTCDate() - 14);
  const from = start.toISOString().slice(0, 10);
  // Today's assignments are not overdue until the following day.
  return homework.filter(
    (h) => h.handed_in_at == null && h.due_date >= from && h.due_date < today,
  );
}

export function performancePoints(records) {
  const ordered = records.toSorted(
    (a, b) =>
      (a.date + a.time).localeCompare(b.date + b.time) ||
      a.lesson_id.localeCompare(b.lesson_id),
  );
  if (!ordered.length) return [];
  const timestamp = (record) => Date.parse(`${record.date}T${record.time}:00Z`);
  const first = timestamp(ordered[0]);
  const span = timestamp(ordered.at(-1)) - first;
  return ordered.map((record) => ({
    ...record,
    x: span ? 60 + ((timestamp(record) - first) / span) * 600 : 360,
    y: 210 - ((record.rating - 1) / 3) * 180,
  }));
}

export function filterPupilHistory(
  records,
  { subject = "", from = "", until = "" } = {},
) {
  return records.filter((record) => {
    const date = record.date || record.due_date;
    return (
      (!subject || record.subject_id === subject) &&
      (!from || date >= from) &&
      (!until || date <= until)
    );
  });
}

// Each point describes one lesson; unrated children never count as zero.
export function classLessonAverages(records) {
  const lessons = new Map();
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

export function classPupilSummaries(students, performances, homework, today) {
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
