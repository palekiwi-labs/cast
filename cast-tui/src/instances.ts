/** One cast instance as reported by `cast ps` (schema version 1). */
export interface Instance {
  id: string;
  container: string;
  name: string | null;
  workspace: string | null;
  workspaceId: string | null;
  repo: string | null;
  castVersion: string | null;
  state: string;
  /** Unix seconds. */
  created: number;
  /** Unix seconds; null when the container never started. */
  started: number | null;
}

export function parseListing(text: string): Instance[] {
  const listing = JSON.parse(text);
  return listing.instances.map((raw: Record<string, unknown>) => ({
    id: raw.id,
    container: raw.container,
    name: raw.name,
    workspace: raw.workspace,
    workspaceId: raw.workspace_id,
    repo: raw.repo,
    castVersion: raw.cast_version,
    state: raw.state,
    created: raw.created,
    started: raw.started,
  }));
}
