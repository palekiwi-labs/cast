import { test } from "node:test";
import assert from "node:assert/strict";
import { Refresher } from "../src/refresh.js";

test("a refresh requested while one is running shares it", async () => {
  let calls = 0;
  let release = (): void => {};
  const refresher = new Refresher(
    () =>
      new Promise<void>((resolve) => {
        calls++;
        release = resolve;
      }),
  );
  const first = refresher.run();
  const second = refresher.run();
  assert.equal(calls, 1);
  release();
  await Promise.all([first, second]);
  const third = refresher.run();
  assert.equal(calls, 2);
  release();
  await third;
});

test("polls on an interval until stopped", async (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  let calls = 0;
  const refresher = new Refresher(async () => {
    calls++;
  });
  refresher.start(2000);
  t.mock.timers.tick(2000);
  await refresher.run();
  t.mock.timers.tick(2000);
  await refresher.run();
  refresher.stop();
  t.mock.timers.tick(2000);
  assert.equal(calls, 2);
});
