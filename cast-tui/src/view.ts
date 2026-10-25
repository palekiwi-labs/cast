import { type Component, truncateToWidth } from "@earendil-works/pi-tui";
import type { Instance } from "./instances.js";
import { fg, type ThemeColor } from "./theme.js";

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

function stateColor(state: string): ThemeColor {
  if (state === "running") return "success";
  if (state === "exited" || state === "dead") return "error";
  return "warning";
}

/** The repository directory name for a git dir like /home/u/repo/.git. */
function repoName(repo: string | null): string {
  if (repo === null) return "-";
  const parts = repo.replace(/\/+$/, "").split("/");
  if (parts[parts.length - 1] === ".git") parts.pop();
  return parts[parts.length - 1] || repo;
}

export interface InstanceListViewOptions {
  /** Current time in unix seconds. */
  now?: () => number;
  /** Home directory, shown as ~ in workspace paths. */
  home?: string;
}

interface Row {
  instance: Instance;
  cells: string[];
}

/** The instance list screen: header, selectable rows and a key hint footer. */
export class InstanceListView implements Component {
  private instances: Instance[] = [];
  private selectedId: string | undefined;
  private error: string | undefined;
  private loaded = false;

  private readonly now: () => number;
  private readonly home: string | undefined;

  constructor(options: InstanceListViewOptions = {}) {
    this.now = options.now ?? (() => Date.now() / 1000);
    this.home = options.home;
  }

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

  private displayPath(path: string): string {
    const home = this.home;
    if (home && (path === home || path.startsWith(`${home}/`))) {
      return `~${path.slice(home.length)}`;
    }
    return path;
  }

  private rows(): Row[] {
    return this.instances.map((instance) => ({
      instance,
      cells: [
        instance.workspace === null
          ? instance.container
          : this.displayPath(instance.workspace),
        instance.name ?? "default",
        repoName(instance.repo),
        instance.state,
        instance.started === null || instance.state !== "running"
          ? "-"
          : formatUptime(this.now() - instance.started),
      ],
    }));
  }

  render(width: number): string[] {
    const count = this.loaded ? ` (${this.instances.length})` : "";
    const out = [fg("accent", "cast", { bold: true }) + fg("dim", ` instances${count}`), ""];
    if (!this.loaded) {
      out.push(fg("muted", "  loading..."));
    } else if (this.instances.length === 0) {
      out.push(fg("muted", "  no running instances"));
    } else {
      const header = ["WORKSPACE", "NAME", "REPO", "STATE", "UPTIME"];
      const rows = this.rows();
      const widths = header.map((h, col) =>
        Math.max(h.length, ...rows.map((r) => r.cells[col]?.length ?? 0)),
      );
      const line = (cells: string[]) =>
        cells.map((c, col) => pad(c, widths[col] ?? 0)).join("  ").trimEnd();
      out.push(fg("dim", `  ${line(header)}`));
      for (const { instance, cells } of rows) {
        const selected = instance.id === this.selectedId;
        const [workspace = "", name = "", repo = "", state = "", uptime = ""] = cells.map(
          (c, col) => pad(c, widths[col] ?? 0),
        );
        const body = selected
          ? fg("accent", `${workspace}  ${name}  ${repo}  `)
          : `${workspace}  ${fg("muted", name)}  ${fg("muted", repo)}  `;
        out.push(
          (selected ? fg("accent", "> ") : "  ") +
            body +
            fg(stateColor(instance.state), state) +
            "  " +
            fg("muted", uptime.trimEnd()),
        );
      }
    }
    if (this.error !== undefined) out.push("", fg("error", `error: ${this.error}`));
    out.push(
      "",
      fg("dim", "\u2191/\u2193 select  r refresh  q quit"),
    );
    return out.map((l) => truncateToWidth(l, width, fg("dim", "...")));
  }
}
