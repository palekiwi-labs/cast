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
  const view = new InstanceListView(() => NOW);
  view.setInstances([instance("a", { name: "review" }), instance("b")]);
  const out = lines(view).join("\n");
  assert.match(out, /\/home\/u\/a\s+review\s+running\s+1h 2m/);
  assert.match(out, /\/home\/u\/b\s+default\s+running/);
});

test("selects the first instance and moves with up/down, clamped", () => {
  const view = new InstanceListView(() => NOW);
  view.setInstances([instance("a"), instance("b")]);
  assert.match(selectedLine(view) ?? "", /\/home\/u\/a/);
  view.moveDown();
  view.moveDown();
  assert.match(selectedLine(view) ?? "", /\/home\/u\/b/);
  view.moveUp();
  view.moveUp();
  assert.match(selectedLine(view) ?? "", /\/home\/u\/a/);
});

test("keeps the selected instance across a refresh", () => {
  const view = new InstanceListView(() => NOW);
  view.setInstances([instance("a"), instance("b")]);
  view.moveDown();
  view.setInstances([instance("c"), instance("a"), instance("b")]);
  assert.match(selectedLine(view) ?? "", /\/home\/u\/b/);
  assert.equal(view.selected()?.id, "b");
});

test("shows an empty state", () => {
  const view = new InstanceListView(() => NOW);
  view.setInstances([]);
  assert.match(lines(view).join("\n"), /no running instances/i);
  assert.equal(view.selected(), undefined);
});

test("shows an error without dropping the last listing", () => {
  const view = new InstanceListView(() => NOW);
  view.setInstances([instance("a")]);
  view.setError("cast ps failed: docker is down");
  const out = lines(view).join("\n");
  assert.match(out, /docker is down/);
  assert.match(out, /\/home\/u\/a/);
  view.setInstances([instance("a")]);
  assert.doesNotMatch(lines(view).join("\n"), /docker is down/);
});
