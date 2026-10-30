import { test } from "node:test";
import assert from "node:assert/strict";
import {
  performanceLessons,
  pupilPerformances,
} from "./performance-history.js";
const lessons = [
  {
    id: "old",
    lesson_date: "2024-05-02",
    starts_at: "08:00",
    subject_id: "math",
    performance_count: 2,
  },
  {
    id: "new",
    lesson_date: "2026-10-29",
    starts_at: "09:00",
    subject_id: "german",
    performance_count: 1,
  },
  {
    id: "empty",
    lesson_date: "2026-10-30",
    starts_at: "10:00",
    subject_id: "math",
    performance_count: 0,
  },
];
test("history defaults to saved lessons, newest first, without hiding older records", () => {
  assert.deepEqual(
    performanceLessons(lessons).map((l) => l.id),
    ["new", "old"],
  );
  assert.deepEqual(
    performanceLessons(lessons, { recordedOnly: false }).map((l) => l.id),
    ["empty", "new", "old"],
  );
});
test("history filters by subject and inclusive date range", () => {
  assert.deepEqual(
    performanceLessons(lessons, { subject: "math" }).map((l) => l.id),
    ["old"],
  );
  assert.deepEqual(
    performanceLessons(lessons, {
      from: "2026-10-29",
      until: "2026-10-29",
    }).map((l) => l.id),
    ["new"],
  );
});
test("pupil results preserve duplicate names and distinguish unrecorded pupils", () => {
  const pupils = [
    { id: "one", name: "Anna Becker" },
    { id: "two", name: "Anna Becker" },
  ];
  const records = [
    {
      student_id: "two",
      rating: 3,
      observation: "Begründet ihren Lösungsweg.",
    },
  ];
  const rows = pupilPerformances(pupils, records, "  ANNA ");
  assert.equal(rows[0].performance, null);
  assert.equal(rows[1].performance.rating, 3);
  assert.equal(rows[1].performance.observation, "Begründet ihren Lösungsweg.");
  assert.deepEqual(
    pupilPerformances(pupils, records, "", true).map((p) => p.id),
    ["two"],
  );
});
