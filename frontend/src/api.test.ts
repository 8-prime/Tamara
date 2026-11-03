import test from "node:test";
import assert from "node:assert/strict";
import { z } from "zod";
import { api, ApiError, errorMessage, errorStatus } from "./api.ts";
import { HomeworkDetailSchema, LessonSchema, ConferenceSchema } from "./models.ts";

test("API validates JSON and rejects malformed or empty typed responses", async (t) => {
  const fetchMock = t.mock.method(globalThis, "fetch");
  fetchMock.mock.mockImplementation(async () => Response.json({ id: "created" }));
  assert.deepEqual(await api(z.object({id: z.string()}), "/example"), { id: "created" });
  fetchMock.mock.mockImplementation(async () => Response.json({ id: 123 }));
  await assert.rejects(api(z.object({id: z.string()}), "/example"), z.ZodError);
  fetchMock.mock.mockImplementation(async () => new Response(null, {status: 204}));
  assert.equal(await api("/example", {method: "DELETE"}), null);
  await assert.rejects(api(z.object({id: z.string()}), "/example"), z.ZodError);
});

test("API preserves upload headers, Headers objects, and authentication", async (t) => {
  t.mock.method(globalThis, "fetch", async (_url: string | URL | Request, init?: RequestInit) => {
    const headers = new Headers(init?.headers);
    assert.equal(headers.get("Content-Type"), "application/pdf");
    assert.equal(headers.get("X-Tamara-Request"), "1");
    assert.equal(init?.credentials, "same-origin");
    return new Response(null, {status: 204});
  });
  await api("/worksheet", {method: "PUT", headers: new Headers({"Content-Type": "application/pdf"}), body: new Blob(["pdf"])});
});

test("API errors preserve status and handle non-JSON failures", async (t) => {
  const fetchMock = t.mock.method(globalThis, "fetch", async () => Response.json({error: "Session expired"}, {status: 401}));
  await assert.rejects(api("/example"), (error: unknown) => {
    assert.ok(error instanceof ApiError);
    assert.equal(errorStatus(error), 401);
    assert.equal(errorMessage(error), "Session expired");
    return true;
  });
  fetchMock.mock.mockImplementation(async () => new Response("Unavailable", {status: 503}));
  await assert.rejects(api("/example"), (error: unknown) => errorStatus(error) === 503);
  assert.equal(errorStatus(new Error("offline")), undefined);
});

test("domain schemas reject incomplete records and preserve nullable conference fields", () => {
  assert.equal(LessonSchema.safeParse({id: "lesson"}).success, false);
  assert.equal(HomeworkDetailSchema.safeParse({homework: {}, submissions: []}).success, false);
  assert.deepEqual(ConferenceSchema.parse({id: "note", classroom_id: null, date: "2026-11-03", title: null, content: ""}),
    {id: "note", classroom_id: null, date: "2026-11-03", title: null, content: ""});
});
