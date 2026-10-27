import { Action, ActionPanel, Color, Icon, Keyboard, List, showToast, Toast } from "@raycast/api";
import { showFailureToast, usePromise } from "@raycast/utils";
import { useState } from "react";
import { describe, parseStart, shortDuration, startedAt, type Start, type Status } from "./status";
import { DOWNLOAD_URL, isOn, readStatus, send } from "./app";

const presets: Start[] = [
  { url: "wick://start?minutes=15", label: "for 15 minutes" },
  { url: "wick://start?minutes=60", label: "for 1 hour" },
  { url: "wick://start?minutes=120", label: "for 2 hours" },
  { url: "wick://start", label: "until you stop it" },
];

export default function Command() {
  const [search, setSearch] = useState("");
  const {
    data: status,
    isLoading,
    revalidate,
    mutate,
  } = usePromise(() => readStatus(), [], {
    onError: (error) => {
      showFailureToast(error, { title: "Couldn’t read Wick’s status" });
    },
  });

  async function perform(url: string, done: (next: Status) => boolean, success: string) {
    const toast = await showToast({ style: Toast.Style.Animated, title: "Talking to Wick…" });
    try {
      await mutate(send(url, done), { shouldRevalidateAfter: false });
      toast.style = Toast.Style.Success;
      toast.title = success;
    } catch (error) {
      toast.style = Toast.Style.Failure;
      toast.title = "Wick didn’t respond";
      toast.message = error instanceof Error ? error.message : String(error);
    }
  }

  function start(choice: Start) {
    const before = startedAt(status ?? null);
    return perform(
      choice.url,
      (next) => startedAt(next) !== null && startedAt(next) !== before,
      `Keeping your Mac awake ${choice.label}`,
    );
  }

  if (status === null) {
    return (
      <List>
        <List.EmptyView
          icon={Icon.Download}
          title="Wick isn’t installed"
          description="Install Wick, open it once, then come back."
          actions={
            <ActionPanel>
              <Action.OpenInBrowser title="Get Wick" url={DOWNLOAD_URL} />
              <Action.CopyToClipboard title="Copy Homebrew Command" content="brew install --cask abulkosim/tap/wick" />
            </ActionPanel>
          }
        />
      </List>
    );
  }

  const now = describe(status ?? { running: false, awake: false });
  const on = status ? status.running && isOn(status) : false;
  const typed = search.trim() ? parseStart(search) : null;
  const today = status?.awakeTodayMinutes;

  return (
    <List
      isLoading={isLoading}
      searchBarPlaceholder="Minutes or a time, like 45 or 16:30"
      onSearchTextChange={setSearch}
      filtering={false}
    >
      <List.Section title="Now">
        <List.Item
          icon={{ source: on ? Icon.CircleFilled : Icon.Circle, tintColor: on ? Color.Green : Color.SecondaryText }}
          title={now.title}
          subtitle={now.detail}
          accessories={today !== undefined && today > 0 ? [{ text: `Today ${shortDuration(today * 60_000)}` }] : []}
          actions={
            <ActionPanel>
              {on ? (
                <Action
                  title="Stop"
                  icon={Icon.Stop}
                  onAction={() => perform("wick://stop", (next) => !isOn(next), "Your Mac can sleep again")}
                />
              ) : (
                <Action title="Keep Awake Until Stopped" icon={Icon.Play} onAction={() => start(presets[3])} />
              )}
              <Action
                title="Refresh"
                icon={Icon.ArrowClockwise}
                shortcut={Keyboard.Shortcut.Common.Refresh}
                onAction={revalidate}
              />
            </ActionPanel>
          }
        />
      </List.Section>
      <List.Section title="Keep awake">
        {search.trim() && !typed ? (
          <List.Item icon={Icon.ExclamationMark} title="Type minutes like 45 or a time like 16:30" />
        ) : null}
        {typed ? (
          <List.Item
            icon={Icon.Clock}
            title={`Keep awake ${typed.label}`}
            actions={
              <ActionPanel>
                <Action title="Start" icon={Icon.Play} onAction={() => start(typed)} />
              </ActionPanel>
            }
          />
        ) : null}
        {presets.map((preset) => (
          <List.Item
            key={preset.url}
            icon={preset.url === "wick://start" ? Icon.Sun : Icon.Hourglass}
            title={`Keep awake ${preset.label}`}
            actions={
              <ActionPanel>
                <Action title="Start" icon={Icon.Play} onAction={() => start(preset)} />
              </ActionPanel>
            }
          />
        ))}
      </List.Section>
    </List>
  );
}
