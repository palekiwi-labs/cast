import { test } from "node:test";
import assert from "node:assert/strict";
import { isQuitKey } from "../src/keys.js";

test("q and ctrl+c quit", () => {
  assert.equal(isQuitKey("q"), true);
  assert.equal(isQuitKey("\x03"), true);
});

test("other keys do not quit", () => {
  assert.equal(isQuitKey("r"), false);
  assert.equal(isQuitKey("\x1b[A"), false);
});
