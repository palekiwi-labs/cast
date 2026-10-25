import { test } from "node:test";
import assert from "node:assert/strict";
import { ListingError, parseListing } from "../src/instances.js";

const LISTING = JSON.stringify({
  version: 1,
  instances: [
    {
      id: "abc123",
      container: "cast-dev-1a2b3c-review",
      name: "review",
      workspace: "/home/u/repo",
      workspace_id: "1a2b3c",
      repo: "/home/u/repo/.git",
      cast_version: "0.3.0",
      state: "running",
      created: 1777629600,
      started: 1777629601,
    },
    {
      id: "def456",
      container: "cast-dev-9z",
      name: null,
      workspace: "/w",
      workspace_id: "9z",
      repo: "/w/.git",
      cast_version: "0.3.0",
      state: "created",
      created: 86400,
      started: null,
    },
  ],
});

test("parses a v1 listing", () => {
  const instances = parseListing(LISTING);
  assert.equal(instances.length, 2);
  assert.deepEqual(instances[0], {
    id: "abc123",
    container: "cast-dev-1a2b3c-review",
    name: "review",
    workspace: "/home/u/repo",
    workspaceId: "1a2b3c",
    repo: "/home/u/repo/.git",
    castVersion: "0.3.0",
    state: "running",
    created: 1777629600,
    started: 1777629601,
  });
  assert.equal(instances[1]?.name, null);
  assert.equal(instances[1]?.started, null);
});
