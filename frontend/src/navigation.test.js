import { test } from "node:test";
import assert from "node:assert/strict";
import { readRoute, workspaceUrl } from "./navigation.js";

test("pupil detail URLs restore class and pupil on refresh and history navigation", () => {
  const url = workspaceUrl("class-1", "students", "open", "", "", "", "pupil-2");
  assert.equal(url, "/classes/class-1/students?pupil=pupil-2");
  const location = new URL(url, "https://tamara.test");
  globalThis.window = { location };
  try {
    assert.equal(readRoute().pupil, "pupil-2");
    assert.equal(readRoute().classId, "class-1");
    assert.equal(readRoute().feature, "students");
    window.location = new URL(workspaceUrl("class-1", "students"), location);
    assert.equal(readRoute().pupil, "");
    assert.equal(workspaceUrl("class-1", "notes", "open", "", "", "", "pupil-2"), "/classes/class-1/notes");
  } finally { delete globalThis.window; }
});
