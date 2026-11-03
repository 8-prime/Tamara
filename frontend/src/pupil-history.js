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
  const first = Date.parse(ordered[0].date);
  const span = Date.parse(ordered.at(-1).date) - first;
  return ordered.map((record) => ({
    ...record,
    x: span ? 60 + ((Date.parse(record.date) - first) / span) * 600 : 360,
    y: 210 - ((record.rating - 1) / 3) * 180,
  }));
}
