export function performanceLessons<T extends { id: string; subject_id: string; lesson_date: string; starts_at: string; performance_count: number; classroom_id?: string; status?: string; subject_name?: string; class_name?: string; topic?: string; ends_at?: string; pupil_count?: number }>(
  lessons: T[],
  { subject = "", from = "", until = "", recordedOnly = true } = {},
) {
  return lessons
    .filter(
      (lesson) =>
        (!recordedOnly || lesson.performance_count > 0) &&
        (!subject || lesson.subject_id === subject) &&
        (!from || lesson.lesson_date >= from) &&
        (!until || lesson.lesson_date <= until),
    )
    .toSorted(
      (a, b) =>
        (b.lesson_date + b.starts_at).localeCompare(
          a.lesson_date + a.starts_at,
        ) || a.id.localeCompare(b.id),
    );
}

export function pupilPerformances<S extends { id: string; name: string }, R extends { student_id: string }>(
  students: S[],
  records: R[],
  search = "",
  recordedOnly = false,
) {
  const byPupil = new Map(records.map((record) => [record.student_id, record]));
  const query = search.trim().toLocaleLowerCase("de");
  return students
    .filter((student) => student.name.toLocaleLowerCase("de").includes(query))
    .map((student) => ({
      ...student,
      performance: byPupil.get(student.id) || null,
    }))
    .filter((student) => !recordedOnly || student.performance);
}
