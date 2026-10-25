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

export const LISTING_VERSION = 1;

/** `cast ps` output that cast-tui cannot use. */
export class ListingError extends Error {
  override name = "ListingError";
}

type Raw = Record<string, unknown>;

function isObject(value: unknown): value is Raw {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function str(raw: Raw, key: string): string {
  const value = raw[key];
  if (typeof value !== "string") {
    throw new ListingError(`instance field ${key} is not a string`);
  }
  return value;
}

function optStr(raw: Raw, key: string): string | null {
  const value = raw[key] ?? null;
  if (value !== null && typeof value !== "string") {
    throw new ListingError(`instance field ${key} is not a string or null`);
  }
  return value;
}

function num(raw: Raw, key: string): number {
  const value = raw[key];
  if (typeof value !== "number") {
    throw new ListingError(`instance field ${key} is not a number`);
  }
  return value;
}

function optNum(raw: Raw, key: string): number | null {
  const value = raw[key] ?? null;
  if (value !== null && typeof value !== "number") {
    throw new ListingError(`instance field ${key} is not a number or null`);
  }
  return value;
}

function parseInstance(raw: unknown): Instance {
  if (!isObject(raw)) throw new ListingError("instance is not an object");
  return {
    id: str(raw, "id"),
    container: str(raw, "container"),
    name: optStr(raw, "name"),
    workspace: optStr(raw, "workspace"),
    workspaceId: optStr(raw, "workspace_id"),
    repo: optStr(raw, "repo"),
    castVersion: optStr(raw, "cast_version"),
    state: str(raw, "state"),
    created: num(raw, "created"),
    started: optNum(raw, "started"),
  };
}

/** Parses `cast ps` output, throwing ListingError on anything but schema v1. */
export function parseListing(text: string): Instance[] {
  let listing: unknown;
  try {
    listing = JSON.parse(text);
  } catch {
    throw new ListingError("cast ps output is not JSON");
  }
  if (!isObject(listing)) throw new ListingError("cast ps output is not an object");
  if (listing.version !== LISTING_VERSION) {
    throw new ListingError(
      `unsupported cast ps schema version ${String(listing.version)} ` +
        `(expected ${LISTING_VERSION})`,
    );
  }
  if (!Array.isArray(listing.instances)) {
    throw new ListingError("cast ps output has no instances list");
  }
  return listing.instances.map(parseInstance);
}
