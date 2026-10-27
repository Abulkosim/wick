import { showHUD } from "@raycast/api";
import { showFailureToast } from "@raycast/utils";
import { describe } from "./status";
import { isOn, readStatus, send } from "./app";

export default async function Command() {
  try {
    const before = await readStatus();
    if (before && (!before.running || !isOn(before))) {
      await showHUD("Wick is already off");
      return;
    }
    const status = await send("wick://stop", (next) => !isOn(next));
    await showHUD(describe(status).detail);
  } catch (error) {
    await showFailureToast(error, { title: "Wick didn’t stop" });
  }
}
