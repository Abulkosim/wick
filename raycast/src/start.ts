import { LaunchProps, showHUD } from "@raycast/api";
import { showFailureToast } from "@raycast/utils";
import { describe, parseStart, startedAt } from "./status";
import { readStatus, send } from "./app";

export default async function Command(props: LaunchProps<{ arguments: Arguments.Start }>) {
  const start = parseStart(props.arguments.duration ?? "");
  if (!start) {
    await showFailureToast("Use minutes like 45 or a time like 16:30.", { title: "Can’t read that duration" });
    return;
  }
  try {
    const before = startedAt(await readStatus());
    const status = await send(start.url, (next) => startedAt(next) !== null && startedAt(next) !== before);
    await showHUD(describe(status).title);
  } catch (error) {
    await showFailureToast(error, { title: "Wick didn’t start" });
  }
}
