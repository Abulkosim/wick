import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { parseStatus, type Status } from "./status";

const run = promisify(execFile);

export const DOWNLOAD_URL = "https://github.com/Abulkosim/wick#install";

/** The app's own binary answers `wick status` without opening a window. */
export function binary(): string | null {
  const candidates = [
    "/Applications/Wick.app/Contents/MacOS/wick",
    join(homedir(), "Applications/Wick.app/Contents/MacOS/wick"),
    "/opt/homebrew/bin/wick",
    "/usr/local/bin/wick",
  ];
  return candidates.find((path) => existsSync(path)) ?? null;
}

/** `null` when Wick isn't installed. */
export async function readStatus(): Promise<Status | null> {
  const path = binary();
  if (!path) return null;
  const { stdout } = await run(path, ["status"], { timeout: 5000 });
  const status = parseStatus(stdout);
  if (!status) throw new Error("Wick answered with something unexpected. Update Wick and try again.");
  return status;
}

/**
 * Sends a `wick://` link through Launch Services, which opens Wick in the background if it isn't
 * running, then waits for the status to show the result.
 */
export async function send(url: string, done: (status: Status) => boolean): Promise<Status> {
  if (!binary()) throw new Error("Wick isn’t installed.");
  await run("/usr/bin/open", ["-g", url], { timeout: 5000 });
  let status = await readStatus();
  for (let attempt = 0; attempt < 25 && status && !(status.running && done(status)); attempt++) {
    await new Promise((resolve) => setTimeout(resolve, 200));
    status = await readStatus();
  }
  if (!status?.running) throw new Error("Wick didn’t respond. Open it once from Applications.");
  return status;
}

export const isOn = (status: Status) => status.awake || Boolean(status.heldBy && status.session?.mode === "off");
