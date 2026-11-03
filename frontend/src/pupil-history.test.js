import { test } from "node:test";
import assert from "node:assert/strict";
import { recentMissing, performancePoints } from "./pupil-history.js";

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
  assert.equal(records[0].lesson_id, "c");
  assert.equal(performancePoints([records[0]])[0].x, 360);
  assert.deepEqual(performancePoints([]), []);
});
