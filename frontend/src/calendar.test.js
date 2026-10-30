import { test } from "node:test";
import assert from "node:assert/strict";
import {
  timeMinutes,
  timeString,
  snapMinute,
  moveRange,
  resizeRange,
  dayLayout,
} from "./calendar.js";

test("calendar maps times to a continuous quarter-hour axis", () => {
  assert.equal(timeMinutes("08:45"), 525);
  assert.equal(timeString(525), "08:45");
  assert.equal(snapMinute(532), 525);
  assert.equal(snapMinute(533), 540);
});
test("moving preserves lesson duration and stays inside the same calendar day", () => {
  assert.deepEqual(moveRange(480, 525, 558), { start: 555, end: 600 });
  assert.deepEqual(moveRange(480, 525, -80), { start: 0, end: 45 });
  assert.deepEqual(moveRange(480, 525, 1450), { start: 1394, end: 1439 });
});
test("resizing keeps the start fixed and prevents empty or overnight lessons", () => {
  assert.deepEqual(resizeRange(480, 550), { start: 480, end: 555 });
  assert.deepEqual(resizeRange(480, 450), { start: 480, end: 495 });
  assert.deepEqual(resizeRange(480, 1500), { start: 480, end: 1439 });
  assert.deepEqual(resizeRange(1430, 1500), { start: 1430, end: 1439 });
});
test("adjacent lessons use the same lane while overlaps remain independently visible", () => {
  const lesson = (id, start, end) => ({ id, starts_at: start, ends_at: end });
  const result = dayLayout([
    lesson("a", "08:00", "08:45"),
    lesson("b", "08:45", "09:30"),
    lesson("replacement", "08:15", "08:40"),
    lesson("later", "10:00", "10:45"),
  ]);
  assert.equal(result.find((i) => i.lesson.id === "a").lanes, 2);
  assert.equal(result.find((i) => i.lesson.id === "replacement").lane, 1);
  assert.equal(result.find((i) => i.lesson.id === "b").lanes, 1);
  assert.equal(result.find((i) => i.lesson.id === "later").lanes, 1);
  assert.deepEqual(dayLayout([]), []);
});
