import { test } from "node:test";
import assert from "node:assert/strict";
import { stripTerminalSequences } from "@earendil-works/pi-tui";
import { InstanceListView } from "../src/view.js";
import type { Instance } from "../src/instances.js";

const NOW = 1_000_000;

function instance(id: string, overrides: Partial<Instance> = {}): Instance {
  return {
    id,
    container: `cast-dev-${id}`,
    name: null,
    workspace: `/home/u/${id}`,
    workspaceId: id,
    repo: `/home/u/${id}/.git`,
    castVersion: "0.3.0",
    state: "running",
    created: NOW - 7200,
    started: NOW - 3720,
    ...overrides,
  };
}

function lines(view: InstanceListView): string[] {
  return view.render(100).map((l) => stripTerminalSequences(l));
}

function selectedLine(view: InstanceListView): string | undefined {
  return lines(view).find((l) => l.startsWith(">"));
}

test("lists instances with workspace, name, state and uptime", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([instance("a", { name: "review" }), instance("b")]);
  const out = lines(view).join("\n");
  assert.match(out, /~\/a\s+review\s+a\s+running\s+1h 2m/);
  assert.match(out, /~\/b\s+default\s+b\s+running/);
});

test("selects the first instance and moves with up/down, clamped", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([instance("a"), instance("b")]);
  assert.match(selectedLine(view) ?? "", /~\/a/);
  view.moveDown();
  view.moveDown();
  assert.match(selectedLine(view) ?? "", /~\/b/);
  view.moveUp();
  view.moveUp();
  assert.match(selectedLine(view) ?? "", /~\/a/);
});

test("keeps the selected instance across a refresh", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([instance("a"), instance("b")]);
  view.moveDown();
  view.setInstances([instance("c"), instance("a"), instance("b")]);
  assert.match(selectedLine(view) ?? "", /~\/b/);
  assert.equal(view.selected()?.id, "b");
});

test("shows an empty state", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([]);
  assert.match(lines(view).join("\n"), /no running instances/i);
  assert.equal(view.selected(), undefined);
});

test("shows an error without dropping the last listing", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([instance("a")]);
  view.setError("cast ps failed: docker is down");
  const out = lines(view).join("\n");
  assert.match(out, /docker is down/);
  assert.match(out, /~\/a/);
  view.setInstances([instance("a")]);
  assert.doesNotMatch(lines(view).join("\n"), /docker is down/);
});

test("shows the repo directory and abbreviates home", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([
    instance("wt", { workspace: "/home/u/repo/worktrees/feat", repo: "/home/u/repo/.git" }),
    instance("x", { workspace: "/srv/x", repo: null }),
  ]);
  const out = lines(view).join("\n");
  assert.match(out, /~\/repo\/worktrees\/feat\s+default\s+repo\s+running/);
  assert.match(out, /\/srv\/x\s+default\s+-\s+running/);
});

test("marks stopped containers without an uptime", () => {
  const view = new InstanceListView({ now: () => NOW, home: "/home/u" });
  view.setInstances([instance("a", { state: "created", started: null })]);
  assert.match(lines(view).join("\n"), /created\s+-/);
});
