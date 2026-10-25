import { type Component, truncateToWidth } from "@earendil-works/pi-tui";
import type { Instance } from "./instances.js";

/** Formats seconds as a short uptime such as "3d 4h", "1h 2m" or "45s". */
export function formatUptime(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}

function pad(text: string, width: number): string {
  return text.length >= width ? text : text + " ".repeat(width - text.length);
}

/** The instance list screen: header, selectable rows and a key hint footer. */
export class InstanceListView implements Component {
  private instances: Instance[] = [];
  private selectedId: string | undefined;
  private error: string | undefined;
  private loaded = false;

  constructor(private readonly now: () => number = () => Date.now() / 1000) {}

  setInstances(instances: Instance[]): void {
    const index = this.selectedIndex();
    this.instances = instances;
    this.error = undefined;
    this.loaded = true;
    if (!instances.some((i) => i.id === this.selectedId)) {
      const clamped = Math.min(Math.max(index, 0), instances.length - 1);
      this.selectedId = instances[clamped]?.id;
    }
  }

  setError(message: string): void {
    this.error = message;
    this.loaded = true;
  }

  selected(): Instance | undefined {
    return this.instances.find((i) => i.id === this.selectedId);
  }

  moveUp(): void {
    this.moveBy(-1);
  }

  moveDown(): void {
    this.moveBy(1);
  }

  private selectedIndex(): number {
    return this.instances.findIndex((i) => i.id === this.selectedId);
  }

  private moveBy(delta: number): void {
    if (this.instances.length === 0) return;
    const next = Math.min(
      Math.max(this.selectedIndex() + delta, 0),
      this.instances.length - 1,
    );
    this.selectedId = this.instances[next]?.id;
  }

  invalidate(): void {}

  render(width: number): string[] {
    const out = ["cast instances", ""];
    if (!this.loaded) {
      out.push("  loading...");
    } else if (this.instances.length === 0) {
      out.push("  no running instances");
    } else {
      const workspaceWidth = Math.max(
        ...this.instances.map((i) => (i.workspace ?? i.container).length),
      );
      const nameWidth = Math.max(7, ...this.instances.map((i) => (i.name ?? "").length));
      for (const i of this.instances) {
        const marker = i.id === this.selectedId ? ">" : " ";
        const uptime = i.started === null ? "-" : formatUptime(this.now() - i.started);
        out.push(
          [
            marker,
            pad(i.workspace ?? i.container, workspaceWidth),
            pad(i.name ?? "default", nameWidth),
            pad(i.state, 8),
            uptime,
          ].join("  "),
        );
      }
    }
    if (this.error !== undefined) out.push("", `error: ${this.error}`);
    out.push("", "up/down select  r refresh  q quit");
    return out.map((line) => truncateToWidth(line, width));
  }
}
