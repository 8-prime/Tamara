export function dateKey(d = new Date()) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
export function addDays(day, amount) {
  const d = new Date(day + "T12:00:00");
  d.setDate(d.getDate() + amount);
  return dateKey(d);
}
export function monday(day) {
  const d = new Date(day + "T12:00:00");
  return addDays(day, -(d.getDay() + 6) % 7);
}
export function dateLabel(
  day,
  options = { day: "numeric", month: "long", year: "numeric" },
) {
  return new Date(day + "T12:00:00").toLocaleDateString("de-DE", options);
}
export function initials(name = "") {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((n) => n[0])
    .join("")
    .toUpperCase();
}
export function subjectStyle(name = "") {
  if (/deutsch|englisch/i.test(name)) return "blue";
  if (/musik|kunst/i.test(name)) return "rose";
  if (/sport/i.test(name)) return "orange";
  if (/religion|förder|rat/i.test(name)) return "purple";
  return "green";
}
export function currentOrNext(lessons, now = new Date()) {
  const active = lessons
    .filter((l) => l.status === "scheduled")
    .toSorted((a, b) =>
      (a.lesson_date + a.starts_at).localeCompare(b.lesson_date + b.starts_at),
    );
  return (
    active.find((l) => new Date(`${l.lesson_date}T${l.ends_at}`) > now) || null
  );
}
export function lessonAction(lesson, now = new Date()) {
  if (!lesson)
    return { kind: "create", label: "Stundenplan ergänzen", icon: "calendar" };
  const start = new Date(`${lesson.lesson_date}T${lesson.starts_at}`);
  const end = new Date(`${lesson.lesson_date}T${lesson.ends_at}`);
  if (now < start)
    return { kind: "prepare", label: "Stunde vorbereiten", icon: "book" };
  if (now >= end)
    return { kind: "performance", label: "Leistungen ansehen", icon: "chart" };
  if ((end - now) / 60000 <= 5)
    return {
      kind: "performance",
      label:
        lesson.performance_count >= lesson.pupil_count && lesson.pupil_count
          ? "Leistungen ansehen"
          : "Leistung erfassen",
      icon: "star",
    };
  if (
    (now - start) / 60000 < 20 &&
    lesson.attendance_count < lesson.pupil_count
  )
    return {
      kind: "attendance",
      label: `Anwesenheit für ${lesson.class_name} erfassen`,
      icon: "users",
    };
  return { kind: "note", label: "Notiz zur Stunde erfassen", icon: "note" };
}
export function taskDone(task) {
  return task.completion_mode === "once"
    ? !!task.completed_at
    : task.assigned_count > 0 && task.completed_count >= task.assigned_count;
}
