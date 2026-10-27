import { showHUD } from "@raycast/api";
import { showFailureToast } from "@raycast/utils";
import { describe } from "./status";
import { isOn, readStatus, send } from "./app";

export default async function Command() {
  try {
    const before = await readStatus();
    const wasOn = Boolean(before?.running && isOn(before));
    const status = await send("wick://toggle", (next) => isOn(next) !== wasOn);
    const { title, detail } = describe(status);
    await showHUD(isOn(status) ? title : detail);
  } catch (error) {
    await showFailureToast(error, { title: "Wick didn’t respond" });
  }
}
