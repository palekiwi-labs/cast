import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CastError, listInstances, stopInstance } from "../src/cast.js";
import type { Instance } from "../src/instances.js";

// A fake cast that records its arguments, then prints $FAKE_STDOUT and
// $FAKE_STDERR and exits with $FAKE_EXIT.
function fakeCast(): { bin: string; argsFile: string } {
  const dir = mkdtempSync(join(tmpdir(), "cast-tui-test-"));
  const bin = join(dir, "cast");
  const argsFile = join(dir, "args");
  writeFileSync(
    bin,
    `#!/bin/sh
printf '%s\\n' "$@" > '${argsFile}'
printf '%s' "$FAKE_STDOUT"
printf '%s' "$FAKE_STDERR" >&2
exit "\${FAKE_EXIT:-0}"
`,
  );
  chmodSync(bin, 0o755);
  return { bin, argsFile };
}

function recordedArgs(argsFile: string): string[] {
  return readFileSync(argsFile, "utf8").trimEnd().split("\n");
}

const INSTANCE: Instance = {
  id: "abc123",
  container: "cast-dev-1a2b3c-review",
  name: "review",
  workspace: "/home/u/repo",
  workspaceId: "1a2b3c",
  repo: "/home/u/repo/.git",
  castVersion: "0.3.0",
  state: "running",
  created: 1,
  started: 2,
};

test("listInstances runs cast ps and parses its output", async () => {
  const { bin, argsFile } = fakeCast();
  const env = { CAST_BIN: bin, FAKE_STDOUT: '{"version":1,"instances":[]}' };
  assert.deepEqual(await listInstances({ env }), []);
  assert.deepEqual(recordedArgs(argsFile), ["ps"]);
});

test("listInstances failure carries stderr", async () => {
  const { bin } = fakeCast();
  const env = { CAST_BIN: bin, FAKE_EXIT: "1", FAKE_STDERR: "docker is down" };
  await assert.rejects(
    listInstances({ env }),
    (e: Error) => e instanceof CastError && e.message.includes("docker is down"),
  );
});

test("listInstances reports a missing cast binary", async () => {
  const env = { CAST_BIN: join(tmpdir(), "cast-tui-no-such-cast") };
  await assert.rejects(
    listInstances({ env }),
    (e: Error) => e instanceof CastError && /not found/.test(e.message),
  );
});

test("stopInstance runs cast down in the workspace with the name", async () => {
  const { bin, argsFile } = fakeCast();
  await stopInstance(INSTANCE, { env: { CAST_BIN: bin } });
  assert.deepEqual(recordedArgs(argsFile), [
    "-C",
    "/home/u/repo",
    "down",
    "--name",
    "review",
  ]);
});

test("stopInstance omits --name for the default instance", async () => {
  const { bin, argsFile } = fakeCast();
  await stopInstance({ ...INSTANCE, name: null }, { env: { CAST_BIN: bin } });
  assert.deepEqual(recordedArgs(argsFile), ["-C", "/home/u/repo", "down"]);
});

test("stopInstance failure carries stderr", async () => {
  const { bin } = fakeCast();
  const env = { CAST_BIN: bin, FAKE_EXIT: "1", FAKE_STDERR: "no such instance" };
  await assert.rejects(
    stopInstance(INSTANCE, { env }),
    (e: Error) => e instanceof CastError && e.message.includes("no such instance"),
  );
});

test("stopInstance refuses an instance without a workspace", async () => {
  const { bin } = fakeCast();
  await assert.rejects(
    stopInstance({ ...INSTANCE, workspace: null }, { env: { CAST_BIN: bin } }),
    CastError,
  );
});
