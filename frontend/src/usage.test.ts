import { test } from "node:test";
import assert from "node:assert/strict";
import {
  autoBucket,
  isoWeek,
  metricValue,
  niceMax,
  periodLabel,
  presetRange,
  usagePath,
} from "./usage.ts";

test("presets choose a range and a readable bucket", () => {
  const today = "2026-11-11";
  assert.deepEqual(presetRange("7d", today, "2026-01-01"), {
    from: "2026-11-05",
    until: today,
    bucket: "day",
  });
  assert.equal(presetRange("30d", today, "2026-01-01").from, "2026-10-13");
  // Twelve whole weeks including the current one, starting on a Monday.
  assert.deepEqual(presetRange("12w", today, "2026-01-01"), {
    from: "2026-08-24",
    until: today,
    bucket: "week",
  });
  assert.deepEqual(presetRange("12m", today, "2026-01-01"), {
    from: "2025-12-01",
    until: today,
    bucket: "month",
  });
  assert.deepEqual(presetRange("12m", "2026-01-31", "2026-01-01").from, "2025-02-01");
  assert.deepEqual(presetRange("all", today, today), {
    from: today,
    until: today,
    bucket: "day",
  });
});

test("automatic buckets grow with the range", () => {
  assert.equal(autoBucket("2026-11-01", "2026-12-01"), "day");
  assert.equal(autoBucket("2026-11-01", "2026-12-02"), "week");
  assert.equal(autoBucket("2026-01-01", "2026-07-01"), "week");
  assert.equal(autoBucket("2026-01-01", "2026-07-02"), "month");
});

test("active teachers average per day; other metrics are sums", () => {
  const counts = { teacher_active: 10, note_created: 7 };
  assert.equal(metricValue("note_created", counts, 7), 7);
  assert.equal(metricValue("teacher_active", counts, 7), 1.4);
  assert.equal(metricValue("teacher_active", counts, 0), 0);
  assert.equal(metricValue("pdf_exported", counts, 7), 0);
});

test("labels and axis", () => {
  assert.equal(isoWeek("2026-11-09"), 46);
  assert.equal(isoWeek("2027-01-01"), 53);
  assert.equal(isoWeek("2025-12-29"), 1);
  assert.equal(periodLabel("2026-11-09", "week"), "KW 46");
  assert.equal(periodLabel("2026-11-09", "day"), "9.11.");
  assert.equal(niceMax(0), 1);
  assert.equal(niceMax(3), 5);
  assert.equal(niceMax(17), 20);
  assert.equal(niceMax(100), 100);
  assert.equal(
    usagePath({ from: "2026-11-01", until: "2026-11-11", bucket: "week" }),
    "/admin/usage?bucket=week&from=2026-11-01&until=2026-11-11",
  );
});
