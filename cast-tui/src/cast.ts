import { execFile } from "node:child_process";
import { type Instance, parseListing } from "./instances.js";

/** A cast CLI invocation failed; the message carries cast's stderr. */
export class CastError extends Error {
  override name = "CastError";
}

export interface CastOptions {
  /** Environment for cast; `CAST_BIN` overrides the binary. Default: process.env. */
  env?: NodeJS.ProcessEnv;
}

function runCast(args: string[], options: CastOptions): Promise<string> {
  const env = options.env ?? process.env;
  const bin = env.CAST_BIN || "cast";
  return new Promise((resolve, reject) => {
    execFile(bin, args, { env }, (error, stdout, stderr) => {
      if (!error) return resolve(stdout);
      const command = `cast ${args.join(" ")}`;
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        return reject(new CastError(`${command}: ${bin} not found`));
      }
      const detail = stderr.trim() || error.message;
      reject(new CastError(`${command} failed: ${detail}`));
    });
  });
}

/** Lists cast instances across all workspaces via `cast ps`. */
export async function listInstances(options: CastOptions = {}): Promise<Instance[]> {
  return parseListing(await runCast(["ps"], options));
}

/** Stops an instance via `cast down` in its workspace. */
export async function stopInstance(
  instance: Instance,
  options: CastOptions = {},
): Promise<void> {
  if (instance.workspace === null) {
    throw new CastError(`${instance.container} has no workspace label; cannot stop it`);
  }
  const args = ["-C", instance.workspace, "down"];
  if (instance.name !== null) args.push("--name", instance.name);
  await runCast(args, options);
}
