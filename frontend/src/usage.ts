import { addDays, dateLabel, monday } from "./teaching.ts";

export type Bucket = "day" | "week" | "month";
export interface Range {
  from: string;
  until: string;
  bucket: Bucket;
}
export interface Metric {
  id: string;
  label: string;
  unit?: string;
}

/** Metrics as returned by GET /api/admin/usage, grouped for display. */
export const METRIC_GROUPS: { label: string; metrics: Metric[] }[] = [
  {
    label: "Nutzung",
    metrics: [
      { id: "teacher_active", label: "Aktive Lehrkräfte", unit: "Ø pro Tag" },
      { id: "session_started", label: "Anmeldungen" },
      { id: "teacher_registered", label: "Neue Lehrkräfte" },
    ],
  },
  {
    label: "Klassen",
    metrics: [
      { id: "classroom_created", label: "Klassen angelegt" },
      { id: "pupil_added", label: "Kinder hinzugefügt" },
    ],
  },
  {
    label: "Unterricht",
    metrics: [
      { id: "lesson_created", label: "Stunden geplant" },
      { id: "attendance_taken", label: "Anwesenheitslisten" },
      { id: "performance_recorded", label: "Leistungen erfasst" },
      { id: "performance_corrected", label: "Leistungen korrigiert" },
    ],
  },
  {
    label: "Notizen",
    metrics: [
      { id: "note_created", label: "Notizen geschrieben" },
      { id: "note_edited", label: "Notizen bearbeitet" },
    ],
  },
  {
    label: "Hausaufgaben & Aufgaben",
    metrics: [
      { id: "homework_created", label: "Hausaufgaben gestellt" },
      { id: "homework_handed_in", label: "Abgaben erfasst" },
      { id: "homework_rated", label: "Abgaben bewertet" },
      { id: "worksheet_uploaded", label: "Arbeitsblätter hochgeladen" },
      { id: "task_created", label: "Klassenaufgaben angelegt" },
      { id: "task_completed", label: "Aufgaben erledigt" },
    ],
  },
  {
    label: "Lernentwicklung",
    metrics: [
      { id: "period_created", label: "Zeiträume angelegt" },
      { id: "assessment_recorded", label: "Kompetenzen eingeschätzt" },
      { id: "pdf_exported", label: "PDF-Bögen erstellt" },
      { id: "zip_exported", label: "Klassen-ZIPs erstellt" },
    ],
  },
];

export const METRICS = METRIC_GROUPS.flatMap((g) => g.metrics);

export const PRESETS = [
  { id: "7d", label: "7 Tage" },
  { id: "30d", label: "30 Tage" },
  { id: "12w", label: "12 Wochen" },
  { id: "12m", label: "12 Monate" },
  { id: "all", label: "Seit Beginn" },
];

export const BUCKETS: { id: Bucket; label: string; per: string }[] = [
  { id: "day", label: "Tag", per: "je Tag" },
  { id: "week", label: "Woche", per: "je Woche" },
  { id: "month", label: "Monat", per: "je Monat" },
];

/** The finest bucket that keeps the chart readable for a range. */
export function autoBucket(from: string, until: string): Bucket {
  const days =
    (new Date(until + "T12:00:00").getTime() - new Date(from + "T12:00:00").getTime()) /
      864e5 +
    1;
  return days <= 31 ? "day" : days <= 26 * 7 ? "week" : "month";
}

/** Range and bucket for a preset; `since` is the first counted day. */
export function presetRange(id: string, today: string, since: string): Range {
  if (id === "7d") return { from: addDays(today, -6), until: today, bucket: "day" };
  if (id === "30d")
    return { from: addDays(today, -29), until: today, bucket: "day" };
  if (id === "12w")
    return { from: addDays(monday(today), -77), until: today, bucket: "week" };
  if (id === "12m") {
    const d = new Date(today + "T12:00:00");
    const start = new Date(d.getFullYear(), d.getMonth() - 11, 1, 12);
    const from = `${start.getFullYear()}-${String(start.getMonth() + 1).padStart(2, "0")}-01`;
    return { from, until: today, bucket: "month" };
  }
  return { from: since, until: today, bucket: autoBucket(since, today) };
}

export function usagePath({ from, until, bucket }: Range) {
  const query = new URLSearchParams({ bucket });
  if (from) query.set("from", from);
  if (until) query.set("until", until);
  return "/admin/usage?" + query;
}

/**
 * Active teachers cannot be summed across days (the same person is active on
 * many days), so a bucket shows the daily average. Everything else is a sum.
 */
export function metricValue(
  metric: string,
  counts: Record<string, number> | undefined,
  days: number,
) {
  const n = counts?.[metric] ?? 0;
  if (metric !== "teacher_active") return n;
  return days ? Math.round((n / days) * 10) / 10 : 0;
}

export function formatValue(value: number) {
  return value.toLocaleString("de-DE", { maximumFractionDigits: 1 });
}

export function isoWeek(day: string) {
  const d = new Date(day + "T12:00:00");
  const thursday = new Date(d);
  thursday.setDate(d.getDate() - ((d.getDay() + 6) % 7) + 3);
  const firstThursday = new Date(thursday.getFullYear(), 0, 4, 12);
  firstThursday.setDate(
    firstThursday.getDate() - ((firstThursday.getDay() + 6) % 7) + 3,
  );
  return 1 + Math.round((thursday.getTime() - firstThursday.getTime()) / (7 * 864e5));
}

export function periodLabel(start: string, bucket: Bucket, long = false) {
  if (bucket === "week")
    return long
      ? `KW ${isoWeek(start)} · ab ${dateLabel(start, { day: "numeric", month: "long", year: "numeric" })}`
      : `KW ${isoWeek(start)}`;
  if (bucket === "month")
    return dateLabel(start, { month: long ? "long" : "short", year: "numeric" });
  return dateLabel(
    start,
    long
      ? { weekday: "short", day: "numeric", month: "long", year: "numeric" }
      : { day: "numeric", month: "numeric" },
  );
}

/** A round axis maximum (1, 2 or 5 × 10ⁿ) at or above `max`. */
export function niceMax(max: number) {
  if (!(max > 0)) return 1;
  const power = 10 ** Math.floor(Math.log10(max));
  return [1, 2, 5, 10].map((f) => f * power).find((v) => v >= max) ?? 10 * power;
}
