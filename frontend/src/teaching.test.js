import { test } from "node:test";
import assert from "node:assert/strict";
import {
  currentOrNext,
  lessonAction,
  taskDone,
  monday,
  addDays,
} from "./teaching.js";
const lesson = {
  id: "one",
  lesson_date: "2026-10-29",
  starts_at: "09:50",
  ends_at: "10:35",
  status: "scheduled",
  class_name: "2a",
  pupil_count: 24,
  attendance_count: 0,
  performance_count: 0,
};
test("suggestion follows the lesson clock and completed attendance", () => {
  for (const [time, kind] of [
    ["09:49", "prepare"],
    ["09:50", "attendance"],
    ["10:09", "attendance"],
    ["10:10", "note"],
    ["10:29", "note"],
    ["10:30", "performance"],
  ])
    assert.equal(
      lessonAction(lesson, new Date(`2026-10-29T${time}`)).kind,
      kind,
    );
  assert.equal(
    lessonAction(
      { ...lesson, attendance_count: 24 },
      new Date("2026-10-29T09:55"),
    ).kind,
    "note",
  );
  assert.equal(
    lessonAction({ ...lesson, ends_at: "10:00" }, new Date("2026-10-29T09:56"))
      .kind,
    "performance",
  );
});
test("dashboard skips finished and cancelled lessons and finds the next day", () => {
  const next = { ...lesson, id: "tomorrow", lesson_date: "2026-10-30" };
  const list = [
    next,
    lesson,
    {
      ...lesson,
      id: "cancelled",
      starts_at: "11:00",
      ends_at: "11:45",
      status: "cancelled",
    },
  ];
  assert.equal(currentOrNext(list, new Date("2026-10-29T09:55")).id, "one");
  assert.equal(
    currentOrNext(list, new Date("2026-10-29T10:35")).id,
    "tomorrow",
  );
  assert.equal(currentOrNext(list, new Date("2026-10-31T10:35")), null);
});
test("task completion distinguishes one-off and per-pupil work", () => {
  assert.equal(taskDone({ completion_mode: "once", completed_at: 123 }), true);
  assert.equal(
    taskDone({
      completion_mode: "per_pupil",
      assigned_count: 24,
      completed_count: 23,
    }),
    false,
  );
  assert.equal(
    taskDone({
      completion_mode: "per_pupil",
      assigned_count: 24,
      completed_count: 24,
    }),
    true,
  );
  assert.equal(
    taskDone({
      completion_mode: "per_pupil",
      assigned_count: 0,
      completed_count: 0,
    }),
    false,
  );
});
test("calendar day arithmetic follows local days across daylight saving changes", () => {
  assert.equal(monday("2026-10-25"), "2026-10-19");
  assert.equal(addDays("2026-10-25", 1), "2026-10-26");
});
