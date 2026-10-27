import assert from "node:assert/strict";
import test from "node:test";
import { describe, parseStart, parseStatus, shortDuration } from "./status.ts";

test("reads wick status output", () => {
  assert.deepEqual(parseStatus('{"running":false,"awake":false}'), { running: false, awake: false });
  assert.equal(parseStatus("not json"), null);
  assert.equal(parseStatus('{"awake":true}'), null);
  assert.equal(parseStatus("[]"), null);
});

test("describes each state like the popover", () => {
  const now = Date.UTC(2026, 9, 27, 12, 0);
  assert.equal(describe({ running: false, awake: false }).title, "Wick isn’t running");
  assert.deepEqual(describe({ running: true, awake: false, session: { mode: "off" } }), {
    title: "Off",
    detail: "Your Mac sleeps as usual.",
  });
  const timed = describe(
    { running: true, awake: true, session: { mode: "for", startedAt: now, endsAt: now + 42 * 60_000 } },
    now,
  );
  assert.equal(timed.title, "Awake · 42m left");
  assert.match(timed.detail, /^Until /);
  assert.deepEqual(
    describe({ running: true, awake: true, session: { mode: "indefinite", startedAt: now }, keepDisplayOn: true }, now),
    { title: "Awake", detail: "Until you stop it · Display on" },
  );
  assert.equal(
    describe({ running: true, awake: true, session: { mode: "off" }, heldBy: "Xcode is running" }).detail,
    "While Xcode is running",
  );
  assert.equal(
    describe({ running: true, awake: false, session: { mode: "off" }, pausedBy: "claude quits" }).detail,
    "Paused until claude quits",
  );
  assert.equal(
    describe({ running: true, awake: false, pausedWhileLocked: true }).detail,
    "Paused while the screen is locked",
  );
});

test("start input becomes a wick link", () => {
  assert.deepEqual(parseStart(""), { url: "wick://start", label: "until you stop it" });
  assert.deepEqual(parseStart(" 45 "), { url: "wick://start?minutes=45", label: "for 45m" });
  assert.deepEqual(parseStart("90"), { url: "wick://start?minutes=90", label: "for 1h 30m" });
  assert.deepEqual(parseStart("9:05"), { url: "wick://start?until=09:05", label: "until 09:05" });
  assert.equal(parseStart("0"), null);
  assert.equal(parseStart("10081"), null);
  assert.equal(parseStart("24:00"), null);
  assert.equal(parseStart("soon"), null);
});

test("durations read like the app", () => {
  assert.equal(shortDuration(42 * 60_000), "42m");
  assert.equal(shortDuration(65 * 60_000), "1h 05m");
  assert.equal(shortDuration(27 * 60 * 60_000), "1d 3h");
});
