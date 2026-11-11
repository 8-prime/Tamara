import { test } from "node:test";
import assert from "node:assert/strict";
import { filterAccounts, initials } from "./accounts.ts";

const accounts = [
  { id: "1", name: "Zoe Weber", email: "zoe@schule.test", is_admin: false, created_at: 30 },
  { id: "2", name: "Änne Albers", email: "aenne@schule.test", is_admin: true, created_at: 10 },
  { id: "3", name: "Bernd Becker", email: "Leitung@Schule.test", is_admin: false, created_at: 20 },
];

test("account lists sort by German name order or newest registration", () => {
  assert.deepEqual(filterAccounts(accounts, "").map((a) => a.id), ["2", "3", "1"]);
  assert.deepEqual(filterAccounts(accounts, "", "all", "newest").map((a) => a.id), ["1", "3", "2"]);
  // Filtering never mutates the caller's array.
  assert.deepEqual(accounts.map((a) => a.id), ["1", "2", "3"]);
});

test("account search matches name or email case-insensitively and combines with role", () => {
  assert.deepEqual(filterAccounts(accounts, "  leitung ").map((a) => a.id), ["3"]);
  assert.deepEqual(filterAccounts(accounts, "änne").map((a) => a.id), ["2"]);
  assert.deepEqual(filterAccounts(accounts, "", "admin").map((a) => a.id), ["2"]);
  assert.deepEqual(filterAccounts(accounts, "schule", "teacher").map((a) => a.id), ["3", "1"]);
  assert.deepEqual(filterAccounts(accounts, "niemand"), []);
});

test("initials use the first letters of up to two name parts", () => {
  assert.equal(initials("Stefan Hoffmeister"), "SH");
  assert.equal(initials("anna maria von berg"), "AM");
  assert.equal(initials("Tamara"), "T");
});
