export type Session =
  | { mode: "off" }
  | { mode: "for" | "until"; startedAt: number; endsAt: number }
  | { mode: "indefinite"; startedAt: number };

/** What `wick status` prints. Older Wick versions leave out the newer fields. */
export type Status = {
  running: boolean;
  awake: boolean;
  session?: Session;
  heldBy?: string | null;
  pausedBy?: string | null;
  pausedWhileLocked?: boolean;
  keepDisplayOn?: boolean;
  awakeTodayMinutes?: number;
};

export function parseStatus(text: string): Status | null {
  try {
    const value: unknown = JSON.parse(text);
    if (typeof value !== "object" || value === null) return null;
    const status = value as Record<string, unknown>;
    if (typeof status.running !== "boolean" || typeof status.awake !== "boolean") return null;
    return status as unknown as Status;
  } catch {
    return null;
  }
}

/** When the running session began, so a new start can be told apart from the old one. */
export function startedAt(status: Status | null): number | null {
  const session = status?.running ? status.session : undefined;
  return session && session.mode !== "off" ? session.startedAt : null;
}

/** "42m", "1h 05m", "2d 3h", like the app. */
export function shortDuration(milliseconds: number): string {
  const minutes = Math.ceil(Math.max(0, milliseconds) / 60_000);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ${String(minutes % 60).padStart(2, "0")}m`;
  return `${Math.floor(hours / 24)}d ${hours % 24}h`;
}

function clock(time: number): string {
  return new Date(time).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

/** The same title and detail the Wick popover shows. */
export function describe(status: Status, now = Date.now()): { title: string; detail: string } {
  if (!status.running) return { title: "Wick isn’t running", detail: "Starting a session opens it." };
  const session = status.session ?? { mode: "off" };
  if (status.pausedWhileLocked) return { title: "Paused", detail: "Paused while the screen is locked" };
  const display = status.keepDisplayOn ? " · Display on" : "";
  switch (session.mode) {
    case "for":
      return {
        title: `Awake · ${shortDuration(session.endsAt - now)} left`,
        detail: `Until ${clock(session.endsAt)}${display}`,
      };
    case "until":
      return {
        title: `Awake until ${clock(session.endsAt)}`,
        detail: `${shortDuration(session.endsAt - now)} left${display}`,
      };
    case "indefinite":
      return { title: "Awake", detail: `Until you stop it${display}` };
  }
  if (status.heldBy) return { title: "Awake", detail: `While ${status.heldBy}${display}` };
  if (status.pausedBy) return { title: "Off", detail: `Paused until ${status.pausedBy}` };
  return { title: "Off", detail: "Your Mac sleeps as usual." };
}

export type Start = { url: string; label: string };

/** "" → until stopped, "45" → 45 minutes, "16:30" → until 16:30. `null` for anything else. */
export function parseStart(input: string): Start | null {
  const value = input.trim();
  if (!value) return { url: "wick://start", label: "until you stop it" };
  if (/^\d{1,5}$/.test(value)) {
    const minutes = Number(value);
    if (minutes < 1 || minutes > 7 * 24 * 60) return null;
    return { url: `wick://start?minutes=${minutes}`, label: `for ${shortDuration(minutes * 60_000)}` };
  }
  const match = /^(\d{1,2}):(\d{2})$/.exec(value);
  if (!match) return null;
  const [hour, minute] = [Number(match[1]), Number(match[2])];
  if (hour > 23 || minute > 59) return null;
  const time = `${String(hour).padStart(2, "0")}:${match[2]}`;
  return { url: `wick://start?until=${time}`, label: `until ${time}` };
}
