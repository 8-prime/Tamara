import { test } from "node:test";
import assert from "node:assert/strict";
import { recentMissing, performancePoints } from "./pupil-history.ts";

test("recent missing homework excludes today, future, older and submitted work", () => {
  const homework = [
    { id: "boundary", due_date: "2026-10-20", handed_in_at: null },
    { id: "yesterday", due_date: "2026-11-02", handed_in_at: null },
    { id: "old", due_date: "2026-10-19", handed_in_at: null },
    { id: "today", due_date: "2026-11-03", handed_in_at: null },
    { id: "future", due_date: "2026-11-04", handed_in_at: null },
    { id: "submitted", due_date: "2026-11-02", handed_in_at: 0 },
  ];
  assert.deepEqual(
    recentMissing(homework, "2026-11-03").map((h) => h.id),
    ["boundary", "yesterday"],
  );
  assert.deepEqual(recentMissing([], "2026-11-03"), []);
});

test("chart positions preserve date gaps, rating scale and chronological order", () => {
  const records = [
    { lesson_id: "c", date: "2026-11-11", time: "08:00", rating: 4 },
    { lesson_id: "a", date: "2026-11-01", time: "08:00", rating: 1 },
    { lesson_id: "b", date: "2026-11-02", time: "08:00", rating: 2 },
  ];
  assert.deepEqual(
    performancePoints(records).map(({ x, y }) => [x, y]),
    [
      [60, 210],
      [120, 150],
      [660, 30],
    ],
  );
  assert.equal(records[0]?.lesson_id, "c");
  assert.equal(performancePoints(records.slice(0, 1))[0]?.x, 360);
  assert.deepEqual(performancePoints([]), []);
});

test("class averages group by lesson without filling in missing ratings", async () => {
  const { classLessonAverages } = await import("./pupil-history.ts");
  const records = [
    {
      lesson_id: "one",
      student_id: "a",
      subject_id: "math",
      date: "2026-11-01",
      time: "08:00",
      rating: 1,
    },
    {
      lesson_id: "one",
      student_id: "b",
      subject_id: "math",
      date: "2026-11-01",
      time: "08:00",
      rating: 4,
    },
    {
      lesson_id: "two",
      student_id: "a",
      subject_id: "german",
      date: "2026-11-01",
      time: "09:00",
      rating: 3,
    },
  ];
  const lessons = classLessonAverages(records);
  assert.deepEqual(
    lessons.map((r) => [r.lesson_id, r.rating, r.count]),
    [
      ["one", 2.5, 2],
      ["two", 3, 1],
    ],
  );
  assert.equal(lessons[0]?.subject_id, "math");
  assert.deepEqual(classLessonAverages([]), []);
});

test("class summaries keep duplicate names separate and do not count future homework as missing", async () => {
  const { classPupilSummaries, filterPupilHistory } =
    await import("./pupil-history.ts");
  const students = [
    { id: "a", name: "Alex" },
    { id: "b", name: "Alex" },
  ];
  const performances = [
    { student_id: "a", subject_id: "math", date: "2026-11-01", rating: 2 },
    { student_id: "a", subject_id: "math", date: "2026-11-02", rating: 4 },
    { student_id: "a", subject_id: "german", date: "2026-10-01", rating: 1 },
  ];
  const homework = [
    {
      student_id: "a",
      subject_id: "math",
      due_date: "2026-11-01",
      handed_in_at: 123,
    },
    {
      student_id: "a",
      subject_id: "math",
      due_date: "2026-11-02",
      handed_in_at: null,
    },
    {
      student_id: "a",
      subject_id: "math",
      due_date: "2026-11-03",
      handed_in_at: null,
    },
    {
      student_id: "a",
      subject_id: "math",
      due_date: "2026-11-04",
      handed_in_at: null,
    },
  ];
  const filters = { subject: "math", from: "2026-11-01", until: "2026-11-04" };
  const rows = classPupilSummaries(
    students,
    filterPupilHistory(performances, filters),
    filterPupilHistory(homework, filters),
    "2026-11-03",
  );
  assert.deepEqual(rows[0], {
    id: "a",
    name: "Alex",
    count: 2,
    latest: "2026-11-02",
    average: 3,
    due: 2,
    submitted: 1,
    missing: 1,
  });
  assert.deepEqual(rows[1], {
    id: "b",
    name: "Alex",
    count: 0,
    latest: "",
    average: null,
    due: 0,
    submitted: 0,
    missing: 0,
  });
  assert.deepEqual(
    filterPupilHistory(performances, {
      from: "2026-11-02",
      until: "2026-11-01",
    }),
    [],
  );
});

test("chart separates multiple lessons on the same day", () => {
  const records = [
    { lesson_id: "early", date: "2026-11-03", time: "08:00", rating: 2 },
    { lesson_id: "late", date: "2026-11-03", time: "10:00", rating: 3 },
  ];
  assert.deepEqual(
    performancePoints(records).map((p) => p.x),
    [60, 660],
  );
});
